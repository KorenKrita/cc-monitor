import { ThemeTokens, colorPool, getModelDisplayName } from "../theme";

interface Props {
  models: string[];
  selected: string[];
  onChange: (models: string[]) => void;
  latestValue: string;
  metricUnit: string;
  theme: ThemeTokens;
  isDark: boolean;
  aliases: Record<string, string>;
}

export function ModelFilter({ models, selected, onChange, latestValue, metricUnit, theme, isDark, aliases }: Props) {
  const colors = isDark ? colorPool.dark : colorPool.light;
  const allSelected = selected.length === 0;

  const toggle = (model: string) => {
    if (allSelected) {
      onChange([model]);
    } else if (selected.includes(model)) {
      const next = selected.filter((m) => m !== model);
      onChange(next);
    } else {
      onChange([...selected, model]);
    }
  };

  const selectAll = () => onChange([]);

  const isActive = (model: string) => allSelected || selected.includes(model);

  return (
    <div style={{ width: 100, paddingRight: 14, borderRight: `1px solid ${theme.border}`, display: "flex", flexDirection: "column" }}>
      <div style={{ fontSize: 9, color: theme.tabInactiveText, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 10 }}>
        Models
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {models.length > 1 && (
          <label
            onClick={selectAll}
            style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}
          >
            <div style={{
              width: 10, height: 10, borderRadius: 2,
              background: allSelected ? theme.muted : "transparent",
              border: allSelected ? "none" : `1.5px solid ${theme.border}`,
            }} />
            <span style={{
              fontSize: 10,
              color: allSelected ? theme.foreground : theme.muted,
            }}>All</span>
          </label>
        )}
        {models.map((model, index) => {
          const color = colors[index % colors.length];
          const active = isActive(model);
          return (
            <label
              key={model}
              onClick={() => toggle(model)}
              style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}
            >
              <div style={{
                width: 10, height: 10, borderRadius: 2,
                background: active ? color : "transparent",
                border: active ? "none" : `1.5px solid ${theme.border}`,
              }} />
              <span style={{
                fontSize: 10,
                color: active ? theme.foreground : theme.muted,
              }}>
                {getModelDisplayName(model, aliases)}
              </span>
            </label>
          );
        })}
      </div>

      <div style={{ marginTop: "auto", paddingTop: 10, borderTop: `1px solid ${theme.border}` }}>
        <div style={{ fontSize: 8, color: theme.tabInactiveText, textTransform: "uppercase" }}>Latest</div>
        <div style={{ fontSize: 16, fontFamily: "'Fira Code', monospace", color: theme.accentGreen, fontWeight: 600, marginTop: 2 }}>
          {latestValue}
        </div>
        <div style={{ fontSize: 8, color: theme.tabInactiveText }}>{metricUnit}</div>
      </div>
    </div>
  );
}
