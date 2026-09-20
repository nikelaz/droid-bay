use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_providers")]
    pub providers: Vec<String>,
    #[serde(default = "default_codex_bin")]
    pub codex_bin: String,
    #[serde(default = "default_codex_args")]
    pub codex_args: Vec<String>,
    #[serde(default = "default_opencode_bin")]
    pub opencode_bin: String,
    #[serde(default = "default_opencode_args")]
    pub opencode_args: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub models: Vec<String>,
}

fn default_providers() -> Vec<String> {
    vec!["codex".into(), "opencode".into()]
}
fn default_codex_bin() -> String {
    "codex".into()
}
fn default_opencode_bin() -> String {
    "opencode".into()
}
fn default_opencode_args() -> Vec<String> {
    Vec::new()
}
fn default_timeout() -> u64 {
    1800
}
fn default_codex_args() -> Vec<String> {
    [
        "--skip-git-repo-check",
        "-s",
        "workspace-write",
        "-c",
        "sandbox_workspace_write.network_access=true",
        "-c",
        "tools.web_search=true",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            providers: default_providers(),
            codex_bin: default_codex_bin(),
            codex_args: default_codex_args(),
            opencode_bin: default_opencode_bin(),
            opencode_args: default_opencode_args(),
            timeout_secs: default_timeout(),
            models: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Config {
        match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Config>(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("warning: failed to parse {path}: {e}; using defaults");
                    Config::default()
                }
            },
            Err(_) => {
                let cfg = Config::default();
                if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                    let _ = std::fs::write(path, json + "\n");
                    eprintln!("wrote default config to {path}");
                }
                cfg
            }
        }
    }
}
