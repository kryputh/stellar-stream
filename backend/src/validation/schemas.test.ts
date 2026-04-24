import { describe, it, expect } from "vitest";
import {
    stellarAccountIdSchema,
    webhookRegistrationSchema,
    isStellarPublicKey,
} from "./schemas";

describe("Stellar Account ID Validation", () => {
    it("should accept valid Stellar public keys", () => {
        const validKeys = [
            "GCLJJD5FHTSEBHFXAA3BBTODUJ4RXDK6B3OSVNKY6TUHF76AQQT2WNFC",
            "GBLHBYX72TJQH5EVPUN4ATAREH6TWYXQAH37MHNCVQG2NKLHFDSMFS3D",
            "GANNU4KAOYHV6FSY7Z44QWUEUCRBH56Y5BOP6NP6OKU3AUL3B54V34HU",
        ];

        validKeys.forEach((key) => {
            const result = stellarAccountIdSchema.safeParse(key);
            expect(result.success).toBe(true);
        });
    });

    it("should reject invalid Stellar public keys", () => {
        const invalidKeys = [
            "GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB3BHPOMONNHJGKYJPJJY",
            "GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB3BHPOMONNHJGKYJPJJYFF",
            "SBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB3BHPOMONNHJGKYJPJJYF",
            "CBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNQB3BHPOMONNHJGKYJPJJYF",
            "not-a-key",
            "",
        ];

        invalidKeys.forEach((key) => {
            const result = stellarAccountIdSchema.safeParse(key);
            expect(result.success).toBe(false);
        });
    });

    it("should trim whitespace", () => {
        const result = stellarAccountIdSchema.safeParse(
            "  GCLJJD5FHTSEBHFXAA3BBTODUJ4RXDK6B3OSVNKY6TUHF76AQQT2WNFC  "
        );
        expect(result.success).toBe(true);
    });
});

describe("Webhook Registration Schema", () => {
    it("should accept valid webhook registration", () => {
        const validPayload = {
            url: "https://example.com/webhooks",
            events: ["created", "claimed"],
            secret: "my-secret-key-1234567890",
        };

        const result = webhookRegistrationSchema.safeParse(validPayload);
        expect(result.success).toBe(true);
    });

    it("should reject http URLs", () => {
        const payload = {
            url: "http://example.com/webhooks",
            events: ["created"],
        };

        const result = webhookRegistrationSchema.safeParse(payload);
        expect(result.success).toBe(false);
        expect(result.error?.issues[0].message).toContain("https");
    });

    it("should reject empty events array", () => {
        const payload = {
            url: "https://example.com/webhooks",
            events: [],
        };

        const result = webhookRegistrationSchema.safeParse(payload);
        expect(result.success).toBe(false);
    });

    it("should reject invalid event types", () => {
        const payload = {
            url: "https://example.com/webhooks",
            events: ["created", "invalid_event"],
        };

        const result = webhookRegistrationSchema.safeParse(payload);
        expect(result.success).toBe(false);
    });

    it("should reject secret shorter than 16 chars", () => {
        const payload = {
            url: "https://example.com/webhooks",
            events: ["created"],
            secret: "short",
        };

        const result = webhookRegistrationSchema.safeParse(payload);
        expect(result.success).toBe(false);
    });

    it("should accept secret with exactly 16 chars", () => {
        const payload = {
            url: "https://example.com/webhooks",
            events: ["created"],
            secret: "1234567890123456",
        };

        const result = webhookRegistrationSchema.safeParse(payload);
        expect(result.success).toBe(true);
    });

    it("should allow optional secret", () => {
        const payload = {
            url: "https://example.com/webhooks",
            events: ["created", "claimed", "canceled"],
        };

        const result = webhookRegistrationSchema.safeParse(payload);
        expect(result.success).toBe(true);
    });

    it("should accept all valid event types", () => {
        const payload = {
            url: "https://example.com/webhooks",
            events: ["created", "claimed", "canceled", "start_time_updated"],
        };

        const result = webhookRegistrationSchema.safeParse(payload);
        expect(result.success).toBe(true);
    });
});
