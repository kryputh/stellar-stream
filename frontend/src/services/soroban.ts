/**
 * Soroban / on-chain interactions for the StellarStream frontend.
 *
 * `claimStream` calls the backend `/api/streams/:id/claim` endpoint which
 * builds, signs (fee-sponsored), and submits the Soroban `claim` transaction
 * on behalf of the recipient.  The backend returns the claimed amount and the
 * updated event history.
 */

import { getAuthToken } from "./api";
import type { StreamEvent } from "./api";

const API_BASE = (import.meta as any).env?.VITE_API_URL ?? "/api";

export interface ClaimResult {
  /** Amount of tokens transferred to the recipient in this claim. */
  claimedAmount: number;
  /** Asset code of the claimed tokens. */
  assetCode: string;
  /** Stellar transaction hash confirming the on-chain claim. */
  txHash: string;
}

export interface ClaimResponse {
  result: ClaimResult;
  history: StreamEvent[];
}

/**
 * Claim vested tokens from a stream.
 *
 * @param streamId       - Numeric stream ID.
 * @param recipientAddress - Stellar public key of the recipient (must match stream).
 * @param amount         - Claimable amount as reported by the backend (for display only;
 *                         the contract determines the actual claimable amount on-chain).
 */
export async function claimStream(
  streamId: string,
  recipientAddress: string,
  amount: number,
): Promise<ClaimResponse> {
  const token = getAuthToken();

  const response = await fetch(`${API_BASE}/streams/${streamId}/claim`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify({ recipientAddress, amount }),
  });

  if (!response.ok) {
    let message = `Claim failed (${response.status})`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body.error) message = body.error;
    } catch {
      // ignore JSON parse errors
    }
    throw new Error(message);
  }

  return response.json() as Promise<ClaimResponse>;
}
