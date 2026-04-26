import {
  Keypair,
  Networks,
  TransactionBuilder,
  WebAuth,
} from "@stellar/stellar-sdk";
import jwt from "jsonwebtoken";
import crypto from "crypto";
import { Request, Response, NextFunction } from "express";
import { sendApiError } from "../apiErrors";

const SERVER_SIGNING_KEY =
  process.env.SERVER_SIGNING_KEY || (process.env.NODE_ENV === 'production' 
    ? ((): string => { throw new Error("SERVER_SIGNING_KEY must be set in production") })() 
    : Keypair.random().secret());

const DOMAIN = (process.env.DOMAIN || "localhost").trim();
const NETWORK_PASSPHRASE = process.env.NETWORK_PASSPHRASE || Networks.TESTNET;

let jwtSecret = process.env.JWT_SECRET;

if (!jwtSecret) {
  if (process.env.NODE_ENV === "production") {
    throw new Error("JWT_SECRET must be set in production");
  }

  jwtSecret = crypto.randomBytes(32).toString("hex");

  console.warn(
    "JWT_SECRET not set — using ephemeral secret. All tokens will be invalidated on restart.",
  );
}

function getJwtSecret() {
  return jwtSecret as string;
}

export interface AuthUser {
  accountId: string;
}

export function generateChallenge(accountId: string): string {
  const serverKeypair = Keypair.fromSecret(SERVER_SIGNING_KEY);

  const challenge = WebAuth.buildChallengeTx(
    serverKeypair,
    accountId,
    DOMAIN,
    300, // Valid for 5 minutes
    NETWORK_PASSPHRASE,
    DOMAIN,
  );

  return challenge;
}

/**
 * Verifies a SEP-10 challenge transaction and issues a JWT.
 * Rejects if:
 * - Transaction is malformed or not a SEP-10 challenge
 * - Transaction has expired (stale)
 * - Domain/Network doesn't match
 * - Client signature is missing or invalid
 */
export function verifyChallengeAndIssueToken(
  transactionBase64: string,
): string {
  const serverKeypair = Keypair.fromSecret(SERVER_SIGNING_KEY);
  const serverAccountId = serverKeypair.publicKey();

  try {
    // readChallengeTx validates the transaction structure and server signature
    const { clientAccountID } = WebAuth.readChallengeTx(
      transactionBase64,
      serverAccountId,
      NETWORK_PASSPHRASE,
      DOMAIN,
      DOMAIN,
    );

    // verifyChallengeTxSigners ensures the clientAccountID actually signed it
    const signersFound = WebAuth.verifyChallengeTxSigners(
      transactionBase64,
      serverAccountId,
      NETWORK_PASSPHRASE,
      [clientAccountID],
      DOMAIN,
      DOMAIN,
    );

    const isSignedByClient = signersFound.some(signer => signer === clientAccountID);

    if (!isSignedByClient) {
      throw new Error(
        "Challenge transaction verification failed (invalid signature).",
      );
    }

    const token = jwt.sign({ accountId: clientAccountID }, getJwtSecret(), {
      expiresIn: "24h",
    });
    return token;
  } catch (error: any) {
    // Catch stale transaction errors specifically if needed
    if (error.message?.includes("TimeBounds")) {
      throw new Error("Challenge has expired. Please request a new one.");
    }
    throw new Error(`Challenge verification failed: ${error.message}`);
  }
}

/**
 * Refreshes a still-valid JWT and returns a new one with a fresh 24h expiry.
 *
 * Accepts the current token in the Authorization header (Bearer scheme).
 * Returns 401 if the token is missing, malformed, or already expired.
 */
export function refreshToken(req: Request, res: Response): void {
  const authHeader = req.headers.authorization;

  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    sendApiError(req, res, 401, "Missing or invalid authorization header.", {
      code: "UNAUTHORIZED",
    });
    return;
  }

  const token = authHeader.split(" ")[1];

  try {
    const decoded = jwt.verify(token, getJwtSecret()) as AuthUser;

    const newToken = jwt.sign(
      { accountId: decoded.accountId },
      getJwtSecret(),
      { expiresIn: "24h" },
    );

    res.json({ token: newToken });
  } catch {
    sendApiError(req, res, 401, "Invalid or expired authorization token.", {
      code: "UNAUTHORIZED",
    });
  }
}

export function authMiddleware(
  req: Request,
  res: Response,
  next: NextFunction,
) {
  const authHeader = req.headers.authorization;

  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    sendApiError(req, res, 401, "Missing or invalid authorization header.", {
      code: "UNAUTHORIZED",
    });
    return;
  }

  const token = authHeader.split(" ")[1];

  try {
    const decoded = jwt.verify(token, getJwtSecret()) as AuthUser;
    (req as any).user = decoded; // Attach user to request
    next();
  } catch (error) {
    sendApiError(req, res, 401, "Invalid or expired authorization token.", {
      code: "UNAUTHORIZED",
    });
  }
}
