import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { RequestRecord, TimeRange } from "../types";

export function useMonitorData() {
  const [requests, setRequests] = useState<RequestRecord[]>([]);
  const [models, setModels] = useState<string[]>([]);
  const [latest, setLatest] = useState<RequestRecord | null>(null);
  const currentRange = useRef<{ timeRange: TimeRange; modelFilter?: string[] }>({ timeRange: "1h" });

  useEffect(() => {
    const unlisten = listen<RequestRecord>("new-request", (event) => {
      setRequests((prev) => {
        const next = [...prev, event.payload];
        return next.length > 500 ? next.slice(-500) : next;
      });
      setLatest(event.payload);
      setModels((prev) => {
        if (prev.includes(event.payload.model)) return prev;
        return [...prev, event.payload.model];
      });
    });

    invoke<string[]>("get_models").then(setModels).catch(console.error);
    invoke<RequestRecord | null>("get_latest").then((r) => { if (r) setLatest(r); }).catch(console.error);

    // Refresh data when window becomes visible
    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        const { timeRange, modelFilter } = currentRange.current;
        fetchData(timeRange, modelFilter);
        invoke<string[]>("get_models").then(setModels).catch(console.error);
        invoke<RequestRecord | null>("get_latest").then((r) => { if (r) setLatest(r); }).catch(console.error);
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      unlisten.then((fn) => fn());
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  const fetchData = useCallback(async (timeRange: TimeRange, modelFilter?: string[]) => {
    currentRange.current = { timeRange, modelFilter };
    const { since, until } = getTimeRange(timeRange);
    const data = await invoke<RequestRecord[]>("get_requests", {
      since,
      until: until || null,
      models: modelFilter && modelFilter.length > 0 ? modelFilter : null,
    });
    setRequests(data);
  }, []);

  return { requests, models, latest, fetchData };
}

function getTimeRange(range: TimeRange): { since: string; until: string | null } {
  const now = new Date();
  switch (range) {
    case "1h":
      return {
        since: new Date(now.getTime() - 60 * 60 * 1000).toISOString(),
        until: null,
      };
    case "today":
      return {
        since: new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString(),
        until: null,
      };
    case "yesterday": {
      const yesterdayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
      return {
        since: yesterdayStart.toISOString(),
        until: todayStart.toISOString(),
      };
    }
  }
}
