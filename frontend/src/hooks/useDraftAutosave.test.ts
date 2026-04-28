import { renderHook, act } from "@testing-library/react";
import { describe, expect, it, beforeEach, vi, afterEach } from "vitest";
import { useDraftAutosave } from "./useDraftAutosave";

describe("useDraftAutosave", () => {
  const KEY = "test-draft-key";
  const INITIAL_VALUE = { name: "", email: "" };

  beforeEach(() => {
    vi.useFakeTimers();
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("initializes with initialValue when localStorage is empty", () => {
    const { result } = renderHook(() => useDraftAutosave(KEY, INITIAL_VALUE));
    const [value, , hasDraft] = result.current;
    
    expect(value).toEqual(INITIAL_VALUE);
    expect(hasDraft).toBe(false);
  });

  it("restores draft from localStorage on mount", () => {
    const draft = { name: "John Doe", email: "john@example.com" };
    window.localStorage.setItem(KEY, JSON.stringify(draft));

    const { result } = renderHook(() => useDraftAutosave(KEY, INITIAL_VALUE));
    const [value, , hasDraft] = result.current;

    expect(value).toEqual(draft);
    expect(hasDraft).toBe(true);
  });

  it("debounces writes to localStorage", () => {
    const { result } = renderHook(() => useDraftAutosave(KEY, INITIAL_VALUE, 500));
    const [, setValue] = result.current;

    const newValue = { name: "Alice", email: "alice@example.com" };
    
    act(() => {
      setValue(newValue);
    });

    // Should not be in localStorage immediately
    expect(window.localStorage.getItem(KEY)).toBeNull();

    // Advance timers by 499ms
    act(() => {
      vi.advanceTimersByTime(499);
    });
    expect(window.localStorage.getItem(KEY)).toBeNull();

    // Advance to 500ms
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(JSON.parse(window.localStorage.getItem(KEY)!)).toEqual(newValue);
  });

  it("clears draft and removes from localStorage when value equals initialValue", () => {
    const draft = { name: "John", email: "john@example.com" };
    window.localStorage.setItem(KEY, JSON.stringify(draft));

    const { result } = renderHook(() => useDraftAutosave(KEY, INITIAL_VALUE, 500));
    const [, setValue] = result.current;

    act(() => {
      setValue(draft); // Set same value first to ensure state is synchronized
      vi.advanceTimersByTime(500);
    });
    expect(JSON.parse(window.localStorage.getItem(KEY)!)).toEqual(draft);

    act(() => {
      setValue(INITIAL_VALUE);
    });
    
    // Still there before debounce
    expect(window.localStorage.getItem(KEY)).not.toBeNull();

    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(window.localStorage.getItem(KEY)).toBeNull();
    const [, , hasDraft] = result.current;
    expect(hasDraft).toBe(false);
  });

  it("manually clears draft via clearDraft function", () => {
    const draft = { name: "John", email: "john@example.com" };
    window.localStorage.setItem(KEY, JSON.stringify(draft));

    const { result } = renderHook(() => useDraftAutosave(KEY, INITIAL_VALUE));
    const [, , , clearDraft] = result.current;

    act(() => {
      clearDraft();
    });

    expect(window.localStorage.getItem(KEY)).toBeNull();
    const [value, , hasDraft] = result.current;
    expect(value).toEqual(INITIAL_VALUE);
    expect(hasDraft).toBe(false);
  });

  it("handles malformed JSON in localStorage gracefully", () => {
    window.localStorage.setItem(KEY, "invalid-json");
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    const { result } = renderHook(() => useDraftAutosave(KEY, INITIAL_VALUE));
    const [value] = result.current;

    expect(value).toEqual(INITIAL_VALUE);
    expect(warnSpy).toHaveBeenCalled();
    warnSpy.mockRestore();
  });
});
