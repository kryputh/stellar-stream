import { createHmac } from "crypto";

/**
 * Computes the HMAC-SHA256 signature for a webhook payload.
 * 
 * @param payload The raw string body of the webhook request.
 * @param secret The WEBHOOK_SIGNING_SECRET.
 * @returns The hex-encoded HMAC-SHA256 signature.
 */
export function computeWebhookSignature(payload: string, secret: string): string {
  return createHmac("sha256", secret).update(payload).digest("hex");
}

/**
 * Generates the headers for a webhook request, including the signature if a secret is provided.
 * 
 * @param payload The raw string body of the webhook request.
 * @param secret Optional WEBHOOK_SIGNING_SECRET.
 * @returns A record of headers.
 */
export function getWebhookHeaders(payload: string, secret?: string): Record<string, string> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (secret) {
    const signature = computeWebhookSignature(payload, secret);
    headers["X-Webhook-Signature"] = `sha256=\${signature}`;
  }

  return headers;
}
