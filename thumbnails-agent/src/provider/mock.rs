use super::{extract_json, ModelInfo, Provider};
use std::path::Path;

pub struct MockProvider;

impl Provider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        Ok(vec![ModelInfo {
            id: "mock".into(),
            display_name: "Mock".into(),
            is_default: true,
        }])
    }

    fn run(
        &self,
        system: &str,
        user: &str,
        _model: &str,
        _workdir: &Path,
    ) -> Result<String, String> {
        std::thread::sleep(std::time::Duration::from_millis(800));

        if system.starts_with("You are an image-generation model") {
            let source = user
                .split("Current concepts or thumbnails:")
                .nth(1)
                .and_then(extract_json)
                .ok_or("mock image model: no concepts found")?;
            let v: serde_json::Value = serde_json::from_str(&source).map_err(|e| e.to_string())?;
            let thumbnails = v.get("concepts").or_else(|| v.get("thumbnails"))
                .and_then(|x| x.as_array()).ok_or("mock image model: no items")?
                .iter().enumerate().map(|(i, x)| serde_json::json!({
                    "id": x.get("id").cloned().unwrap_or(serde_json::json!(format!("c{}", i+1))),
                    "prompt": x.get("prompt").cloned().unwrap_or(serde_json::json!("Mock thumbnail")),
                    "image_url": format!("https://placehold.co/1280x720/171717/ffffff?text=Thumbnail+{}", i+1)
                })).collect::<Vec<_>>();
            Ok(serde_json::json!({"thumbnails": thumbnails}).to_string())
        } else if system.contains("prompt refiner") {
            let bib = user
                .split("Current concepts:")
                .nth(1)
                .and_then(extract_json)
                .ok_or("mock fixer: no bibliography found in prompt")?;
            let mut v: serde_json::Value =
                serde_json::from_str(&bib).map_err(|e| format!("mock fixer: {e}"))?;
            if let Some(arr) = v.get_mut("concepts").and_then(|s| s.as_array_mut()) {
                for s in arr {
                    s["revised"] = serde_json::json!(true);
                }
            }
            Ok(serde_json::to_string_pretty(&v).unwrap())
        } else if system.contains("thumbnail reviewer") {
            if user.contains("\"revised\"") {
                Ok(serde_json::json!({"verdict": "pass", "issues": []}).to_string())
            } else {
                Ok(serde_json::json!({
                    "verdict": "revise",
                    "issues": ["MOCK ISSUE: c2 needs a stronger focal subject and larger mobile-readable text."]
                }).to_string())
            }
        } else {
            let topic = user
                .lines()
                .find_map(|l| l.strip_prefix("Topic: "))
                .unwrap_or("the topic")
                .trim()
                .to_string();
            let out = serde_json::json!({
                "concepts": [
                    {
                        "id": "c1", "title": "The impossible question",
                        "prompt": format!("16:9 YouTube thumbnail for {topic}, one shocked creator close-up, giant question mark, high contrast blue and yellow, 3 word headline, clean mobile-readable layout"),
                        "rationale": "A direct curiosity hook with one clear focal point."
                    },
                    {
                        "id": "c2", "title": "Before and after",
                        "prompt": format!("16:9 split-screen thumbnail explaining {topic}, dramatic red failure versus green success, oversized arrow, bold 2 word headline, cinematic lighting"),
                        "rationale": "Shows the payoff in a glance."
                    },
                    {
                        "id": "c3", "title": "The hidden mechanism",
                        "prompt": format!("16:9 thumbnail for {topic}, one mysterious glowing object on dark background, creator pointing, vivid cyan accent, short punchy text, strong negative space"),
                        "rationale": "Builds intrigue without clutter."
                    }
                ]
            });
            Ok(serde_json::to_string_pretty(&out).unwrap())
        }
    }
}
