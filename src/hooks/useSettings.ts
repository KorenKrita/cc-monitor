import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Config } from "../types";

export function useSettings() {
  const [config, setConfig] = useState<Config>({
    theme: "system",
    tray: { items: ["out_rate", "in_rate", "ttft"], model_filter: "last", model_whitelist: [] },
    model_aliases: {},
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
