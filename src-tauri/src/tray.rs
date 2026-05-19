use crate::config::TrayConfig;
use crate::parser::ParsedRequest;

pub fn format_tray_text(request: &ParsedRequest, config: &TrayConfig) -> String {
    let mut parts: Vec<String> = Vec::new();

    for item in &config.items {
        match item.as_str() {
            "out_rate" => parts.push(format!("↓{}", format_tokens(request.output_tokens))),
            "in_rate" => parts.push(format!("↑{}", format_tokens(request.input_tokens))),
            "ttft" => {
                let ms = request.duration_ms.unwrap_or(0);
                parts.push(format_duration(ms));
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        "⬡".to_string()
    } else {
        format!("⬡ {}", parts.join(" "))
    }
}

pub fn format_idle_tray_text(config: &TrayConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    for item in &config.items {
        match item.as_str() {
            "out_rate" => parts.push("↓0".to_string()),
            "in_rate" => parts.push("↑0".to_string()),
            "ttft" => parts.push("0s".to_string()),
            _ => {}
        }
    }
    if parts.is_empty() {
        "⬡".to_string()
    } else {
        format!("⬡ {}", parts.join(" "))
    }
}

fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
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
