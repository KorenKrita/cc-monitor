# CC Monitor — Claude Code Token 监控菜单栏 App

## 概述

macOS 菜单栏应用，实时监控 Claude Code 的 token 使用速率和 TTFT。支持多模型分开统计，可配置状态栏显示内容。

## 技术栈

- **框架**: Tauri (Rust backend + Web frontend)
- **前端**: React + TailwindCSS + ECharts
- **存储**: SQLite (rusqlite)，存储路径 `~/.config/cc-monitor/data.db`
- **配置**: JSON，路径 `~/.config/cc-monitor/settings.json`

## 数据源

Claude Code 会话日志：`~/.claude/projects/*/*.jsonl`

每条 assistant 类型消息包含：
```json
{
  "type": "assistant",
  "timestamp": "2026-05-15T06:23:30.210Z",
  "sessionId": "xxx",
  "message": {
    "model": "claude-opus-4-7",
    "usage": {
      "input_tokens": 3,
      "output_tokens": 61,
      "cache_creation_input_tokens": 44295,
      "cache_read_input_tokens": 0
    }
  }
}
```

## 数据采集

- 只统计 app 运行期间的数据，不回填历史
- 启动时 watch `~/.claude/projects/` 整个目录树
- 监听所有 `.jsonl` 文件变化，从文件末尾开始处理新增行
- 解析 `type: "assistant"` 条目，提取 model + usage
- TTFT/duration 计算：当前 assistant timestamp - 前一条 user timestamp（同 session 内）
- 新 session 文件出现时自动纳入监听
- 多 session 并发时全部采集，按 model 聚合

## 数据模型

```sql
CREATE TABLE requests (
  id INTEGER PRIMARY KEY,
  timestamp TEXT NOT NULL,
  session_id TEXT NOT NULL,
  model TEXT NOT NULL,
  input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL,
  cache_creation_tokens INTEGER DEFAULT 0,
  cache_read_tokens INTEGER DEFAULT 0,
  duration_ms INTEGER
);

CREATE INDEX idx_timestamp ON requests(timestamp);
CREATE INDEX idx_model ON requests(model);
```

## 状态栏

### 显示格式

示例：`⬡ ↑1.2k ↓45k 2.3s`

- `↑` = input tokens（本次请求）
- `↓` = output tokens（本次请求）
- `2.3s` = TTFT（最近一次请求耗时）

### 配置

```json
{
  "tray": {
    "items": ["out_rate", "in_rate", "ttft"],
    "model_filter": "last",
    "model_whitelist": ["claude-opus-4-7", "claude-sonnet-4-6"]
  }
}
```

- `items`: 显示哪些指标及顺序
- `model_filter`: `"last"` 最近一次请求的模型 / `"whitelist"` 只统计白名单 / `"all"` 所有模型聚合

## Popover 面板

### 布局（~480px 宽）

```
┌────────┬────────────────────────────────┐
│ Model  │  ECharts 折线图                │
│ Filter │  (多系列，不同颜色)            │
│ ────── │                                │
│ □ All  │                                │
│ ■ Opus │                                │
│ □ Son. │ ┌──────────────────────────┐   │
│ □ Hai. │ │ [1h]  [Today] [Yesterday]│   │
│        │ └──────────────────────────┘   │
└────────┴────────────────────────────────┘
```

### 交互

- **左侧**：模型 checkbox 列表，每个模型带颜色标识点
- **右侧**：ECharts 折线图 + 时间段 tab
- **不选模型时**：所有模型各自一条折线，不同颜色叠加
- **选择模型时**：只显示选中模型
- **时间段**：
  - 1h：最近一小时，X 轴按分钟
  - Today：今天，X 轴按小时
  - Yesterday：昨天，X 轴按小时
- **Tooltip**：悬停显示精确数值（model, tokens, time）

## 架构流程

```
JSONL files (fs watch)
       │
       ▼
Rust: parse new lines → extract assistant messages
       │
       ▼
SQLite: insert request record
       │
       ├──▶ Update tray text (latest request stats)
       │
       └──▶ Tauri event → Frontend
                              │
                              ▼
                    React: update ECharts
```

## 项目结构

```
cc-monitor/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Tauri 入口
│   │   ├── watcher.rs       # JSONL 文件监听
│   │   ├── parser.rs        # JSONL 行解析
│   │   ├── db.rs            # SQLite 操作
│   │   └── tray.rs          # 状态栏更新
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.tsx
│   ├── components/
│   │   ├── Chart.tsx         # ECharts 折线图
│   │   ├── ModelFilter.tsx   # 模型筛选侧边栏
│   │   └── TimeRangeTabs.tsx # 时间段切换
│   ├── hooks/
│   │   └── useMonitorData.ts # 监听 Tauri events
│   └── main.tsx
├── docs/
├── package.json
└── settings.json (example)
```

## 约束与边界

- 不采集历史数据，只监控 app 运行期间
- TTFT 精度受限于 JSONL 写入时机（消息完成后写入，非流式）
- 速率为 tokens/request 而非 tokens/second
- 仅支持 macOS（Tauri tray API）
