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
- 多 session 并发时全部采集，统一按 model 聚合（不区分 session）

## 数据模型

```sql
CREATE TABLE requests (
  id INTEGER PRIMARY KEY,
  timestamp TEXT NOT NULL,
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
  "theme": "system",
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
┌─────────────────────────────────────────────┐
│ [Out] [In] [TTFT]     [1h] [Today] [Yest.] │
├────────┬────────────────────────────────────┤
│ Models │  ECharts 折线图（波动，非累计）    │
│ ────── │  Y轴: tok/req 或 seconds           │
│ □ All  │                                    │
│ ■ Opus │  ~~~~/\~~~/\~~~~  (紫)             │
│ ■ Son. │  ~~~/\~~/\~~~~~   (绿)             │
│ □ Hai. │  ~~--~--~--~~--   (琥珀,虚线)      │
│ ────── │                                    │
│ Latest │  Hover: 竖虚线 + tooltip           │
│  847   │  [14:00] [14:15] [14:30] [15:00]   │
│ tok/req│                                    │
└────────┴────────────────────────────────────┘
```

### 指标切换

顶部左侧 tab 切换当前图表展示的指标：
- **Out**: output tokens/request（默认）
- **In**: input tokens/request
- **TTFT**: 响应耗时（seconds）

### 时间范围

顶部右侧 tab：
- **1h**: 最近一小时，X 轴按分钟
- **Today**: 今天，X 轴按小时
- **Yesterday**: 昨天，X 轴按小时

### 模型筛选

- 左侧 checkbox 列表，每个模型带颜色标识点
- 不选时：所有模型各自一条折线，不同颜色叠加
- 选择时：只显示选中模型
- 左下角显示 Latest 值（最近一次请求的当前指标值）

### Tooltip

- 鼠标划过图表时显示竖虚线
- 各模型在该时间点的圆点标记
- 浮层显示：时间 + 各模型精确数值

### 主题

支持三种主题，配置项 `"theme": "system" | "dark" | "light"`，默认 `"system"`。

**Dark 主题色值：**
- Background: `#0E1223`
- Card/Chart bg: `#1A1E2F`
- Border/Grid: `#272F42`
- Muted text: `#94A3B8`
- Foreground: `#F8FAFC`

**Light 主题色值：**
- Background: `#FAFBFC`
- Card/Chart bg: `#FFFFFF`
- Border/Grid: `#E2E8F0`
- Muted text: `#94A3B8`
- Foreground: `#1E293B`

**模型颜色：**
| Model  | Dark       | Light      | 线型   |
|--------|-----------|-----------|--------|
| Opus   | `#6366f1` | `#4F46E5` | 实线   |
| Sonnet | `#22C55E` | `#16A34A` | 实线   |
| Haiku  | `#F59E0B` | `#D97706` | 虚线   |

### 字体

- 数字/数据: Fira Code (monospace)
- UI 标签: Fira Sans / system-ui

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
