import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Config } from "../types";

export function useSettings() {
  const [config, setConfig] = useState<Config>({
    theme: "system",
    tray: { items: ["out_rate", "in_rate", "ttft"], model_filter: "last", model_whitelist: [], display_mode: "last", average_minutes: 5 },
    model_aliases: {},
    cost: { time_window: "day", time_window_value: 1, project_whitelist: [], model_whitelist: [], model_prices: {}, last_sync_time: null, watch_sources: ["claude", "codex"], sync_source: "all" },
  });

  useEffect(() => {
    invoke<Config>("get_config").then(setConfig).catch(console.error);
  }, []);

  const resolvedTheme = (): "dark" | "light" => {
    if (config.theme === "system") {
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    return config.theme;
  };

  return { config, setConfig, resolvedTheme };
}
