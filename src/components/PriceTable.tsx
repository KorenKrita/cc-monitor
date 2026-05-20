import { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Config, ModelPrice, SyncSource } from "../types";
import { ThemeTokens } from "../theme";

interface Props {
  config: Config;
  models: string[];
  onUpdate: (prices: Record<string, ModelPrice>) => void;
  onSyncComplete: (config: Config) => void;
  onSyncSourceChange: (source: SyncSource) => void;
  theme: ThemeTokens;
}

export function PriceTable({ config, models, onUpdate, onSyncComplete, onSyncSourceChange, theme }: Props) {
  const [syncing, setSyncing] = useState(false);
  const [newModel, setNewModel] = useState("");

  const prices = config.cost.model_prices;
  const syncSource = config.cost.sync_source;

  const updatePrice = (model: string, field: keyof ModelPrice, value: number) => {
    const current = prices[model] || { input: 0, output: 0, cache: 0, source: "manual" };
    const updated = { ...prices, [model]: { ...current, [field]: value, source: "manual" } };
    onUpdate(updated);
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      const newConfig = await invoke<Config>("sync_prices", { source: syncSource });
      onSyncComplete(newConfig);
    } catch (e) {
      console.error("Sync failed:", e);
    } finally {
      setSyncing(false);
    }
  };

  const addModel = () => {
    const name = newModel.trim();
    if (!name) return;
    const existing = prices[name];
    if (existing && existing.source === "manual") return;
    const updated = { ...prices, [name]: { input: existing?.input ?? 0, output: existing?.output ?? 0, cache: existing?.cache ?? 0, source: "manual" } };
    onUpdate(updated);
    setNewModel("");
  };

  const removePrice = (model: string) => {
    const updated = { ...prices };
    delete updated[model];
    onUpdate(updated);
  };

  const displayModels = useMemo(() => [...new Set([
    ...models,
    ...Object.keys(prices).filter((m) => prices[m].source === "manual"),
  ])].sort(), [models, prices]);

  const cellStyle = {
    background: theme.card,
    border: `1px solid ${theme.border}`,
    borderRadius: 4,
    padding: "3px 6px",
    fontSize: 10,
    color: theme.foreground,
    outline: "none",
    width: 60,
    textAlign: "right" as const,
  };

  const selectStyle = {
    background: theme.card,
    border: `1px solid ${theme.border}`,
    borderRadius: 4,
    padding: "3px 6px",
    fontSize: 10,
    color: theme.foreground,
    outline: "none",
  };

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <span style={{ fontSize: 9, color: theme.muted, textTransform: "uppercase", letterSpacing: 0.5 }}>
          Model Prices ($/M tokens)
        </span>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {config.cost.last_sync_time && (
            <span style={{ fontSize: 9, color: theme.muted }}>
              Last: {new Date(config.cost.last_sync_time).toLocaleDateString()}
            </span>
          )}
          <select
            value={syncSource}
            onChange={(e) => onSyncSourceChange(e.target.value as SyncSource)}
            style={selectStyle}
          >
            <option value="litellm">LiteLLM</option>
            <option value="models.dev">models.dev</option>
            <option value="basellm">BaseLLM</option>
            <option value="all">All</option>
          </select>
          <button
            onClick={handleSync}
            disabled={syncing}
            style={{
              background: theme.accentGreen, border: "none", borderRadius: 4,
              color: "#fff", fontSize: 10, padding: "3px 8px", cursor: syncing ? "wait" : "pointer",
              opacity: syncing ? 0.6 : 1,
            }}
          >
            {syncing ? "Syncing..." : "Sync"}
          </button>
        </div>
      </div>

      <div style={{ maxHeight: 160, overflowY: "auto" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 10 }}>
          <thead>
            <tr style={{ color: theme.muted }}>
              <th style={{ textAlign: "left", padding: "2px 4px", fontWeight: 400 }}>Model</th>
              <th style={{ textAlign: "right", padding: "2px 4px", fontWeight: 400 }}>Input</th>
              <th style={{ textAlign: "right", padding: "2px 4px", fontWeight: 400 }}>Output</th>
              <th style={{ textAlign: "right", padding: "2px 4px", fontWeight: 400 }}>Cache</th>
              <th style={{ textAlign: "center", padding: "2px 4px", fontWeight: 400 }}>Src</th>
              <th style={{ width: 20 }}></th>
            </tr>
          </thead>
          <tbody>
            {displayModels.map((model) => {
              const p = prices[model] || { input: 0, output: 0, cache: 0, source: "" };
              const isManual = p.source === "manual";
              return (
                <tr key={model} style={{ borderTop: `1px solid ${theme.border}` }}>
                  <td style={{ padding: "3px 4px", fontSize: 10, color: isManual ? theme.accentGreen : theme.foreground }}>
                    {model}
                  </td>
                  <td style={{ padding: "2px" }}>
                    <input type="number" step="0.01" value={p.input ?? ""} onChange={(e) => updatePrice(model, "input", parseFloat(e.target.value) || 0)} style={cellStyle} />
                  </td>
                  <td style={{ padding: "2px" }}>
                    <input type="number" step="0.01" value={p.output ?? ""} onChange={(e) => updatePrice(model, "output", parseFloat(e.target.value) || 0)} style={cellStyle} />
                  </td>
                  <td style={{ padding: "2px" }}>
                    <input type="number" step="0.01" value={p.cache ?? ""} onChange={(e) => updatePrice(model, "cache", parseFloat(e.target.value) || 0)} style={cellStyle} />
                  </td>
                  <td style={{ textAlign: "center", fontSize: 9, color: theme.muted }}>{p.source || "—"}</td>
                  <td style={{ padding: "2px" }}>
                    {isManual && (
                      <button onClick={() => removePrice(model)} style={{ background: "transparent", border: "none", color: "#EF4444", fontSize: 12, cursor: "pointer", padding: 0 }}>×</button>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div style={{ display: "flex", gap: 4, marginTop: 6 }}>
        <input
          value={newModel}
          onChange={(e) => setNewModel(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addModel()}
          placeholder="Add model..."
          style={{ ...cellStyle, flex: 1, textAlign: "left", width: "auto" }}
        />
        <button
          onClick={addModel}
          disabled={!newModel.trim()}
          style={{
            background: theme.accentGreen, border: "none", borderRadius: 4,
            color: "#fff", fontSize: 10, padding: "3px 8px", cursor: "pointer",
            opacity: newModel.trim() ? 1 : 0.4,
          }}
        >+</button>
      </div>
    </div>
  );
}
