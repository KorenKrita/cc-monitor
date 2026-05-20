import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ThemeTokens } from "../theme";

interface Props {
  models: string[];
  theme: ThemeTokens;
  onRefresh: () => void;
}

type ConfirmStage = "idle" | "first" | "second";

const LABEL_MAP: Record<ConfirmStage, string> = {
  idle: "Delete All",
  first: "Confirm? (1/2)",
  second: "Confirm? (2/2)",
};

export function DataManagement({ models, theme, onRefresh }: Props) {
  const [selectedModel, setSelectedModel] = useState("");
  const [confirmModel, setConfirmModel] = useState(false);
  const [confirmStage, setConfirmStage] = useState<ConfirmStage>("idle");
  const [isDeleting, setIsDeleting] = useState(false);

  const deleteByModel = async () => {
    if (!selectedModel) return;
    if (!confirmModel) {
      setConfirmModel(true);
      return;
    }
    await invoke("delete_model_data", { model: selectedModel });
    setSelectedModel("");
    setConfirmModel(false);
    onRefresh();
  };

  const deleteAll = async () => {
    if (isDeleting) return;
    if (confirmStage === "idle") {
      setConfirmStage("first");
      return;
    }
    if (confirmStage === "first") {
      setConfirmStage("second");
      return;
    }
    setIsDeleting(true);
    try {
      await invoke("delete_all_data");
      onRefresh();
    } finally {
      setIsDeleting(false);
      setConfirmStage("idle");
    }
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
        Model History Data
      </div>

      <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 8 }}>
        <select
          value={selectedModel}
          onChange={(e) => { setSelectedModel(e.target.value); setConfirmModel(false); }}
          style={selectStyle}
        >
          <option value="">Select model...</option>
          {models.map((m) => <option key={m} value={m}>{m}</option>)}
        </select>
        <button onClick={deleteByModel} disabled={!selectedModel} style={{ ...dangerBtn, background: confirmModel ? "#DC2626" : "#EF4444", opacity: selectedModel ? 1 : 0.4 }}>
          {confirmModel ? "Confirm?" : "Delete"}
        </button>
        {confirmModel && (
          <button
            onClick={() => setConfirmModel(false)}
            style={{ background: "transparent", border: `1px solid ${theme.border}`, borderRadius: 4, padding: "4px 10px", fontSize: 10, color: theme.muted, cursor: "pointer" }}
          >
            Cancel
          </button>
        )}
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <button
          onClick={deleteAll}
          disabled={isDeleting}
          style={{ ...dangerBtn, background: confirmStage !== "idle" ? "#DC2626" : "#EF4444" }}
        >
          {LABEL_MAP[confirmStage]}
        </button>
        {confirmStage !== "idle" && (
          <button
            onClick={() => setConfirmStage("idle")}
            style={{ background: "transparent", border: `1px solid ${theme.border}`, borderRadius: 4, padding: "4px 10px", fontSize: 10, color: theme.muted, cursor: "pointer" }}
          >
            Cancel
          </button>
        )}
      </div>
    </div>
  );
}
