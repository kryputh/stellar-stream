import { getDb } from "./db";

const MAX_RETRIES = 3;

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
    const now = Date.now();
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