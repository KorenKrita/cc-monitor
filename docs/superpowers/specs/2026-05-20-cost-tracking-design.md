# Cost Tracking Feature Design

## Overview

为 cc-monitor 添加实时成本追踪功能，同时扩展支持 Codex CLI session 监控。状态栏显示累计花费，设置中提供价格配置、数据源同步、白名单过滤和数据管理。

## 状态栏

- 新增 `cost` 指标项，显示格式：`$12.34`
- 与现有 `out_rate` / `in_rate` / `ttft` 一致的排序和开关机制
- 可配置统计时间窗口：day / month / year / all（永久）

## 数据模型

不新增数据库字段。现有 `requests` 表已存储 `input_tokens`、`output_tokens`、`cache_creation_tokens`、`cache_read_tokens`，费用在展示时根据当前价格表实时计算。

计算公式：
```
request_cost = (input_tokens * input_price + output_tokens * output_price + (cache_read_tokens + cache_creation_tokens) * cache_price) / 1_000_000
```

## 价格配置

### 数据结构

```rust
pub struct ModelPrice {
    pub input: f64,       // $/M tokens
    pub output: f64,      // $/M tokens
    pub cache: f64,       // $/M tokens
    pub source: String,   // "manual" | "litellm" | "new-api"
}
```

### 优先级

```
用户手动设置 (source: "manual") > 同步拉取 (source: "litellm" | "new-api") > 无价格（不计费）
```

### 同步源

1. litellm: `model_prices_and_context_window.json` from GitHub
2. new-api: 两个价格源（待确认具体 URL）

同步逻辑：
- 拉取远程价格数据，解析为 ModelPrice
- 只写入 source 非 "manual" 的条目（不覆盖手动配置）
- 设置中提供手动触发同步按钮

## 设置项新增

### Config 结构扩展

```rust
pub struct Config {
    pub theme: String,
    pub tray: TrayConfig,
    pub model_aliases: HashMap<String, String>,
    // 新增
    pub cost: CostConfig,
}

pub struct CostConfig {
    pub time_window: String,                    // "day" | "month" | "year" | "all"
    pub project_whitelist: Vec<String>,         // 参与费用统计的项目路径，空=全部
    pub model_whitelist: Vec<String>,           // 参与费用统计的模型，空=全部
    pub model_prices: HashMap<String, ModelPrice>,  // 模型价格表
}
```

### TrayConfig 扩展

```rust
pub struct TrayConfig {
    pub items: Vec<String>,  // 新增可选值 "cost"
    // ... 其余不变
}
```

## 设置 UI 新增

1. **Cost 区域**
   - 时间窗口选择：Day / Month / Year / All
   - 项目白名单：多行文本框，每行一个项目路径
   - 模型计费白名单：多行文本框，每行一个模型名

2. **模型价格表**
   - 列表展示所有已知模型
   - 每行：模型名 | input $/M | output $/M | cache $/M | source 标签
   - source 为 "manual" 的行高亮显示
   - 可编辑价格字段（编辑后 source 自动变为 "manual"）

3. **同步按钮**
   - "Sync Prices" 按钮，点击后从配置的数据源拉取
   - 显示上次同步时间
   - 同步不覆盖 source: "manual" 的条目

## 数据管理

1. **按模型删除**：删除指定模型的所有 `requests` 记录（性能指标 + 费用数据一起删，因为是同一张表）
2. **全部删除**：清空 `requests` 表所有数据

UI：设置页底部 "Data Management" 区域，下拉选模型 + Delete 按钮，以及 Delete All 按钮（带确认弹窗）。

## 状态栏费用计算逻辑

1. 根据 `cost.time_window` 确定查询时间范围
2. 查询该时间范围内的所有 requests
3. 按 `cost.project_whitelist` 过滤（需要从 watcher 记录请求来源的项目路径）
4. 按 `cost.model_whitelist` 过滤
5. 对每条 request，查找对应模型的价格，计算单条费用
6. 求和，格式化为 `$X.XX` 显示

## 项目路径追踪

当前 `requests` 表不存储项目信息。需要新增字段：

```sql
ALTER TABLE requests ADD COLUMN project TEXT DEFAULT '';
```

项目路径从 JSONL 文件路径解析：
- `~/.claude/projects/-Users-korenkrita-Coding-cc-monitor/session.jsonl`
- 提取 `-Users-korenkrita-Coding-cc-monitor` → `/Users/korenkrita/Coding/cc-monitor`

## Codex CLI 支持

### 概述

扩展 watcher 同时监控 Codex CLI 的 session logs，复用同一套数据存储和费用计算逻辑。

### 日志位置

- 活跃 sessions：`~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl`
- 归档 sessions：`~/.codex/archived_sessions/rollout-*.jsonl`

### JSONL 格式差异

| 字段 | Claude Code | Codex CLI |
|------|-------------|-----------|
| 项目路径 | 从文件目录路径解析 | 从 `session_meta.cwd` 字段读取 |
| 模型 | `message.model` | `turn_context.model` |
| Token 用量 | assistant message 内 `usage` 对象 | 独立 `event_msg` type=`token_count`，取 `last_token_usage` |
| Input tokens | `usage.input_tokens` | `last_token_usage.input_tokens` |
| Output tokens | `usage.output_tokens` | `last_token_usage.output_tokens + reasoning_output_tokens` |
| Cache tokens | `cache_creation_input_tokens` + `cache_read_input_tokens` | `cached_input_tokens` |

### Token 映射规则

```
Codex → DB 字段:
  input_tokens → input_tokens
  output_tokens + reasoning_output_tokens → output_tokens（合并）
  cached_input_tokens → cache_read_tokens
  (无 cache_creation 概念) → cache_creation_tokens = 0
```

### 解析逻辑

1. 读取 `session_meta` 行获取 `cwd`（项目路径）
2. 读取 `turn_context` 行获取 `model`
3. 读取 `event_msg` type=`token_count` 行，取 `last_token_usage` 计算增量
4. 生成 `ParsedRequest` 插入同一张 `requests` 表

### Watcher 扩展

- 新增监控路径：`~/.codex/sessions/` 递归扫描
- 复用现有 polling 机制（500ms）
- 文件名模式：`rollout-*.jsonl`

## 数据库 source 字段

新增字段区分数据来源：

```sql
ALTER TABLE requests ADD COLUMN source TEXT DEFAULT 'claude';
```

值：`"claude"` | `"codex"`

用于 UI 展示和可能的按来源过滤。

## 格式化

- `$0.00` ~ `$9.99`：显示两位小数
- `$10.00` ~ `$999.99`：显示两位小数
- `$1000+`：显示为 `$1.2k`
