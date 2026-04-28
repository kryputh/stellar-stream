import Database from "better-sqlite3";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const dbMocks = vi.hoisted(() => ({
  getDb: vi.fn(),
  initDb: vi.fn(),
}));

vi.mock("./db", () => dbMocks);

function createTestDb() {
  const db = new Database(":memory:");
  db.exec(`
    CREATE TABLE stream_events (
      id              INTEGER PRIMARY KEY AUTOINCREMENT,
      stream_id       TEXT NOT NULL,
      event_type      TEXT NOT NULL,
      timestamp       INTEGER NOT NULL,
      actor           TEXT,
      amount          REAL,
      metadata        TEXT
    );
  `);
  return db;
}

describe("eventHistory", () => {
  let db: ReturnType<typeof createTestDb>;

  beforeEach(() => {
    db = createTestDb();
    dbMocks.getDb.mockReturnValue(db);
  });

  afterEach(() => {
    db.close();
    vi.clearAllMocks();
  });

  describe("recordEvent", () => {
    it("stores both events when two are recorded for the same stream", async () => {
      const { recordEvent, countStreamEvents, getStreamHistory } = await import(
        "./eventHistory"
      );

      recordEvent("stream-1", "created", 1000, "GACTOR1", 100);
      recordEvent("stream-1", "claimed", 2000, "GACTOR2", 50);

      expect(countStreamEvents("stream-1")).toBe(2);

      const history = getStreamHistory("stream-1");
      expect(history).toHaveLength(2);
      expect(history.map((e) => e.eventType)).toEqual(["created", "claimed"]);
    });

    it("stores duplicate event types for the same stream without deduping", async () => {
      const { recordEvent, countStreamEvents } = await import("./eventHistory");

      recordEvent("stream-2", "claimed", 1000, "GACTOR1", 25);
      recordEvent("stream-2", "claimed", 2000, "GACTOR1", 25);

      expect(countStreamEvents("stream-2")).toBe(2);
    });
  });

  describe("recordEventWithDb", () => {
    it("inserts using the provided db handle", async () => {
      const { recordEventWithDb, getStreamHistory } = await import(
        "./eventHistory"
      );

      recordEventWithDb(db, "stream-3", "created", 500, "GACTOR3", 200, {
        note: "test",
      });

      const history = getStreamHistory("stream-3");
      expect(history).toHaveLength(1);
      expect(history[0]).toMatchObject({
        streamId: "stream-3",
        eventType: "created",
        timestamp: 500,
        actor: "GACTOR3",
        amount: 200,
        metadata: { note: "test" },
      });
    });
  });

  describe("getStreamHistory ordering", () => {
    it("returns events ascending by timestamp even when inserted out of order", async () => {
      const { recordEvent, getStreamHistory } = await import("./eventHistory");

      recordEvent("stream-4", "claimed", 3000);
      recordEvent("stream-4", "created", 1000);
      recordEvent("stream-4", "start_time_updated", 2000);
      recordEvent("stream-4", "canceled", 4000);

      const history = getStreamHistory("stream-4");

      expect(history.map((e) => e.timestamp)).toEqual([1000, 2000, 3000, 4000]);
      expect(history.map((e) => e.eventType)).toEqual([
        "created",
        "start_time_updated",
        "claimed",
        "canceled",
      ]);
    });

    it("breaks ties on equal timestamps by insertion order (id ASC)", async () => {
      const { recordEvent, getStreamHistory } = await import("./eventHistory");

      recordEvent("stream-5", "created", 1000, "first");
      recordEvent("stream-5", "claimed", 1000, "second");
      recordEvent("stream-5", "canceled", 1000, "third");

      const history = getStreamHistory("stream-5");

      expect(history.map((e) => e.actor)).toEqual(["first", "second", "third"]);
    });

    it("isolates events by streamId", async () => {
      const { recordEvent, getStreamHistory } = await import("./eventHistory");

      recordEvent("stream-A", "created", 1000);
      recordEvent("stream-B", "created", 500);
      recordEvent("stream-A", "claimed", 2000);

      const historyA = getStreamHistory("stream-A");
      const historyB = getStreamHistory("stream-B");

      expect(historyA).toHaveLength(2);
      expect(historyA.map((e) => e.timestamp)).toEqual([1000, 2000]);
      expect(historyB).toHaveLength(1);
      expect(historyB[0].timestamp).toBe(500);
    });
  });
});
