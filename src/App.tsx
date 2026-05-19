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
import { Settings } from "./components/Settings";

export default function App() {
  const { config, setConfig, resolvedTheme } = useSettings();
  const { requests, models, latest, fetchData } = useMonitorData();
  const [metric, setMetric] = useState<Metric>("out_rate");
  const [timeRange, setTimeRange] = useState<TimeRange>("1h");
  const [selectedModels, setSelectedModels] = useState<string[]>([]);
  const [showSettings, setShowSettings] = useState(false);

  const isDark = resolvedTheme() === "dark";
  const theme: ThemeTokens = isDark ? darkTheme : lightTheme;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "w") {
        e.preventDefault();
        invoke("hide_window");
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

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

  if (showSettings) {
    return (
      <div style={{ background: theme.bg, color: theme.foreground, fontFamily: "'Fira Sans', system-ui, sans-serif", height: "100vh", overflow: "hidden", position: "relative" }}>
        <Settings
          config={config}
          onSave={(c) => { setConfig(c); setShowSettings(false); }}
          onClose={() => setShowSettings(false)}
          theme={theme}
        />
      </div>
    );
  }

  return (
    <div style={{ background: theme.bg, color: theme.foreground, fontFamily: "'Fira Sans', system-ui, sans-serif", padding: 20, height: "100vh", overflow: "hidden", position: "relative" }}>
      {/* Close button - top left green dot with × */}
      <button
        onClick={() => invoke("hide_window")}
        style={{
          position: "absolute", top: 6, left: 6,
          width: 12, height: 12, borderRadius: "50%",
          background: "#22C55E", border: "none",
          cursor: "pointer", padding: 0,
          display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 9, color: "#064E3B", lineHeight: 0, fontWeight: 700,
          paddingBottom: 1,
        }}
        title="Close"
      >×</button>

      {/* Settings gear - bottom right corner */}
      <button
        onClick={() => setShowSettings(true)}
        style={{
          position: "absolute", bottom: 6, right: 6,
          width: 20, height: 20, borderRadius: 4,
          background: "transparent", border: "none",
          color: theme.muted, fontSize: 14, cursor: "pointer",
          display: "flex", alignItems: "center", justifyContent: "center",
        }}
        title="Settings"
      >⚙</button>

      {/* Quit button - bottom left corner */}
      <button
        onClick={() => invoke("quit_app")}
        style={{
          position: "absolute", bottom: 6, left: 6,
          width: 16, height: 16, borderRadius: "50%",
          background: "transparent", border: "none",
          color: "#EF4444", fontSize: 12, cursor: "pointer",
          display: "flex", alignItems: "center", justifyContent: "center",
        }}
        title="Quit"
      >⏻</button>

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
