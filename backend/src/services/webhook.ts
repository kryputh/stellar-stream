import { getDb } from "./db";

const MAX_RETRIES = 5;
const RETRY_DELAYS = [5, 15, 60, 300, 900]; // seconds: 5s, 15s, 60s, 300s, 900s

export const triggerWebhook = async (event: string, data: any): Promise<void> => {
  const url = process.env.WEBHOOK_DESTINATION_URL;

  if (!url) {
    console.log(`[Webhook] Skipping ${event}: WEBHOOK_DESTINATION_URL not set.`);
    return;
  }

  const streamId = data.stream_id || data.id;

  if (!streamId) {
    console.error(`[Webhook] Cannot map event ${event} to a stream ID. Data:`, data);
    return;
  }

  try {
    const db = getDb();
    const stmt = db.prepare(`
      INSERT INTO webhook_deliveries (stream_id, event, payload, attempt, max_attempts, status, next_retry_at, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `);

    // Queue for immediate delivery relative to the worker's polling cycle
    const now = Math.floor(Date.now() / 1000);
    stmt.run(
      streamId,
      event,
      JSON.stringify(data),
      0, // attempt
      MAX_RETRIES, // max_attempts
      'pending', // status
      now, // next_retry_at
      now // created_at
    );
    console.log(`[Webhook] Queued ${event} for stream ${streamId}.`);
  } catch (error: any) {
    console.error(`[Webhook] Failed to queue webhook event ${event}:`, error);
  }
};

export function getDeadLetters(limit = 100, offset = 0): any[] {
  const db = getDb();
  return db
    .prepare(`SELECT * FROM webhook_dead_letters ORDER BY failed_at DESC LIMIT ? OFFSET ?`)
    .all(limit, offset);
}

export function countDeadLetters(): number {
  const db = getDb();
  const row = db
    .prepare(`SELECT COUNT(*) as count FROM webhook_dead_letters`)
    .get() as { count: number };
  return row.count;
}

export function getRetryDelaySeconds(attemptNumber: number): number {
  if (attemptNumber < 0 || attemptNumber >= RETRY_DELAYS.length) {
    return RETRY_DELAYS[RETRY_DELAYS.length - 1];
  }
  return RETRY_DELAYS[attemptNumber];
}