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
    const bucketMs = getBucketSize(timeRange);

    const series = Object.entries(modelGroups).map(([model, records]) => {
      const colorIndex = models.indexOf(model);
      const color = colors[colorIndex % colors.length] || theme.muted;
      const lineType = getLineStyle(colorIndex >= 0 ? colorIndex : 0);

      const rawData = records
        .map((r) => [r.timestamp, getValue(r, metric)] as [string, number | null])
        .filter((d): d is [string, number] => d[1] !== null);

      const chartData = bucketMs ? aggregateIntoBuckets(rawData, bucketMs) : rawData;

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
        data: chartData,
      };
    });

    const xAxisMin = getXAxisMin(timeRange);
    const xAxisMax = getXAxisMax(timeRange);

    return {
      animation: false,
      grid: { top: 12, right: 12, bottom: 28, left: 40 },
      xAxis: {
        type: "time",
        min: xAxisMin,
        max: xAxisMax,
        axisLine: { show: false },
        axisTick: { show: false },
        splitNumber: timeRange === "1h" ? 4 : 4,
        axisLabel: {
          fontSize: 9,
          color: theme.tabInactiveText,
          fontFamily: "'Fira Code', monospace",
          formatter: (value: number) => {
            const d = new Date(value);
            if (timeRange === "1h") {
              return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}`;
            }
            return `${d.getHours()}:00`;
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
              : `${p.value[1].toLocaleString()} tok/s`;
            html += `<div style="display:flex;align-items:center;gap:4px;margin-bottom:2px">`;
            html += `<span style="width:6px;height:6px;border-radius:50%;background:${p.color};display:inline-block"></span>`;
            html += `<span>${p.seriesName}: ${val}</span></div>`;
          }
          return html;
        },
      },
      series,
    };
  }, [requests, metric, timeRange, selectedModels, models, theme, isDark, aliases]);

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

function getValue(record: RequestRecord, metric: Metric): number | null {
  const durationS = (record.duration_ms && record.duration_ms > 0) ? record.duration_ms / 1000 : null;
  switch (metric) {
    case "out_rate":
      if (!durationS) return null;
      return Math.round(record.output_tokens / durationS);
    case "in_rate":
      if (!durationS) return null;
      return Math.round(record.input_tokens / durationS);
    case "ttft":
      return durationS ? record.duration_ms : null;
  }
}

function getXAxisMin(timeRange: TimeRange): string {
  const now = new Date();
  switch (timeRange) {
    case "1h":
      return new Date(now.getTime() - 60 * 60 * 1000).toISOString();
    case "today":
      return new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString();
    case "yesterday":
      return new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1).toISOString();
  }
}

function getBucketSize(timeRange: TimeRange): number | null {
  switch (timeRange) {
    case "1h": return 5 * 60 * 1000;
    case "today": return 30 * 60 * 1000;
    case "yesterday": return 60 * 60 * 1000;
  }
}

function getXAxisMax(timeRange: TimeRange): string {
  const now = new Date();
  switch (timeRange) {
    case "1h":
      return now.toISOString();
    case "today":
      return now.toISOString();
    case "yesterday":
      return new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString();
  }
}

function aggregateIntoBuckets(data: [string, number][], bucketMs: number): [string, number][] {
  if (data.length === 0) return [];

  const buckets = new Map<number, number[]>();

  for (const [ts, val] of data) {
    const time = new Date(ts).getTime();
    const bucketKey = Math.floor(time / bucketMs) * bucketMs;
    if (!buckets.has(bucketKey)) buckets.set(bucketKey, []);
    buckets.get(bucketKey)!.push(val);
  }

  const result: [string, number][] = [];
  const sortedKeys = [...buckets.keys()].sort((a, b) => a - b);
  for (const key of sortedKeys) {
    const values = buckets.get(key)!;
    const avg = Math.round(values.reduce((a, b) => a + b, 0) / values.length);
    result.push([new Date(key + bucketMs / 2).toISOString(), avg]);
  }
  return result;
}
