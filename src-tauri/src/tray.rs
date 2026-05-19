use crate::config::TrayConfig;
use crate::parser::ParsedRequest;

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
        "—".to_string()
    } else {
        parts.join(" ")
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
        "—".to_string()
    } else {
        parts.join(" ")
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
