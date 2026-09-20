"use client";

import { useCallback, useEffect, useState } from "react";
import { request } from "@/lib/api";

/** One request at a time; aborts on navigation and preserves data on transient errors. */
export function usePolling<T>(url: string | null, interval: number) {
  const [revision, setRevision] = useState(0);
  const [state, setState] = useState<{
    url: string | null;
    data?: T;
    error?: Error;
  }>({ url: null });
  const [loading, setLoading] = useState(true);
  const update = useCallback(
    (transform: (data: T) => T) => {
      setState((previous) =>
        previous.url === url && previous.data
          ? { ...previous, data: transform(previous.data) }
          : previous,
      );
    },
    [url],
  );
  const refresh = useCallback(() => setRevision((value) => value + 1), []);

  useEffect(() => {
    if (!url) {
      setLoading(false);
      return;
    }
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    // Only the initial fetch for a URL is "loading"; background polls keep the
    // previous data on screen so the UI never flickers between cycles.
    let initial = true;
    const poll = async () => {
      if (initial) setLoading(true);
      try {
        const data = await request<T>(url, { signal: controller.signal });
        if (!controller.signal.aborted)
          setState((previous) =>
            previous.url === url &&
            JSON.stringify(previous.data) === JSON.stringify(data)
              ? previous
              : { url, data },
          );
      } catch (error) {
        if (!controller.signal.aborted)
          setState((previous) => ({
            url,
            data: previous.url === url ? previous.data : undefined,
            error: error as Error,
          }));
      } finally {
        if (!controller.signal.aborted) {
          if (initial) {
            initial = false;
            setLoading(false);
          }
          timer = setTimeout(poll, interval);
        }
      }
    };
    void poll();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [url, interval, revision]);

  return {
    data: state.url === url ? state.data : undefined,
    error: state.url === url ? state.error : undefined,
    loading,
    refresh,
    update,
  };
}
