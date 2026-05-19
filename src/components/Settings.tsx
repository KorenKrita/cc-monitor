import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Config, Theme, Metric } from "../types";
import { ThemeTokens } from "../theme";

interface Props {
  config: Config;
  onSave: (config: Config) => void;
  onClose: () => void;
  theme: ThemeTokens;
}

export function Settings({ config, onSave, onClose, theme }: Props) {
  const [draft, setDraft] = useState<Config>(structuredClone(config));

  const save = async () => {
    await invoke("set_config", { config: draft });
    onSave(draft);
  };

  const updateTheme = (value: string) => {
    setDraft({ ...draft, theme: value as Theme });
  };

  const updateModelFilter = (value: string) => {
    setDraft({ ...draft, tray: { ...draft.tray, model_filter: value as Config["tray"]["model_filter"] } });
  };

  const updateTrayItems = (items: Metric[]) => {
    setDraft({ ...draft, tray: { ...draft.tray, items } });
  };

  const toggleTrayItem = (item: Metric) => {
    const items = draft.tray.items.includes(item)
      ? draft.tray.items.filter((i) => i !== item)
      : [...draft.tray.items, item];
    updateTrayItems(items);
  };

  const moveItem = (idx: number, dir: -1 | 1) => {
    const items = [...draft.tray.items];
    const target = idx + dir;
    if (target < 0 || target >= items.length) return;
    [items[idx], items[target]] = [items[target], items[idx]];
    updateTrayItems(items);
  };

  const updateAlias = (key: string, newKey: string, value: string) => {
    const aliases = { ...draft.model_aliases };
    if (newKey !== key) delete aliases[key];
    aliases[newKey] = value;
    setDraft({ ...draft, model_aliases: aliases });
  };

  const removeAlias = (key: string) => {
    const aliases = { ...draft.model_aliases };
    delete aliases[key];
    setDraft({ ...draft, model_aliases: aliases });
  };

  const addAlias = () => {
    setDraft({ ...draft, model_aliases: { ...draft.model_aliases, "": "" } });
  };

  const inputStyle = {
    background: theme.card,
    border: `1px solid ${theme.border}`,
    borderRadius: 4,
    padding: "4px 8px",
    fontSize: 11,
    color: theme.foreground,
    outline: "none",
    width: "100%",
  };

  const selectStyle = {
    ...inputStyle,
    cursor: "pointer",
    appearance: "none" as const,
    backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath fill='%2394A3B8' d='M0 2l4 4 4-4z'/%3E%3C/svg%3E")`,
    backgroundRepeat: "no-repeat",
    backgroundPosition: "right 8px center",
    paddingRight: 24,
  };

  const labelStyle = { fontSize: 9, color: theme.muted, textTransform: "uppercase" as const, letterSpacing: 0.5, marginBottom: 4 };

  return (
    <div style={{
      position: "absolute", inset: 0, background: theme.bg, zIndex: 100,
      display: "flex", flexDirection: "column", padding: 20, overflow: "hidden",
    }}>
      <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 16 }}>Settings</div>

      <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 14 }}>
        {/* Theme */}
        <div>
          <div style={labelStyle}>Theme</div>
          <select value={draft.theme} onChange={(e) => updateTheme(e.target.value)} style={selectStyle}>
            <option value="system">System</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </div>

        {/* Model Filter */}
        <div>
          <div style={labelStyle}>Status Bar Model Filter</div>
          <select value={draft.tray.model_filter} onChange={(e) => updateModelFilter(e.target.value)} style={selectStyle}>
            <option value="last">Last Used</option>
            <option value="all">All Models</option>
            <option value="whitelist">Whitelist</option>
          </select>
          {draft.tray.model_filter === "whitelist" && (
            <div style={{ marginTop: 6 }}>
              <textarea
                value={draft.tray.model_whitelist.join(", ")}
                onChange={(e) => {
                  const list = e.target.value.split(",").map((s) => s.trim()).filter(Boolean);
                  setDraft({ ...draft, tray: { ...draft.tray, model_whitelist: list } });
                }}
                placeholder="claude-opus-4-7, claude-sonnet-4-6"
                style={{
                  ...inputStyle,
                  height: 40,
                  resize: "vertical" as const,
                  fontFamily: "'Fira Code', monospace",
                }}
              />
              <div style={{ fontSize: 9, color: theme.muted, marginTop: 3 }}>
                Comma-separated model names, e.g. claude-opus-4-7, claude-sonnet-4-6
              </div>
            </div>
          )}
        </div>

        {/* Tray Items (checkboxes + reorder) */}
        <div>
          <div style={labelStyle}>Status Bar Items (drag to reorder)</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {draft.tray.items.map((item, idx) => (
              <div key={item} style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <input
                  type="checkbox"
                  checked={true}
                  onChange={() => toggleTrayItem(item)}
                  style={{ accentColor: theme.accentGreen }}
                />
                <span style={{ fontSize: 11, flex: 1 }}>
                  {item === "out_rate" ? "↓ Out Rate" : item === "in_rate" ? "↑ In Rate" : "⏱ TTFT"}
                </span>
                <button
                  onClick={() => moveItem(idx, -1)}
                  disabled={idx === 0}
                  style={{ background: "transparent", border: "none", color: idx === 0 ? theme.border : theme.muted, fontSize: 12, cursor: idx === 0 ? "default" : "pointer", padding: "0 2px" }}
                >▲</button>
                <button
                  onClick={() => moveItem(idx, 1)}
                  disabled={idx === draft.tray.items.length - 1}
                  style={{ background: "transparent", border: "none", color: idx === draft.tray.items.length - 1 ? theme.border : theme.muted, fontSize: 12, cursor: idx === draft.tray.items.length - 1 ? "default" : "pointer", padding: "0 2px" }}
                >▼</button>
              </div>
            ))}
            {/* Show unchecked items */}
            {(["out_rate", "in_rate", "ttft"] as Metric[]).filter((m) => !draft.tray.items.includes(m)).map((item) => (
              <div key={item} style={{ display: "flex", alignItems: "center", gap: 6, opacity: 0.5 }}>
                <input
                  type="checkbox"
                  checked={false}
                  onChange={() => toggleTrayItem(item)}
                  style={{ accentColor: theme.accentGreen }}
                />
                <span style={{ fontSize: 11 }}>
                  {item === "out_rate" ? "↓ Out Rate" : item === "in_rate" ? "↑ In Rate" : "⏱ TTFT"}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Model Aliases */}
        <div>
          <div style={{ ...labelStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>Model Aliases</span>
            <button
              onClick={addAlias}
              style={{
                background: theme.accentGreen, border: "none", borderRadius: 4,
                color: "#fff", fontSize: 11, padding: "2px 8px", cursor: "pointer",
              }}
            >+</button>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 6 }}>
            {Object.entries(draft.model_aliases).map(([key, value], idx) => (
              <div key={idx} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <input
                  value={key}
                  onChange={(e) => updateAlias(key, e.target.value, value)}
                  placeholder="model name"
                  style={{ ...inputStyle, flex: 1 }}
                />
                <span style={{ color: theme.muted, fontSize: 11 }}>→</span>
                <input
                  value={value}
                  onChange={(e) => updateAlias(key, key, e.target.value)}
                  placeholder="alias"
                  style={{ ...inputStyle, width: 60 }}
                />
                <button
                  onClick={() => removeAlias(key)}
                  style={{
                    background: "transparent", border: "none",
                    color: "#EF4444", fontSize: 14, cursor: "pointer", padding: "0 4px",
                  }}
                >×</button>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Footer buttons */}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 14, paddingTop: 10, borderTop: `1px solid ${theme.border}` }}>
        <button
          onClick={onClose}
          style={{
            background: "transparent", border: `1px solid ${theme.border}`,
            borderRadius: 6, padding: "5px 14px", fontSize: 11,
            color: theme.muted, cursor: "pointer",
          }}
        >Cancel</button>
        <button
          onClick={save}
          style={{
            background: theme.accentGreen, border: "none",
            borderRadius: 6, padding: "5px 14px", fontSize: 11,
            color: "#fff", cursor: "pointer",
          }}
        >Save</button>
      </div>
    </div>
  );
}
