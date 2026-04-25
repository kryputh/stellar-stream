import {
  Keypair,
  rpc,
  Contract,
  nativeToScVal,
  scValToNative,
  Address,
  TimeoutInfinite,
  TransactionBuilder,
  Networks,
  Account,
} from "@stellar/stellar-sdk";
import { initDb, getDb } from "./db";
import { recordEventWithDb } from "./eventHistory";
import { streamHasEvent } from "./eventHistory";
import { triggerWebhook } from "./webhook";

export type StreamStatus = "scheduled" | "active" | "completed" | "canceled";

export interface StreamInput {
  sender: string;
  recipient: string;
  assetCode: string;
  totalAmount: number;
  durationSeconds: number;
  startAt?: number;
}

export interface StreamRecord {
  id: string;
  sender: string;
  recipient: string;
  assetCode: string;
  totalAmount: number;
  durationSeconds: number;
  startAt: number;
  createdAt: number;
  canceledAt?: number;
  completedAt?: number;
  refundedAmount?: number;
}

export interface StreamProgress {
  status: StreamStatus;
  ratePerSecond: number;
  elapsedSeconds: number;
  vestedAmount: number;
  remainingAmount: number;
  percentComplete: number;
}

interface StreamRow {
  id: string;
  sender: string;
  recipient: string;
  asset_code: string;
  total_amount: number;
  duration_seconds: number;
  start_at: number;
  created_at: number;
  canceled_at: number | null;
  completed_at: number | null;
  refunded_amount: number | null;
  archived_at: number | null;
}

function rowToRecord(row: StreamRow): StreamRecord {
  return {
    id: row.id,
    sender: row.sender,
    recipient: row.recipient,
    assetCode: row.asset_code,
    totalAmount: row.total_amount,
    durationSeconds: row.duration_seconds,
    startAt: row.start_at,
    createdAt: row.created_at,
    canceledAt: row.canceled_at ?? undefined,
    completedAt: row.completed_at ?? undefined,
    refundedAmount: row.refunded_amount ?? undefined,
  };
}

function upsertStream(record: StreamRecord): void {
  const db = getDb();
  db.prepare(
    `
    INSERT INTO streams (id, sender, recipient, asset_code, total_amount, duration_seconds, start_at, created_at, canceled_at, completed_at, refunded_amount, archived_at)
    VALUES (@id, @sender, @recipient, @assetCode, @totalAmount, @durationSeconds, @startAt, @createdAt, @canceledAt, @completedAt, @refundedAmount, @archivedAt)
    ON CONFLICT(id) DO UPDATE SET
      sender = excluded.sender,
      recipient = excluded.recipient,
      asset_code = excluded.asset_code,
      total_amount = excluded.total_amount,
      duration_seconds = excluded.duration_seconds,
      start_at = excluded.start_at,
      created_at = excluded.created_at,
      canceled_at = excluded.canceled_at,
      completed_at = excluded.completed_at,
      refunded_amount = excluded.refunded_amount,
      archived_at = excluded.archived_at
  `,
  ).run({
    id: record.id,
    sender: record.sender,
    recipient: record.recipient,
    assetCode: record.assetCode,
    totalAmount: record.totalAmount,
    durationSeconds: record.durationSeconds,
    startAt: record.startAt,
    createdAt: record.createdAt,
    canceledAt: record.canceledAt ?? null,
    completedAt: record.completedAt ?? null,
    refundedAmount: record.refundedAmount ?? null,
    archivedAt: null,
  });
}

function listLocalStreamIds(): Set<string> {
  const db = getDb();
  const rows = db.prepare("SELECT id FROM streams").all() as Array<{ id: string }>;
  return new Set(rows.map((row) => row.id));
}

let rpcServer: rpc.Server | null = null;
let serverKeypair: Keypair | null = null;

export async function initSoroban() {
  initDb();

  const rpcUrl =
    process.env.RPC_URL || "https://soroban-testnet.stellar.org:443";
  rpcServer = new rpc.Server(rpcUrl);

  if (process.env.SERVER_PRIVATE_KEY) {
    serverKeypair = Keypair.fromSecret(process.env.SERVER_PRIVATE_KEY);
  } else {
    console.warn(
      "SERVER_PRIVATE_KEY missing. Creating streams on-chain will fail.",
    );
  }
}

function nowInSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

function round(value: number): number {
  return Number(value.toFixed(6));
}

interface CacheEntry<T> {
  data: T;
  expiresAt: number;
}

const rpcCache = new Map<string, CacheEntry<any>>();

