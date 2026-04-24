import { describe, it, expect } from "vitest";
import { getRetryDelaySeconds } from "./webhook";

describe("Webhook Retry Logic", () => {
    it("should return correct retry delays", () => {
        const expectedDelays = [5, 15, 60, 300, 900];

        expectedDelays.forEach((expectedDelay, index) => {
            const delay = getRetryDelaySeconds(index);
            expect(delay).toBe(expectedDelay);
        });
    });

    it("should return last delay for attempts beyond max", () => {
        const delay = getRetryDelaySeconds(10);
        expect(delay).toBe(900); // Last delay
    });

    it("should handle negative attempt numbers", () => {
        const delay = getRetryDelaySeconds(-1);
        expect(delay).toBe(900); // Last delay
    });

    it("should have correct sequence: 5s, 15s, 60s, 300s, 900s", () => {
        const delays = [0, 1, 2, 3, 4].map((i) => getRetryDelaySeconds(i));
        expect(delays).toEqual([5, 15, 60, 300, 900]);
    });
});
