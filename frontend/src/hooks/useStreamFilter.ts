import { useCallback, useEffect, useMemo, useState } from "react";
import { Stream } from "../types/stream";

export interface StreamFilters {
  status: string;
  sender: string;
  recipient: string;
  assetCode: string;
  q: string;
}

type FilterKey = keyof StreamFilters;

const EMPTY_FILTERS: StreamFilters = {
  status: "",
  sender: "",
  recipient: "",
  assetCode: "",
  q: "",
};

function getFiltersFromUrl(): StreamFilters {
  const params = new URLSearchParams(window.location.search);
  return {
    status: params.get("status") ?? "",
    sender: params.get("sender") ?? "",
    recipient: params.get("recipient") ?? "",
    assetCode: params.get("assetCode") ?? params.get("asset") ?? "",
    q: params.get("q") ?? "",
  };
}

function includesCaseInsensitive(value: string, needle: string): boolean {
  return value.toLowerCase().includes(needle.toLowerCase());
}

export function useStreamFilter(streams: Stream[]) {
  const [filters, setFilters] = useState<StreamFilters>(() => getFiltersFromUrl());

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const syncValue = (key: FilterKey) => {
      const value = filters[key].trim();
      if (!value) {
        params.delete(key);
        if (key === "assetCode") params.delete("asset");
        return;
      }
      params.set(key, value);
      if (key === "assetCode") params.set("asset", value);
    };

    syncValue("status");
    syncValue("sender");
    syncValue("recipient");
    syncValue("assetCode");
    syncValue("q");

    const next = params.toString();
    const nextUrl = next ? `${window.location.pathname}?${next}` : window.location.pathname;
    window.history.replaceState(null, "", nextUrl);
  }, [filters]);

  useEffect(() => {
    const onPopState = () => setFilters(getFiltersFromUrl());
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);

  const setFilter = useCallback((key: FilterKey, value: string) => {
    setFilters((prev) => ({ ...prev, [key]: value }));
  }, []);

  const filteredStreams = useMemo(() => {
    return streams.filter((stream) => {
      if (filters.status && stream.progress.status !== filters.status) return false;
      if (filters.sender && !includesCaseInsensitive(stream.sender, filters.sender)) {
        return false;
      }
      if (
        filters.recipient &&
        !includesCaseInsensitive(stream.recipient, filters.recipient)
      ) {
        return false;
      }
      if (
        filters.assetCode &&
        !includesCaseInsensitive(stream.assetCode, filters.assetCode)
      ) {
        return false;
      }
      if (filters.q) {
        const search = filters.q.toLowerCase();
        const matches =
          stream.id.toLowerCase().includes(search) ||
          stream.sender.toLowerCase().includes(search) ||
          stream.recipient.toLowerCase().includes(search) ||
          stream.assetCode.toLowerCase().includes(search);
        if (!matches) return false;
      }
      return true;
    });
  }, [filters, streams]);

  return { filters, setFilter, filteredStreams };
}

export const defaultStreamFilters = EMPTY_FILTERS;
