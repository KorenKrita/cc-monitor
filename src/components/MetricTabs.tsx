import { Metric } from "../types";
import { ThemeTokens } from "../theme";

interface Props {
  value: Metric;
  onChange: (m: Metric) => void;
  theme: ThemeTokens;
}

const tabs: { key: Metric; label: string }[] = [
  { key: "out_rate", label: "Out" },
  { key: "in_rate", label: "In" },
  { key: "ttft", label: "TTFT" },
];

export function MetricTabs({ value, onChange, theme }: Props) {
  return (
    <div style={{ display: "flex", gap: 2, background: theme.mutedBg, borderRadius: 8, padding: 3 }}>
      {tabs.map((tab) => (
        <button
          key={tab.key}
          onClick={() => onChange(tab.key)}
          style={{
            padding: "4px 10px",
            borderRadius: 6,
            fontSize: 11,
            fontWeight: value === tab.key ? 500 : 400,
            background: value === tab.key ? theme.tabActiveBg : "transparent",
            color: value === tab.key ? theme.accentGreen : theme.tabInactiveText,
            border: "none",
            cursor: "pointer",
            boxShadow: value === tab.key ? "0 1px 2px rgba(0,0,0,0.06)" : "none",
          }}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
