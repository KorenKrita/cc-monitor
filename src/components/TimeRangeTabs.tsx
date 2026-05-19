import { TimeRange } from "../types";
import { ThemeTokens } from "../theme";

interface Props {
  value: TimeRange;
  onChange: (t: TimeRange) => void;
  theme: ThemeTokens;
}

const tabs: { key: TimeRange; label: string }[] = [
  { key: "1h", label: "1h" },
  { key: "today", label: "Today" },
  { key: "yesterday", label: "Yesterday" },
];

export function TimeRangeTabs({ value, onChange, theme }: Props) {
  return (
    <div style={{ display: "flex", gap: 2, background: theme.mutedBg, borderRadius: 8, padding: 3 }}>
      {tabs.map((tab) => (
        <button
          key={tab.key}
          onClick={() => onChange(tab.key)}
          style={{
            padding: "4px 8px",
            borderRadius: 6,
            fontSize: 11,
            background: value === tab.key ? theme.tabActiveBg : "transparent",
            color: value === tab.key ? theme.tabActiveText : theme.tabInactiveText,
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
