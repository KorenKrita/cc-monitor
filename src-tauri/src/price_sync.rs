use std::collections::HashMap;
use crate::config::ModelPrice;

const LITELLM_URL: &str = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
const MODELS_DEV_URL: &str = "https://models.dev/api.json";

pub async fn fetch_litellm_prices() -> Result<HashMap<String, ModelPrice>, String> {
    let resp = reqwest::get(LITELLM_URL).await.map_err(|e| e.to_string())?;
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let obj = data.as_object().ok_or("Invalid litellm format")?;
    let mut prices = HashMap::new();

    for (model_name, info) in obj {
        if model_name.starts_with("sample_spec") || !info.is_object() {
            continue;
        }
        let input = info.get("input_cost_per_token")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) * 1_000_000.0;
        let output = info.get("output_cost_per_token")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) * 1_000_000.0;
        let cache = info.get("cache_read_input_token_cost")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) * 1_000_000.0;

        if input > 0.0 || output > 0.0 {
            prices.insert(model_name.clone(), ModelPrice {
                input,
                output,
                cache,
                source: "litellm".to_string(),
            });
        }
    }

    Ok(prices)
}

pub async fn fetch_models_dev_prices() -> Result<HashMap<String, ModelPrice>, String> {
    let resp = reqwest::get(MODELS_DEV_URL).await.map_err(|e| e.to_string())?;
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let arr = data.as_array().ok_or("Invalid models.dev format")?;
    let mut prices = HashMap::new();

    for item in arr {
        let model_name = match item.get("id").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let pricing = match item.get("pricing") {
            Some(p) => p,
            None => continue,
        };

        let input_str = pricing.get("input").and_then(|v| v.as_str()).unwrap_or("0");
        let output_str = pricing.get("output").and_then(|v| v.as_str()).unwrap_or("0");

        let input: f64 = input_str.parse().unwrap_or(0.0) * 1_000_000.0;
        let output: f64 = output_str.parse().unwrap_or(0.0) * 1_000_000.0;

        if input > 0.0 || output > 0.0 {
            prices.insert(model_name, ModelPrice {
                input,
                output,
                cache: 0.0,
                source: "models.dev".to_string(),
            });
        }
    }

    Ok(prices)
}

pub async fn sync_prices(current_prices: &HashMap<String, ModelPrice>) -> Result<HashMap<String, ModelPrice>, String> {
    let mut result = current_prices.clone();

    let sources: Vec<Result<HashMap<String, ModelPrice>, String>> = vec![
        fetch_models_dev_prices().await,
        fetch_litellm_prices().await,
    ];

    for fetch_result in sources {
        if let Ok(fetched) = fetch_result {
            for (model, price) in fetched {
                match result.get(&model) {
                    Some(existing) if existing.source == "manual" => {}
                    _ => {
                        result.insert(model, price);
                    }
                }
            }
        }
    }

    Ok(result)
}