import { describe, expect, it, vi, afterEach } from "vitest";
import { ApiError, listStreams } from "./api";

describe("api error handling", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("throws ApiError with statusCode when API returns 400", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ error: "Invalid request" }), {
        status: 400,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await expect(listStreams()).rejects.toMatchObject({
      name: "ApiError",
      statusCode: 400,
      message: "Invalid request",
    });
  });
});
