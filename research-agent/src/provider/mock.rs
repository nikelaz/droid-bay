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

        if system.contains("fixer") {
            let bib = user
                .split("Current annotated bibliography")
                .nth(1)
                .and_then(extract_json)
                .ok_or("mock fixer: no bibliography found in prompt")?;
            let mut v: serde_json::Value =
                serde_json::from_str(&bib).map_err(|e| format!("mock fixer: {e}"))?;
            if let Some(arr) = v.get_mut("sources").and_then(|s| s.as_array_mut()) {
                for s in arr {
                    s["revised"] = serde_json::json!(true);
                }
            }
            Ok(serde_json::to_string_pretty(&v).unwrap())
        } else if system.contains("reviewer") {
            if user.contains("\"revised\"") {
                Ok(serde_json::json!({"verdict": "pass", "issues": []}).to_string())
            } else {
                Ok(serde_json::json!({
                    "verdict": "revise",
                    "issues": ["MOCK ISSUE: source s2 is only tangentially related to the topic; replace it with a source that directly addresses the research focus."]
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
                "sources": [
                    {
                        "id": "s1",
                        "title": format!("A systematic review of {topic}"),
                        "url": "https://example.org/paper/s1",
                        "authors": "Mock, A.",
                        "date": "2024",
                        "summary": format!("(mock) Peer-reviewed overview covering the history, key concepts, and current open questions of {topic}. Relevant because it frames the whole topic area."),
                        "relevance": format!("Directly surveys {topic} end to end.")
                    },
                    {
                        "id": "s2",
                        "title": format!("{topic} — reference overview"),
                        "url": "https://example.org/wiki/topic",
                        "authors": "",
                        "date": "2025",
                        "summary": format!("(mock) General-reference background on {topic}, useful for definitions and context."),
                        "relevance": "Provides baseline definitions and history."
                    },
                    {
                        "id": "s3",
                        "title": format!("Explaining {topic} in 12 minutes"),
                        "url": "https://example.org/video/s3",
                        "authors": "Mock Channel",
                        "date": "2023",
                        "summary": format!("(mock) Video walkthrough with visual examples of {topic}."),
                        "relevance": "Accessible introduction complementing the written sources."
                    }
                ]
            });
            Ok(serde_json::to_string_pretty(&out).unwrap())
        }
    }
}
