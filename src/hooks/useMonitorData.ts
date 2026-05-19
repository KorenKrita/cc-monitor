import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { RequestRecord, TimeRange } from "../types";

export function useMonitorData() {
  const [requests, setRequests] = useState<RequestRecord[]>([]);
  const [models, setModels] = useState<string[]>([]);
  const [latest, setLatest] = useState<RequestRecord | null>(null);

  useEffect(() => {
    const unlisten = listen<RequestRecord>("new-request", (event) => {
      setRequests((prev) => [...prev, event.payload]);
      setLatest(event.payload);
      setModels((prev) => {
        if (prev.includes(event.payload.model)) return prev;
        return [...prev, event.payload.model];
      });
    });

    invoke<string[]>("get_models").then(setModels).catch(console.error);
    invoke<RequestRecord | null>("get_latest").then((r) => { if (r) setLatest(r); }).catch(console.error);

    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const fetchData = useCallback(async (timeRange: TimeRange, modelFilter?: string[]) => {
    const since = getSinceTimestamp(timeRange);
    const data = await invoke<RequestRecord[]>("get_requests", {
      since,
      models: modelFilter && modelFilter.length > 0 ? modelFilter : null,
    });
    setRequests(data);
  }, []);

  return { requests, models, latest, fetchData };
}

function getSinceTimestamp(range: TimeRange): string {
  const now = new Date();
  switch (range) {
    case "1h":
      return new Date(now.getTime() - 60 * 60 * 1000).toISOString();
    case "today":
      return new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString();
    case "yesterday": {
      const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      return yesterday.toISOString();
    }
  }
}
