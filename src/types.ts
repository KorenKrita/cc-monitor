export interface RequestRecord {
  id: number;
  timestamp: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  duration_ms: number | null;
}

export type Metric = "out_rate" | "in_rate" | "ttft";
export type TimeRange = "1h" | "today" | "yesterday";
export type Theme = "system" | "dark" | "light";

export interface Config {
  theme: Theme;
  tray: {
    items: Metric[];
    model_filter: "last" | "whitelist" | "all";
    model_whitelist: string[];
  };
  model_aliases: Record<string, string>;
}
