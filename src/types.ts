export interface RequestRecord {
  id: number;
  timestamp: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  duration_ms: number | null;
  project: string;
  source: string;
}

export type Metric = "out_rate" | "in_rate" | "ttft" | "cost";
export type TimeRange = "1h" | "today" | "yesterday";
export type Theme = "system" | "dark" | "light";
export type CostTimeUnit = "day" | "month" | "year" | "all";
export type WatchSource = "claude" | "codex";

export interface ModelPrice {
  input: number;
  output: number;
  cache: number;
  source: string;
}

export type SyncSource = "litellm" | "models.dev" | "basellm" | "all";

export interface CostConfig {
  time_window: CostTimeUnit;
  time_window_value: number;
  project_whitelist: string[];
  model_whitelist: string[];
  model_prices: Record<string, ModelPrice>;
  last_sync_time: string | null;
  watch_sources: WatchSource[];
  sync_source: SyncSource;
}

export interface Config {
  theme: Theme;
  tray: {
    items: Metric[];
    model_filter: "last" | "whitelist";
    model_whitelist: string[];
    display_mode: "last" | "average";
    average_minutes: number;
  };
  model_aliases: Record<string, string>;
  cost: CostConfig;
}
