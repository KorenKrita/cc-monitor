# CC Monitor

[中文文档](README_CN.md)

Real-time AI coding assistant performance metrics in your macOS menu bar.

CC Monitor watches Claude Code and Codex CLI session logs, parses every API request, and displays live throughput stats (tok/s, TTFT, cost) directly in the macOS status bar. Click to open a chart popover with historical data by model.

![Main Interface](assets/mockup-main.svg)

## Features

**Live Menu Bar Metrics**

Your current performance at a glance — output rate, input rate, time-to-first-token, and cumulative cost, updating in real-time as requests complete.

![Menu Bar](assets/mockup-menubar.svg)

**Cost Tracking**

Track spending across all your AI coding sessions. Configurable time windows (N days/months/years), per-project whitelists, and automatic price sync from LiteLLM, models.dev, and BaseLLM. User-defined prices are never overwritten by sync.

**Multi-Source Monitoring**

Monitors both Claude Code (`~/.claude/projects/`) and Codex CLI (`~/.codex/sessions/`) session logs. Toggle sources independently.

**Multi-Model Tracking**

Tracks all models separately (Opus, Sonnet, Haiku, GPT-4o, Gemini, DeepSeek, etc.) with color-coded chart lines. Filter by model in the sidebar or whitelist specific models for the status bar.

**Interactive Chart**

- Time ranges: 1 hour, today, yesterday
- Metrics: output tok/s, input tok/s, TTFT, cost
- Data aggregation: 5-min buckets (1h), 30-min (today), 1-hour (yesterday)
- Smooth lines with per-model color coding
- Tooltip with exact values on hover

**Configurable Settings**

![Settings](assets/mockup-settings.svg)

- **Theme**: System / Dark / Light
- **Display mode**: Last request or rolling average (configurable window)
- **Model filter**: Show all or whitelist specific models
- **Status bar items**: Choose and reorder which metrics appear (out_rate, in_rate, ttft, cost)
- **Model aliases**: Shorten long model names (e.g., `claude-opus-4-7` → `opus`)
- **Cost time window**: Configurable N days/months/years/all time
- **Price sync**: LiteLLM / models.dev / BaseLLM / All, with manual override
- **Watch sources**: Claude Code / Codex CLI toggle
- **Project whitelist**: Track cost for specific projects only
- **Data management**: Delete by model or clear all history

## Install

Download the `.dmg` from [Releases](../../releases/latest), drag to Applications, and launch.

CC Monitor runs as a menu bar app (no Dock icon). Click the metrics text in the menu bar to toggle the chart popover.

## How It Works

CC Monitor polls session log directories every 500ms. When a new assistant response is detected, it computes per-turn token deltas and calculates:

- **Output rate**: `output_tokens / duration`
- **Input rate**: `input_tokens / duration`
- **TTFT**: Time from user message to assistant response
- **Cost**: `tokens × price_per_M / 1,000,000` (summed over time window)

All data is stored locally in SQLite (`~/Library/Application Support/cc-monitor/data.db`).

## Tech Stack

- **Backend**: Rust + Tauri 2
- **Frontend**: React 19 + TypeScript + Tailwind CSS 4
- **Charts**: ECharts 6
- **Storage**: SQLite (rusqlite, WAL mode)
- **Platform**: macOS (Apple Silicon)

## Build from Source

```bash
# Prerequisites: Rust, Node.js, pnpm

# Install dependencies
pnpm install

# Development
pnpm tauri dev

# Build DMG
pnpm tauri build
```

## License

[MIT](LICENSE)
