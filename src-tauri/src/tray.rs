use crate::config::TrayConfig;
use crate::db::Database;
use crate::parser::ParsedRequest;
use std::sync::Arc;

pub fn format_tray_text(request: &ParsedRequest, config: &TrayConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    let duration_s = request.duration_ms.filter(|&ms| ms > 0).map(|ms| ms as f64 / 1000.0);

    for item in &config.items {
        match item.as_str() {
            "out_rate" => {
                if let Some(ds) = duration_s {
                    let rate = (request.output_tokens as f64 / ds).round() as i64;
                    parts.push(format!("↓{}", format_rate(rate)));
                }
            }
            "in_rate" => {
                if let Some(ds) = duration_s {
                    let rate = (request.input_tokens as f64 / ds).round() as i64;
                    parts.push(format!("↑{}", format_rate(rate)));
                }
            }
            "ttft" => {
                if let Some(ms) = request.duration_ms.filter(|&ms| ms > 0) {
                    parts.push(format!("⏱{}", format_duration(ms)));
                } else {
                    parts.push("⏱—".to_string());
                }
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        "✧".to_string()
    } else {
        format!("✧ {}", parts.join(" "))
    }
}

pub fn format_idle_tray_text(config: &TrayConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    for item in &config.items {
        match item.as_str() {
            "out_rate" => parts.push("↓—".to_string()),
            "in_rate" => parts.push("↑—".to_string()),
            "ttft" => parts.push("⏱—".to_string()),
            _ => {}
        }
    }
    if parts.is_empty() {
        "✧".to_string()
    } else {
        format!("✧ {}", parts.join(" "))
    }
}

fn format_rate(tok_per_s: i64) -> String {
    if tok_per_s >= 1_000 {
        format!("{:.1}k", tok_per_s as f64 / 1_000.0)
    } else {
        tok_per_s.to_string()
    }
}

fn format_duration(ms: i64) -> String {
    if ms >= 60_000 {
        format!("{:.1}m", ms as f64 / 60_000.0)
    } else if ms >= 1_000 {
        format!("{:.1}s", ms as f64 / 1_000.0)
    } else {
        format!("{}ms", ms)
    }
}

pub fn format_average_tray_text(db: &Arc<Database>, since: &str, config: &TrayConfig) -> String {
    let records = match db.query_requests(since, None, None) {
        Ok(r) => r,
        Err(_) => return format_idle_tray_text(config),
    };

    let valid: Vec<_> = records.iter().filter(|r| r.duration_ms.map_or(false, |ms| ms > 0)).collect();
    if valid.is_empty() {
        return format_idle_tray_text(config);
    }

    let total_out: i64 = valid.iter().map(|r| r.output_tokens).sum();
    let total_in: i64 = valid.iter().map(|r| r.input_tokens).sum();
    let total_duration_s: f64 = valid.iter().map(|r| r.duration_ms.unwrap() as f64 / 1000.0).sum();
    let avg_duration_ms: i64 = valid.iter().map(|r| r.duration_ms.unwrap()).sum::<i64>() / valid.len() as i64;

    let avg_out_rate = (total_out as f64 / total_duration_s).round() as i64;
    let avg_in_rate = (total_in as f64 / total_duration_s).round() as i64;

    let mut parts: Vec<String> = Vec::new();
    for item in &config.items {
        match item.as_str() {
            "out_rate" => parts.push(format!("↓{}", format_rate(avg_out_rate))),
            "in_rate" => parts.push(format!("↑{}", format_rate(avg_in_rate))),
            "ttft" => parts.push(format!("⏱{}", format_duration(avg_duration_ms))),
            _ => {}
        }
    }

    if parts.is_empty() { "—".to_string() } else { parts.join(" ") }
}
