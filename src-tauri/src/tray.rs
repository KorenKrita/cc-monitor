use crate::config::TrayConfig;
use crate::db::Database;
use crate::parser::ParsedRequest;
use std::sync::Arc;

pub fn format_cost(amount: f64) -> String {
    if amount >= 1000.0 {
        format!("${:.1}k", amount / 1000.0)
    } else {
        format!("${:.2}", amount)
    }
}

fn format_cost_item(cost: Option<f64>) -> String {
    match cost {
        Some(c) => format_cost(c),
        None => "$—".to_string(),
    }
}

pub fn calculate_cost_since(time_window: &str) -> String {
    let now = chrono::Utc::now();
    match time_window {
        "day" => (now - chrono::Duration::days(1)).to_rfc3339(),
        "month" => (now - chrono::Duration::days(30)).to_rfc3339(),
        "year" => (now - chrono::Duration::days(365)).to_rfc3339(),
        "all" => "1970-01-01T00:00:00Z".to_string(),
        _ => (now - chrono::Duration::days(1)).to_rfc3339(),
    }
}

pub fn format_tray_text(request: &ParsedRequest, config: &TrayConfig, cost: Option<f64>) -> String {
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
            "cost" => {
                parts.push(format_cost_item(cost));
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

pub fn format_idle_tray_text(config: &TrayConfig, cost: Option<f64>) -> String {
    let mut parts: Vec<String> = Vec::new();
    for item in &config.items {
        match item.as_str() {
            "out_rate" => parts.push("↓—".to_string()),
            "in_rate" => parts.push("↑—".to_string()),
            "ttft" => parts.push("⏱—".to_string()),
            "cost" => {
                parts.push(format_cost_item(cost));
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

pub fn format_average_tray_text(db: &Arc<Database>, since: &str, config: &TrayConfig, cost: Option<f64>) -> String {
    let records = match db.query_requests(since, None, None) {
        Ok(r) => r,
        Err(_) => return format_idle_tray_text(config, cost),
    };

    let valid: Vec<_> = records.iter().filter(|r| r.duration_ms.map_or(false, |ms| ms > 0)).collect();
    if valid.is_empty() {
        return format_idle_tray_text(config, cost);
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
            "cost" => {
                parts.push(format_cost_item(cost));
            }
            _ => {}
        }
    }

    if parts.is_empty() { "—".to_string() } else { parts.join(" ") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ParsedRequest;

    fn make_request(out: i64, inp: i64, dur_ms: i64) -> ParsedRequest {
        ParsedRequest {
            timestamp: "2026-05-20T10:00:00Z".to_string(),
            model: "test-model".to_string(),
            input_tokens: inp,
            output_tokens: out,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            duration_ms: Some(dur_ms),
            project: "".to_string(),
            source: "claude".to_string(),
        }
    }

    fn make_config(items: Vec<&str>) -> TrayConfig {
        TrayConfig {
            items: items.into_iter().map(String::from).collect(),
            model_filter: "last".to_string(),
            model_whitelist: vec![],
            display_mode: "last".to_string(),
            average_minutes: 5,
        }
    }

    #[test]
    fn test_format_cost_values() {
        assert_eq!(format_cost(0.0), "$0.00");
        assert_eq!(format_cost(1.5), "$1.50");
        assert_eq!(format_cost(99.99), "$99.99");
        assert_eq!(format_cost(1000.0), "$1.0k");
        assert_eq!(format_cost(2500.0), "$2.5k");
    }

    #[test]
    fn test_calculate_cost_since_all() {
        assert_eq!(calculate_cost_since("all"), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_calculate_cost_since_day() {
        let since = calculate_cost_since("day");
        assert!(since.contains("T"));
        assert!(since.len() > 20);
    }

    #[test]
    fn test_tray_text_out_rate_only() {
        let req = make_request(500, 1000, 5000);
        let config = make_config(vec!["out_rate"]);
        let text = format_tray_text(&req, &config, None);
        assert_eq!(text, "✧ ↓100");
    }

    #[test]
    fn test_tray_text_with_cost() {
        let req = make_request(500, 1000, 5000);
        let config = make_config(vec!["out_rate", "cost"]);
        let text = format_tray_text(&req, &config, Some(12.34));
        assert!(text.contains("↓100"));
        assert!(text.contains("$12.34"));
    }

    #[test]
    fn test_tray_text_cost_none_shows_dash() {
        let req = make_request(500, 1000, 5000);
        let config = make_config(vec!["cost"]);
        let text = format_tray_text(&req, &config, None);
        assert!(text.contains("$—"));
    }

    #[test]
    fn test_idle_tray_with_cost() {
        let config = make_config(vec!["out_rate", "cost"]);
        let text = format_idle_tray_text(&config, Some(5.67));
        assert!(text.contains("↓—"));
        assert!(text.contains("$5.67"));
    }

    #[test]
    fn test_empty_items_shows_hexagon() {
        let config = make_config(vec![]);
        let req = make_request(500, 1000, 5000);
        assert_eq!(format_tray_text(&req, &config, None), "✧");
        assert_eq!(format_idle_tray_text(&config, None), "✧");
    }
}
