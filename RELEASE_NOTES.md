# Release Notes

## v0.0.2

### New Features

**Cost Tracking**
- Real-time cumulative cost displayed in the menu bar (`$ Cost` metric)
- Configurable time window: N days / months / years / all time
- Per-project cost filtering via whitelist
- 11 bundled model prices (Claude, GPT, Gemini, DeepSeek) out of the box

**Price Sync**
- Sync model prices from 3 sources: LiteLLM, models.dev, BaseLLM
- Dropdown to select sync source (single or all)
- User-created/edited prices (source: "manual") are never overwritten by sync
- Add custom model prices via input field

**Codex CLI Support**
- Monitor OpenAI Codex CLI sessions (`~/.codex/sessions/`)
- Toggle Claude Code / Codex CLI monitoring independently
- Unified project path encoding across both sources

**Data Management**
- Delete history by model (with confirmation)
- Delete all data (two-step confirmation)
- Model list refreshes after deletion

### Bug Fixes

- **Fixed cost calculation**: Parser now computes per-turn token deltas instead of storing cumulative session totals. Eliminates massive overcounting.
- **Fixed duplicate entries**: Repeated JSONL lines with identical token counts are now deduplicated (zero-delta skip).
- **Fixed chart X-axis**: Midnight now displays as "0:00" instead of "24:00".
- **Fixed delete button**: Replaced `window.confirm()` (broken in Tauri webview) with inline confirmation UI.
- **Fixed model list refresh**: Dropdown updates after deletion operations.
- **Fixed add model button**: No longer silently fails when model exists in synced (hidden) prices.

### Improvements

- Cost time window now accepts a numeric value (e.g., "3 days" instead of just "day")
- Project whitelist shows format example in placeholder
- Price table only shows models with actual usage data + manually added ones
- Sync source preference is persisted in config
- `CostConfig::cost_since()` method eliminates repeated calculation logic
- Removed unnecessary `to_string()` allocation in hot polling path

---

## v0.0.1

Initial release.

- Real-time output rate, input rate, TTFT in macOS menu bar
- Multi-model tracking with color-coded charts
- Interactive chart with 1h / today / yesterday time ranges
- Configurable display mode (last request / rolling average)
- Model aliases and whitelist filtering
- Dark / Light / System theme
- SQLite local storage with WAL mode