function getCached<T>(key: string): T | null {
  const entry = rpcCache.get(key);
  if (!entry) return null;
  if (Date.now() > entry.expiresAt) {
    rpcCache.delete(key);
    return null;
  }
  return entry.data;
}

function setCached<T>(key: string, data: T, ttlSeconds = 5): void {
  rpcCache.set(key, {
    data,
    expiresAt: Date.now() + ttlSeconds * 1000,
  });
}

function invalidateCache(pattern?: string): void {
  if (!pattern) {
    rpcCache.clear();
  } else {
    for (const key of rpcCache.keys()) {
      if (key.includes(pattern)) {
        rpcCache.delete(key);
      }
    }
  }
}

async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxAttempts = 3,
): Promise<T> {
  let lastError: any;
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (err) {
      lastError = err;
      const message = String(err).toLowerCase();
      const isRetryable =
        message.includes("timeout") ||
        message.includes("network") ||
        message.includes("econnrefused") ||
        message.includes("econnreset");

      if (!isRetryable || attempt === maxAttempts) {
        throw err;
      }

      const delayMs = Math.pow(2, attempt - 1) * 1000;
      console.log(
        `[retry] attempt ${attempt} failed, retrying in ${delayMs}ms`,
        err,
      );
      await new Promise((r) => setTimeout(r, delayMs));
    }
  }
  throw lastError;
}

function getSorobanContext():
  | {
      contract: Contract;
      sourceAccountPromise: Promise<Account>;
    }
  | undefined {
  const contractId = process.env.CONTRACT_ID;
  if (!contractId || !rpcServer) {
    return undefined;
  }

  const pubKey = serverKeypair
    ? serverKeypair.publicKey()
    : "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

  return {
    contract: new Contract(contractId),
    sourceAccountPromise: rpcServer.getAccount(pubKey),
  };
}

