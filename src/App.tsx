import { useState, useEffect } from "react";
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
    switch (metric) {
      case "out_rate": return latest.output_tokens.toLocaleString();
      case "in_rate": return latest.input_tokens.toLocaleString();
      case "ttft": return latest.duration_ms ? `${(latest.duration_ms / 1000).toFixed(1)}s` : "—";
    }
  };

  const metricUnit = metric === "ttft" ? "sec" : "tok/req";

  return (
    <div style={{ background: theme.bg, color: theme.foreground, fontFamily: "'Fira Sans', system-ui, sans-serif", padding: 20, height: "100vh", overflow: "hidden" }}>
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
