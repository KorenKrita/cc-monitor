import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ThemeTokens } from "../theme";

interface Props {
  models: string[];
  theme: ThemeTokens;
}

export function DataManagement({ models, theme }: Props) {
  const [selectedModel, setSelectedModel] = useState("");
  const [confirmAll, setConfirmAll] = useState(false);

  const deleteByModel = async () => {
    if (!selectedModel) return;
    if (!window.confirm(`Delete all data for "${selectedModel}"?`)) return;
    await invoke("delete_model_data", { model: selectedModel });
    setSelectedModel("");
  };

  const deleteAll = async () => {
    if (!confirmAll) {
      setConfirmAll(true);
      return;
    }
    await invoke("delete_all_data");
    setConfirmAll(false);
  };

  const selectStyle = {
    background: theme.card,
    border: `1px solid ${theme.border}`,
    borderRadius: 4,
    padding: "4px 8px",
    fontSize: 11,
    color: theme.foreground,
    outline: "none",
    flex: 1,
  };

  const dangerBtn = {
    background: "#EF4444",
    border: "none",
    borderRadius: 4,
    color: "#fff",
    fontSize: 10,
    padding: "4px 10px",
    cursor: "pointer" as const,
  };

  return (
    <div>
      <div style={{ fontSize: 9, color: theme.muted, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 6 }}>
        Data Management
      </div>

      <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 8 }}>
        <select value={selectedModel} onChange={(e) => setSelectedModel(e.target.value)} style={selectStyle}>
          <option value="">Select model...</option>
          {models.map((m) => <option key={m} value={m}>{m}</option>)}
        </select>
        <button onClick={deleteByModel} disabled={!selectedModel} style={{ ...dangerBtn, opacity: selectedModel ? 1 : 0.4 }}>
          Delete
        </button>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <button onClick={deleteAll} style={{ ...dangerBtn, background: confirmAll ? "#DC2626" : "#EF4444" }}>
          {confirmAll ? "Confirm Delete ALL Data" : "Delete All Data"}
        </button>
        {confirmAll && (
          <button
            onClick={() => setConfirmAll(false)}
            style={{ background: "transparent", border: `1px solid ${theme.border}`, borderRadius: 4, padding: "4px 10px", fontSize: 10, color: theme.muted, cursor: "pointer" }}
          >
            Cancel
          </button>
        )}
      </div>
    </div>
  );
}