async function simulateContractCall(
  contract: Contract,
  sourceAccount: Account,
  method: string,
  ...args: any[]
): Promise<rpc.Api.SimulateTransactionResponse> {
  if (!rpcServer) {
    throw new Error("Soroban RPC server is not initialized.");
  }

  const tx = new TransactionBuilder(sourceAccount, {
    fee: "100",
    networkPassphrase: process.env.NETWORK_PASSPHRASE || Networks.TESTNET,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  return rpcServer.simulateTransaction(tx);
}

async function fetchNextOnChainStreamId(
  contract: Contract,
  sourceAccount: Account,
): Promise<number | null> {
  const simRes = await simulateContractCall(
    contract,
    sourceAccount,
    "get_next_stream_id",
  );

  if (!rpc.Api.isSimulationSuccess(simRes) || !simRes.result) {
    console.warn("[reconciliation] failed to simulate get_next_stream_id", simRes);
    return null;
  }

  return Number(scValToNative(simRes.result.retval));
}

async function fetchOnChainStreamRecord(
  contract: Contract,
  sourceAccount: Account,
  id: number,
): Promise<StreamRecord | null> {
  const cacheKey = `stream:${id}`;
  const cached = getCached<StreamRecord>(cacheKey);
  if (cached) {
    return cached;
  }

  const simRes = await simulateContractCall(
    contract,
    sourceAccount,
    "get_stream",
    nativeToScVal(id, { type: "u64" }),
  );

  if (!rpc.Api.isSimulationSuccess(simRes) || !simRes.result) {
    return null;
  }

  const streamData = scValToNative(simRes.result.retval);

  const result = {
    id: id.toString(),
    sender: streamData.sender,
    recipient: streamData.recipient,
    assetCode: streamData.token,
    totalAmount: Number(streamData.total_amount),
    durationSeconds: Number(streamData.end_time) - Number(streamData.start_time),
    startAt: Number(streamData.start_time),
    createdAt: Number(streamData.start_time),
    canceledAt: streamData.canceled ? nowInSeconds() : undefined,
  };

  setCached(cacheKey, result, 5);
  return result;
}

function recordBackfilledCreatedEvent(stream: StreamRecord): void {
  if (streamHasEvent(stream.id, "created")) {
    return;
  }

  const db = getDb();
  db.transaction(() => {
    recordEventWithDb(
      db,
      stream.id,
      "created",
      stream.createdAt,
      stream.sender,
      stream.totalAmount,
      {
        recipient: stream.recipient,
        assetCode: stream.assetCode,
        durationSeconds: stream.durationSeconds,
        source: "reconciliation",
      },
    );
  })();
}

function computeStatus(stream: StreamRecord, at: number): StreamStatus {
  if (stream.canceledAt !== undefined) {
    return "canceled";
  }
  if (stream.completedAt !== undefined) {
    return "completed";
  }
  if (at < stream.startAt) {
    return "scheduled";
  }
  if (at >= stream.startAt + stream.durationSeconds) {
    return "completed";
  }
  return "active";
}

export function calculateProgress(
  stream: StreamRecord,
  at = nowInSeconds(),
): StreamProgress {
  const streamEnd = stream.startAt + stream.durationSeconds;
  const effectiveEnd =
    stream.canceledAt !== undefined
      ? Math.min(stream.canceledAt, streamEnd)
      : streamEnd;
  const elapsed = Math.max(0, Math.min(at, effectiveEnd) - stream.startAt);
  const ratio = Math.min(1, elapsed / stream.durationSeconds);
  const vestedAmount = stream.totalAmount * ratio;

  return {
    status: computeStatus(stream, at),
    ratePerSecond: round(stream.totalAmount / stream.durationSeconds),
    elapsedSeconds: elapsed,
    vestedAmount: round(vestedAmount),
    remainingAmount: round(Math.max(0, stream.totalAmount - vestedAmount)),
    percentComplete: round(ratio * 100),
  };
}

export async function syncStreams() {
  const sorobanContext = getSorobanContext();
  if (!sorobanContext) return;

  try {
    const sourceAccount = await sorobanContext.sourceAccountPromise;
    const nextId = await fetchNextOnChainStreamId(
      sorobanContext.contract,
      sourceAccount,
    );
    if (nextId === null) {
      return;
    }

    for (let i = 1; i < nextId; i++) {
      const stream = await fetchOnChainStreamRecord(
        sorobanContext.contract,
        sourceAccount,
        i,
      );
      if (stream) {
        upsertStream(stream);
      }
    }
  } catch (err) {
    console.error("Failed to sync streams", err);
  }
}

export async function reconcileMissingStreams(): Promise<number> {
  const sorobanContext = getSorobanContext();
  if (!sorobanContext) {
    return 0;
  }

  try {
    const sourceAccount = await sorobanContext.sourceAccountPromise;
    const nextId = await fetchNextOnChainStreamId(
      sorobanContext.contract,
      sourceAccount,
    );

    if (nextId === null || nextId <= 1) {
      console.log("[reconciliation] no on-chain streams available to reconcile");
      return 0;
    }

    const localStreamIds = listLocalStreamIds();
    const missingIds: number[] = [];

    for (let id = 1; id < nextId; id++) {
      if (!localStreamIds.has(id.toString())) {
        missingIds.push(id);
      }
    }

    if (missingIds.length === 0) {
      console.log("[reconciliation] no missing local streams detected");
      return 0;
    }

    console.warn(
      `[reconciliation] detected ${missingIds.length} missing local stream(s): ${missingIds.join(", ")}`,
    );

    let repairedCount = 0;
    for (const missingId of missingIds) {
      try {
        const stream = await fetchOnChainStreamRecord(
          sorobanContext.contract,
          sourceAccount,
          missingId,
        );

        if (!stream) {
          console.error(
            `[reconciliation] missing stream ${missingId} could not be fetched from chain`,
          );
          continue;
        }

        upsertStream(stream);
        recordBackfilledCreatedEvent(stream);
        repairedCount += 1;
      } catch (err) {
        console.error(
          `[reconciliation] failed to backfill missing stream ${missingId}:`,
          err,
        );
      }
    }

    console.log(
      `[reconciliation] repaired ${repairedCount} missing local stream(s) out of ${missingIds.length}`,
    );
    return repairedCount;
  } catch (err) {
    console.error("[reconciliation] reconciliation failed:", err);
    return 0;
  }
}

export async function createStream(input: StreamInput): Promise<StreamRecord> {
  const startAt = input.startAt ?? nowInSeconds();
  const contractId = process.env.CONTRACT_ID;
  const netPass =
    process.env.NETWORK_PASSPHRASE || "Test SDF Network ; September 2015";

  if (!contractId || !rpcServer || !serverKeypair) {
    throw new Error("Backend not configured for Soroban.");
  }

  const contract = new Contract(contractId);
  const endAt = startAt + input.durationSeconds;

  // Let's create an arbitrary testnet asset code for the token
  const fakeToken = contractId;

  const sourceAccount = await rpcServer.getAccount(serverKeypair.publicKey());

  const tx = new Contract(contractId).call(
    "create_stream",
    new Address(input.sender).toScVal(),
    new Address(input.recipient).toScVal(),
    new Address(fakeToken).toScVal(),
    nativeToScVal(input.totalAmount, { type: "i128" }),
    nativeToScVal(startAt, { type: "u64" }),
    nativeToScVal(endAt, { type: "u64" }),
  );

  // We have to build and send this tx. Wait, doing this properly via building is long:
  const built = await rpcServer.prepareTransaction(
    new TransactionBuilder(sourceAccount, {
      fee: "1000",
      networkPassphrase: netPass,
    })
      .addOperation(tx)
      .setTimeout(30)
      .build(),
  );

  built.sign(serverKeypair);

  const sendRes = await retryWithBackoff(() => rpcServer!.sendTransaction(built));
  if (sendRes.status !== "PENDING") {
    throw new Error("Failed to send transaction: " + JSON.stringify(sendRes));
  }

  let txResult;
  let attempts = 0;
  while (attempts < 10) {
    txResult = await retryWithBackoff(() => rpcServer!.getTransaction(sendRes.hash));
    if (txResult.status !== "NOT_FOUND") break;
    await new Promise((r) => setTimeout(r, 1000));
    attempts++;
  }

  if (txResult?.status !== "SUCCESS" || !txResult.returnValue) {
    throw new Error("Tx failed on chain: " + JSON.stringify(txResult));
  }

  const streamIdVal = scValToNative(txResult.returnValue);
  const streamIdStr = streamIdVal.toString();

  const stream: StreamRecord = {
    id: streamIdStr,
    sender: input.sender,
    recipient: input.recipient,
    assetCode: input.assetCode.toUpperCase(),
    totalAmount: input.totalAmount,
    durationSeconds: input.durationSeconds,
    startAt,
    createdAt: nowInSeconds(),
  };

  // Atomically write the stream row and the creation event.
  const db = getDb();
  db.transaction(() => {
    upsertStream(stream);
    recordEventWithDb(
      db,
      streamIdStr,
      "created",
      stream.createdAt,
      input.sender,
      input.totalAmount,
      {
        recipient: input.recipient,
        assetCode: input.assetCode,
        durationSeconds: input.durationSeconds,
      },
    );
  })();

  // Invalidate cache to ensure freshness after stream creation
  invalidateCache("stream:");

  // Webhook fires after the transaction commits — a webhook failure
  // must never roll back an already-persisted stream.
  triggerWebhook("created", stream);
  return stream;
}

export function refreshStreamStatuses(): number {
  const db = getDb();
  const now = nowInSeconds();

  
  const toComplete = db.prepare(`
    SELECT * FROM streams 
    WHERE canceled_at IS NULL AND completed_at IS NULL
      AND (start_at + duration_seconds) <= ?
  `).all() as StreamRow[];

  
  const result = db.prepare(`
    UPDATE streams SET completed_at = ?
    WHERE canceled_at IS NULL AND completed_at IS NULL
      AND (start_at + duration_seconds) <= ?
  `).run(now, now);

  
  toComplete.forEach(row => {
    const record = rowToRecord(row);
    
    record.completedAt = now; 
    triggerWebhook("completed", record);
  });

  return result.changes;
}

export async function archiveOldStreams(): Promise<number> {
  const db = getDb();
  const thirtyDaysAgo = nowInSeconds() - 30 * 24 * 60 * 60;

  try {
    // Find completed streams older than 30 days that haven't been archived yet
    const streamsToArchive = db
      .prepare(
        `
      SELECT * FROM streams
      WHERE completed_at IS NOT NULL
        AND completed_at < ?
        AND archived_at IS NULL
    `,
      )
      .all(thirtyDaysAgo) as StreamRow[];

    if (streamsToArchive.length === 0) {
      return 0;
    }

    const now = nowInSeconds();
    let archived = 0;

    db.transaction(() => {
      for (const row of streamsToArchive) {
        const record = rowToRecord(row);
        record.refundedAmount = row.refunded_amount ?? undefined;

        // Insert into archive
        db.prepare(
          `
        INSERT INTO stream_archive (id, sender, recipient, asset_code, total_amount, duration_seconds, start_at, created_at, canceled_at, completed_at, refunded_amount, archived_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `,
        ).run(
          record.id,
          record.sender,
          record.recipient,
          record.assetCode,
          record.totalAmount,
          record.durationSeconds,
          record.startAt,
          record.createdAt,
          record.canceledAt ?? null,
          record.completedAt ?? null,
          record.refundedAmount ?? null,
          now,
        );

        // Mark as archived in main table
        db.prepare("UPDATE streams SET archived_at = ? WHERE id = ?").run(now, record.id);
        archived++;
      }
    })();

    console.log(`[archive] archived ${archived} completed stream(s)`);
    return archived;
  } catch (err) {
    console.error("[archive] failed to archive old streams:", err);
    return 0;
  }
}

export function listStreams(includeArchived = false): StreamRecord[] {
  const db = getDb();
  const query = includeArchived
    ? "SELECT * FROM streams ORDER BY created_at DESC"
    : "SELECT * FROM streams WHERE archived_at IS NULL ORDER BY created_at DESC";
  const rows = db.prepare(query).all() as StreamRow[];
  return rows.map(rowToRecord);
}

export function listStreamsByRecipient(recipientAddress: string): StreamRecord[] {
  const db = getDb();
  const rows = db
    .prepare("SELECT * FROM streams WHERE recipient = ? ORDER BY created_at DESC")
    .all(recipientAddress) as StreamRow[];
  return rows.map(rowToRecord);
}

export function listStreamsBySender(senderAddress: string): StreamRecord[] {
  const db = getDb();
  const rows = db
    .prepare("SELECT * FROM streams WHERE sender = ? ORDER BY created_at DESC")
    .all(senderAddress) as StreamRow[];
  return rows.map(rowToRecord);
}

export function getStream(id: string): StreamRecord | undefined {
  const db = getDb();
  const row = db.prepare("SELECT * FROM streams WHERE id = ?").get(id) as
    | StreamRow
    | undefined;
  return row ? rowToRecord(row) : undefined;
}

export async function cancelStream(
  id: string,
): Promise<StreamRecord | undefined> {
  const stream = getStream(id);
  if (!stream || stream.canceledAt !== undefined) {
    return stream;
  }

  stream.canceledAt = nowInSeconds();

  // Attempt to get refund amount from on-chain cancel transaction.
  // For now, we extract from potential on-chain response. In production,
  // this would send an actual cancel_stream transaction to the contract.
  let refundAmount: number | undefined = undefined;
  try {
    const sorobanContext = getSorobanContext();
    if (sorobanContext && rpcServer && serverKeypair) {
      const contractId = process.env.CONTRACT_ID;
      if (contractId) {
        const sourceAccount = await rpcServer.getAccount(serverKeypair.publicKey());
        const contract = new Contract(contractId);
        const tx = contract.call(
          "cancel_stream",
          nativeToScVal(parseInt(id), { type: "u64" }),
        );

        const built = await rpcServer.prepareTransaction(
          new TransactionBuilder(sourceAccount, {
            fee: "1000",
            networkPassphrase: process.env.NETWORK_PASSPHRASE || Networks.TESTNET,
          })
            .addOperation(tx)
            .setTimeout(30)
            .build(),
        );

        built.sign(serverKeypair);
        const sendRes = await retryWithBackoff(() => rpcServer!.sendTransaction(built));
        if (sendRes.status === "PENDING") {
          let txResult;
          let attempts = 0;
          while (attempts < 10) {
            txResult = await retryWithBackoff(() =>
              rpcServer!.getTransaction(sendRes.hash),
            );
            if (txResult.status !== "NOT_FOUND") break;
            await new Promise((r) => setTimeout(r, 1000));
            attempts++;
          }

          if (txResult?.status === "SUCCESS" && txResult.returnValue) {
            refundAmount = Number(scValToNative(txResult.returnValue));
            stream.refundedAmount = refundAmount;
          }
        }
      }
    }
  } catch (err) {
    console.warn(
      `[cancel] failed to get refund amount from chain for stream ${id}:`,
      err,
    );
  }

  // Invalidate cache
  invalidateCache(`stream:${id}`);

  // Atomically write the updated stream row and the cancellation event.
  const db = getDb();
  db.transaction(() => {
    upsertStream(stream);
    recordEventWithDb(db, stream.id, "canceled", stream.canceledAt!, stream.sender);
  })();

  // Webhook fires after the transaction commits.
  triggerWebhook("canceled", stream);
  return stream;
}

export function updateStreamStartAt(
  id: string,
  newStartAt: number,
): StreamRecord {
  const stream = getStream(id);
  if (!stream) {
    const err: any = new Error("Stream not found.");
    err.statusCode = 404;
    throw err;
  }

  const status = computeStatus(stream, nowInSeconds());
  if (status !== "scheduled") {
    const err: any = new Error(
      "Can only update start time for scheduled streams.",
    );
    err.statusCode = 400;
    throw err;
  }

  // Capture oldStartAt before mutating the record.
  const oldStartAt = stream.startAt;
  stream.startAt = newStartAt;
  const updatedAt = nowInSeconds();

  // Atomically write the updated stream row and the start-time event.
  const db = getDb();
  db.transaction(() => {
    upsertStream(stream);
    recordEventWithDb(
      db,
      stream.id,
      "start_time_updated",
      updatedAt,
      stream.sender,
      undefined,
      { oldStartAt, newStartAt },
    );
  })();

  return stream;
}

