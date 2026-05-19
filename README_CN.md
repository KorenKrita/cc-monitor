# CC Monitor

[English](README.md)

在 macOS 菜单栏实时显示 Claude Code 性能指标。

CC Monitor 监听 Claude Code 的 JSONL 会话日志，解析每次 API 请求，将实时吞吐量（tok/s、TTFT）直接显示在 macOS 状态栏。点击可弹出按模型分类的历史数据图表。

![主界面](assets/mockup-main.svg)

## 功能

**状态栏实时指标**

一眼掌握当前 Claude Code 性能 — 输出速率、输入速率、首 token 响应时间，每次请求完成后实时更新。

![菜单栏](assets/mockup-menubar.svg)

**多模型追踪**

独立追踪所有 Claude 模型（Opus、Sonnet、Haiku），图表中以不同颜色区分。支持在侧边栏按模型筛选，或为状态栏设置模型白名单。

**交互式图表**

- 时间范围：1 小时、今天、昨天
- 指标：输出 tok/s、输入 tok/s、TTFT
- 数据聚合：5 分钟桶（1h）、30 分钟桶（今天）、1 小时桶（昨天）
- 平滑曲线 + 按模型着色
- 悬停显示精确数值

**可配置设置**

![设置面板](assets/mockup-settings.svg)

- **主题**：跟随系统 / 深色 / 浅色
- **显示模式**：最近一次请求 或 滚动平均值（可配置时间窗口）
- **模型过滤**：显示全部 或 白名单指定模型
- **状态栏项目**：选择并排序显示哪些指标（out_rate、in_rate、ttft）
- **模型别名**：缩短模型名（如 `claude-opus-4-7` → `opus`）

## 安装

从 [Releases](../../releases/latest) 下载 `.dmg`，拖入 Applications 即可。

CC Monitor 以菜单栏应用运行（不显示 Dock 图标）。点击状态栏文字切换图表弹窗。

> **注意**：当前为未签名的 debug 构建，首次启动需右键 → 打开。

## 工作原理

CC Monitor 每 500ms 轮询 `~/.claude/projects/` 下的 JSONL 会话日志。检测到新的 assistant 响应时，计算：

- **输出速率**：`output_tokens / duration`
- **输入速率**：`input_tokens / duration`
- **TTFT**：从用户消息到 assistant 响应的时间差

所有数据存储在本地 SQLite（`~/Library/Application Support/cc-monitor/data.db`）。

## 技术栈

- **后端**：Rust + Tauri 2
- **前端**：React 19 + TypeScript + Tailwind CSS 4
- **图表**：ECharts 6
- **存储**：SQLite (rusqlite)
- **平台**：macOS (Apple Silicon)

## 从源码构建

```bash
# 前置条件：Rust、Node.js

# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建 DMG
npm run tauri build
```

## 许可证

[MIT](LICENSE)
