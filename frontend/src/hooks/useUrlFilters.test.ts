import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useUrlFilters } from "./useUrlFilters";

describe("useUrlFilters", () => {
    beforeEach(() => {
        // Reset URL before each test
        const url = new URL("http://localhost/");
        vi.stubGlobal("location", {
            ...window.location,
            href: url.href,
            search: url.search,
            pathname: url.pathname,
        });

        // Mock history methods
        vi.stubGlobal("history", {
            replaceState: vi.fn((state, title, url) => {
                const newUrl = new URL(url, "http://localhost/");
                window.location.search = newUrl.search;
            }),
            pushState: vi.fn((state, title, url) => {
                const newUrl = new URL(url, "http://localhost/");
                window.location.search = newUrl.search;
            }),
        });
    });

    it("should initialize filter state from URL parameters", () => {
        const url = new URL("http://localhost/?view=recipient&status=active&asset=USDC");
        vi.stubGlobal("location", {
            ...window.location,
            search: url.search,
        });

        const { result } = renderHook(() => useUrlFilters());

        expect(result.current.view).toBe("recipient");
        expect(result.current.filters).toEqual({
            status: "active",
            asset: "USDC",
            sender: "",
            recipient: "",
        });
    });

    it("should update URL search parameters when a filter is set", () => {
        const { result } = renderHook(() => useUrlFilters());

        act(() => {
            result.current.setFilters({
                status: "completed",
                asset: "XLM",
                sender: "",
                recipient: "",
            });
        });

        // The useEffect handles the history.replaceState
        expect(window.history.replaceState).toHaveBeenCalled();
        const searchParams = new URLSearchParams(window.location.search);
        expect(searchParams.get("status")).toBe("completed");
        expect(searchParams.get("asset")).toBe("XLM");
    });

    it("should remove parameters from URL when filters are cleared", () => {
        const url = new URL("http://localhost/?status=active&asset=USDC");
        vi.stubGlobal("location", {
            ...window.location,
            search: url.search,
        });

        const { result } = renderHook(() => useUrlFilters());

        act(() => {
            result.current.setFilters({
                status: "",
                asset: "",
                sender: "",
                recipient: "",
            });
        });

        const searchParams = new URLSearchParams(window.location.search);
        expect(searchParams.has("status")).toBe(false);
        expect(searchParams.has("asset")).toBe(false);
    });

    it("should handle multiple filters simultaneously", () => {
        const { result } = renderHook(() => useUrlFilters());

        act(() => {
            result.current.setFilters({
                status: "canceled",
                asset: "EURC",
                sender: "G123...",
                recipient: "G456...",
            });
        });

        const searchParams = new URLSearchParams(window.location.search);
        expect(searchParams.get("status")).toBe("canceled");
        expect(searchParams.get("asset")).toBe("EURC");
        expect(searchParams.get("sender")).toBe("G123...");
        expect(searchParams.get("recipient")).toBe("G456...");
    });

    it("should update view mode and push to history", () => {
        const { result } = renderHook(() => useUrlFilters());

        act(() => {
            result.current.setView("sender");
        });

        expect(window.history.pushState).toHaveBeenCalled();
        expect(result.current.view).toBe("sender");
        const searchParams = new URLSearchParams(window.location.search);
        expect(searchParams.get("view")).toBe("sender");
    });

    it("should handle opening and closing streams via URL", () => {
        const { result } = renderHook(() => useUrlFilters());

        act(() => {
            result.current.openStream("stream_99");
        });

        expect(result.current.streamId).toBe("stream_99");
        expect(new URLSearchParams(window.location.search).get("streamId")).toBe("stream_99");

        act(() => {
            result.current.closeStream();
        });

        expect(result.current.streamId).toBe(null);
        expect(new URLSearchParams(window.location.search).has("streamId")).toBe(false);
    });
});
