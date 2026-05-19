import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettings } from "./hooks/useSettings";
import { useMonitorData } from "./hooks/useMonitorData";
import { darkTheme, lightTheme, ThemeTokens } from "./theme";
import { Metric, TimeRange } from "./types";
import { Chart } from "./components/Chart";
import { ModelFilter } from "./components/ModelFilter";
import { MetricTabs } from "./components/MetricTabs";
import { TimeRangeTabs } from "./components/TimeRangeTabs";

export default function App() {
  const { config, resolvedTheme } = useSettings();
  const { requests, models, latest, fetchData } = useMonitorData();
  const [metric, setMetric] = useState<Metric>("out_rate");
  const [timeRange, setTimeRange] = useState<TimeRange>("1h");
  const [selectedModels, setSelectedModels] = useState<string[]>([]);

  const theme: ThemeTokens = resolvedTheme() === "dark" ? darkTheme : lightTheme;
  const isDark = resolvedTheme() === "dark";

  useEffect(() => {
    fetchData(timeRange, selectedModels.length > 0 ? selectedModels : undefined);
  }, [timeRange, selectedModels, fetchData]);

  const latestValue = (): string => {
    if (!latest) return "—";
    const durationS = (latest.duration_ms && latest.duration_ms > 0) ? latest.duration_ms / 1000 : null;
    switch (metric) {
      case "out_rate": return durationS ? `${Math.round(latest.output_tokens / durationS)}` : "—";
      case "in_rate": return durationS ? `${Math.round(latest.input_tokens / durationS)}` : "—";
      case "ttft": return durationS ? `${durationS.toFixed(1)}s` : "—";
    }
  };

  const metricUnit = metric === "ttft" ? "sec" : "tok/s";

  return (
    <div style={{ background: theme.bg, color: theme.foreground, fontFamily: "'Fira Sans', system-ui, sans-serif", padding: 20, height: "100vh", overflow: "hidden", position: "relative" }}>
      {/* Close button - top left red dot */}
      <button
        onClick={() => invoke("hide_window")}
        style={{
          position: "absolute", top: 10, left: 10,
          width: 12, height: 12, borderRadius: "50%",
          background: "#EF4444", border: "none",
          cursor: "pointer", padding: 0,
          display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 8, color: "transparent", lineHeight: 1,
        }}
        onMouseEnter={(e) => { e.currentTarget.style.color = "#7F1D1D"; }}
        onMouseLeave={(e) => { e.currentTarget.style.color = "transparent"; }}
        title="Close"
      >×</button>

      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 14 }}>
        <MetricTabs value={metric} onChange={setMetric} theme={theme} />
        <TimeRangeTabs value={timeRange} onChange={setTimeRange} theme={theme} />
      </div>

      <div style={{ display: "flex", gap: 14, height: "calc(100% - 50px)" }}>
        <ModelFilter
          models={models}
          selected={selectedModels}
          onChange={setSelectedModels}
          latestValue={latestValue()}
          metricUnit={metricUnit}
          theme={theme}
          isDark={isDark}
          aliases={config.model_aliases}
        />
        <Chart
          requests={requests}
          metric={metric}
          timeRange={timeRange}
          selectedModels={selectedModels}
          models={models}
          theme={theme}
          isDark={isDark}
          aliases={config.model_aliases}
        />
      </div>
    </div>
  );
}
