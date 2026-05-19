import { useMemo } from "react";
import ReactECharts from "echarts-for-react";
import { RequestRecord, Metric, TimeRange } from "../types";
import { ThemeTokens, colorPool, getLineStyle, getModelDisplayName } from "../theme";

interface Props {
  requests: RequestRecord[];
  metric: Metric;
  timeRange: TimeRange;
  selectedModels: string[];
  models: string[];
  theme: ThemeTokens;
  isDark: boolean;
  aliases: Record<string, string>;
}

export function Chart({ requests, metric, timeRange, selectedModels, models, theme, isDark, aliases }: Props) {
  const colors = isDark ? colorPool.dark : colorPool.light;

  const option = useMemo(() => {
    const modelGroups = groupByModel(requests, selectedModels);
    const series = Object.entries(modelGroups).map(([model, records]) => {
      const colorIndex = models.indexOf(model);
      const color = colors[colorIndex % colors.length] || theme.muted;
      const lineType = getLineStyle(colorIndex >= 0 ? colorIndex : 0);
      return {
        name: getModelDisplayName(model, aliases),
        type: "line" as const,
        smooth: true,
        symbol: "none",
        lineStyle: {
          width: 2,
          type: lineType,
          color,
        },
        itemStyle: { color },
        data: records.map((r) => [r.timestamp, getValue(r, metric)]),
      };
    });

    return {
      animation: false,
      grid: { top: 12, right: 12, bottom: 28, left: 40 },
      xAxis: {
        type: "time",
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          fontSize: 9,
          color: theme.tabInactiveText,
          fontFamily: "'Fira Code', monospace",
          formatter: (value: number) => {
            const d = new Date(value);
            return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}`;
          },
        },
        splitLine: { show: false },
      },
      yAxis: {
        type: "value",
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          fontSize: 9,
          color: theme.tabInactiveText,
          fontFamily: "'Fira Code', monospace",
          formatter: (value: number) => {
            if (metric === "ttft") return `${(value / 1000).toFixed(1)}s`;
            if (value >= 1000) return `${(value / 1000).toFixed(1)}k`;
            return value.toString();
          },
        },
        splitLine: { lineStyle: { color: theme.gridLine, type: "solid" } },
      },
      tooltip: {
        trigger: "axis",
        backgroundColor: isDark ? "#272F42" : "#FFFFFF",
        borderColor: theme.border,
        textStyle: {
          color: theme.foreground,
          fontSize: 10,
          fontFamily: "'Fira Code', monospace",
        },
        axisPointer: {
          type: "line",
          lineStyle: { color: theme.border, type: "dashed" },
        },
        formatter: (params: any[]) => {
          if (!params.length) return "";
          const time = new Date(params[0].value[0]);
          const timeStr = `${time.getHours().toString().padStart(2, "0")}:${time.getMinutes().toString().padStart(2, "0")}`;
          let html = `<div style="margin-bottom:4px;color:${theme.muted}">${timeStr}</div>`;
          for (const p of params) {
            const val = metric === "ttft"
              ? `${(p.value[1] / 1000).toFixed(1)}s`
              : p.value[1].toLocaleString();
            html += `<div style="display:flex;align-items:center;gap:4px;margin-bottom:2px">`;
            html += `<span style="width:6px;height:6px;border-radius:50%;background:${p.color};display:inline-block"></span>`;
            html += `<span>${p.seriesName}: ${val}</span></div>`;
          }
          return html;
        },
      },
      series,
    };
  }, [requests, metric, selectedModels, models, theme, isDark, aliases]);

  return (
    <div style={{ flex: 1, background: theme.card, borderRadius: 8, padding: 8, overflow: "hidden" }}>
      <ReactECharts
        option={option}
        style={{ height: "100%", width: "100%" }}
        opts={{ renderer: "canvas" }}
        notMerge
      />
    </div>
  );
}

function groupByModel(requests: RequestRecord[], selectedModels: string[]): Record<string, RequestRecord[]> {
  const groups: Record<string, RequestRecord[]> = {};
  for (const r of requests) {
    if (selectedModels.length > 0 && !selectedModels.includes(r.model)) continue;
    if (!groups[r.model]) groups[r.model] = [];
    groups[r.model].push(r);
  }
  return groups;
}

function getValue(record: RequestRecord, metric: Metric): number {
  switch (metric) {
    case "out_rate": return record.output_tokens;
    case "in_rate": return record.input_tokens;
    case "ttft": return record.duration_ms ?? 0;
  }
}
