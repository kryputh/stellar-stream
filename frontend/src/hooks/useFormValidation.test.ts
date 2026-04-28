import { describe, expect, it } from "vitest";
import { validateForm, isFormValid, FormValues } from "./useFormValidation";

const validValues: FormValues = {
  sender: "GBX5ZID6H4G365G7O4W6U4E6U4E6U4E6U4E6U4E6U4E6U4E6U4E6U4E6",
  recipient: "GDBX5ZID6H4G365G7O4W6U4E6U4E6U4E6U4E6U4E6U4E6U4E6U4E6U4E6U", // 56 chars, different from sender
  assetCode: "USDC",
  totalAmount: "100",
  durationHours: "24",
  startInMinutes: "0",
};

// Helper to create a valid account ID of 56 chars
const createMockAccount = (char: string) => `G${char.repeat(55)}`;

describe("useFormValidation", () => {
  const sender = createMockAccount("A");
  const recipient = createMockAccount("B");

  const defaultValid: FormValues = {
    ...validValues,
    sender,
    recipient,
  };

  it("returns no errors for valid values", () => {
    const errors = validateForm(defaultValid);
    expect(errors).toEqual({});
    expect(isFormValid(errors)).toBe(true);
  });

  it("detects required fields when empty", () => {
    const emptyValues: FormValues = {
      sender: "",
      recipient: "",
      assetCode: "",
      totalAmount: "",
      durationHours: "",
      startInMinutes: "",
    };

    const errors = validateForm(emptyValues);
    
    expect(errors.sender).toBe("Sender account is required.");
    // Recipient error comes from stellarAccountIdSchema if empty
    expect(errors.recipient).toBe("Account ID is required.");
    expect(errors.assetCode).toBe("Asset code is required.");
    expect(errors.totalAmount).toBe("Total amount is required.");
    expect(errors.durationHours).toBe("Duration is required.");
    expect(errors.startInMinutes).toBe("Enter 0 to start immediately, or a positive number of minutes.");
    
    expect(isFormValid(errors)).toBe(false);
  });

  it("validates Stellar account format", () => {
    const invalidAccountValues: FormValues = {
      ...defaultValid,
      sender: "INVALID",
    };

    const errors = validateForm(invalidAccountValues);
    expect(errors.sender).toMatch(/valid Stellar account ID/i);
    expect(isFormValid(errors)).toBe(false);
  });

  it("validates that recipient must differ from sender", () => {
    const sameAccountValues: FormValues = {
      ...defaultValid,
      recipient: sender,
    };

    const errors = validateForm(sameAccountValues);
    expect(errors.recipient).toBe("Recipient must differ from the sender account.");
    expect(isFormValid(errors)).toBe(false);
  });

  it("validates numeric fields correctly", () => {
    const invalidNumericValues: FormValues = {
      ...defaultValid,
      totalAmount: "-10",
      durationHours: "0.5", // Must be integer >= 1
      startInMinutes: "-5",
    };

    const errors = validateForm(invalidNumericValues);
    expect(errors.totalAmount).toMatch(/greater than zero/i);
    expect(errors.durationHours).toBe("Duration must be a whole number of hours, minimum 1.");
    expect(errors.startInMinutes).toBe("Must be 0 or a positive whole number.");
    expect(isFormValid(errors)).toBe(false);
  });

  it("clears error map when field is fixed", () => {
    let values: FormValues = { ...defaultValid, assetCode: "???" };
    let errors = validateForm(values);
    expect(errors.assetCode).toBeDefined();
    expect(isFormValid(errors)).toBe(false);

    // Fix the field
    values.assetCode = "USDC";
    errors = validateForm(values);
    expect(errors.assetCode).toBeUndefined();
    expect(isFormValid(errors)).toBe(true);
  });

  it("maintains multiple errors simultaneously", () => {
    const values: FormValues = {
      ...defaultValid,
      sender: "short",
      assetCode: "",
    };

    const errors = validateForm(values);
    expect(errors.sender).toBeDefined();
    expect(errors.assetCode).toBeDefined();
    expect(Object.keys(errors)).toHaveLength(2);
    expect(isFormValid(errors)).toBe(false);
  });
});
