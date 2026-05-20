# CC Monitor

[English](README.md)

在 macOS 菜单栏实时显示 AI 编程助手性能指标。

CC Monitor 监听 Claude Code 和 Codex CLI 的 JSONL 会话日志，解析每次 API 请求，将实时吞吐量（tok/s、TTFT、花费）直接显示在 macOS 状态栏。点击可弹出按模型分类的历史数据图表。

![主界面](assets/mockup-main.svg)

## 功能

**状态栏实时指标**

一眼掌握当前性能 — 输出速率、输入速率、首 token 响应时间、累计花费，每次请求完成后实时更新。

![菜单栏](assets/mockup-menubar.svg)

**成本追踪**

追踪所有 AI 编程会话的花费。可配置时间窗口（N 天/月/年），按项目白名单过滤，自动从 LiteLLM、models.dev、BaseLLM 同步价格。用户手动设置的价格不会被同步覆盖。

**多源监控**

同时监控 Claude Code（`~/.claude/projects/`）和 Codex CLI（`~/.codex/sessions/`）会话日志，可独立开关。

**多模型追踪**

独立追踪所有模型（Opus、Sonnet、Haiku、GPT-4o、Gemini、DeepSeek 等），图表中以不同颜色区分。支持在侧边栏按模型筛选，或为状态栏设置模型白名单。

**交互式图表**

- 时间范围：1 小时、今天、昨天
- 指标：输出 tok/s、输入 tok/s、TTFT、花费
- 数据聚合：5 分钟桶（1h）、30 分钟桶（今天）、1 小时桶（昨天）
- 平滑曲线 + 按模型着色
- 悬停显示精确数值

**可配置设置**

![设置面板](assets/mockup-settings.svg)

- **主题**：跟随系统 / 深色 / 浅色
- **显示模式**：最近一次请求 或 滚动平均值（可配置时间窗口）
- **模型过滤**：显示全部 或 白名单指定模型
- **状态栏项目**：选择并排序显示哪些指标（out_rate、in_rate、ttft、cost）
- **模型别名**：缩短模型名（如 `claude-opus-4-7` → `opus`）
- **成本时间窗口**：可配置 N 天/月/年/全部
- **价格同步**：LiteLLM / models.dev / BaseLLM / All，支持手动覆盖
- **监控源**：Claude Code / Codex CLI 独立开关
- **项目白名单**：仅追踪指定项目的花费（支持正常路径如 `/Users/name/project`）
- **数据管理**：按模型删除或清空全部历史

## 安装

从 [Releases](../../releases/latest) 下载 `.dmg`，拖入 Applications 即可。

CC Monitor 以菜单栏应用运行（不显示 Dock 图标）。点击状态栏文字切换图表弹窗。

## 工作原理

CC Monitor 每 500ms 轮询会话日志目录。检测到新的 assistant 响应时，计算每轮 token 增量并得出：

- **输出速率**：`output_tokens / duration`
- **输入速率**：`input_tokens / duration`
- **TTFT**：从用户消息到 assistant 响应的时间差
- **花费**：`tokens × 单价/M / 1,000,000`（在时间窗口内累加）

所有数据存储在本地 SQLite（`~/Library/Application Support/cc-monitor/data.db`）。

## 技术栈

- **后端**：Rust + Tauri 2
- **前端**：React 19 + TypeScript + Tailwind CSS 4
- **图表**：ECharts 6
- **存储**：SQLite (rusqlite, WAL mode)
- **平台**：macOS (Apple Silicon)

## 从源码构建

```bash
# 前置条件：Rust、Node.js、pnpm

# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 构建 DMG
pnpm tauri build
```

## 许可证

[MIT](LICENSE)
