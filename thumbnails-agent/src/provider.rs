pub(crate) mod catalog;
mod codex;
mod mock;
mod opencode;
mod process;

use crate::config::Config;
use codex::CodexProvider;
use mock::MockProvider;
use opencode::OpencodeProvider;
use serde::Serialize;
use std::{
    collections::HashSet,
    path::Path,
    sync::Arc,
};

/// Separates the harness provider name from the raw model id in a user-facing
/// model selection (`codex::gpt-5.5`, `opencode::anthropic/claude-sonnet-4`).
pub const SELECTION_SEPARATOR: &str = "::";

/// A model as reported by a single harness provider.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
}

/// A model exposed to the UI and stored on a run. The `id` is an opaque,
/// globally unique selection value; `provider` and `model` describe how to run
/// it.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogModel {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub display_name: String,
    pub is_default: bool,
}

pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn list_models(&self) -> Result<Vec<ModelInfo>, String>;
    fn run(&self, system: &str, user: &str, model: &str, workdir: &Path) -> Result<String, String>;
}

/// The set of harness providers available to a run. A run may mix providers
/// freely: every model selection names the provider it belongs to.
pub struct Providers {
    providers: Vec<Arc<dyn Provider>>,
}

impl Providers {
    pub fn new(providers: Vec<Arc<dyn Provider>>) -> Self {
        Self { providers }
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.providers.iter().map(|provider| provider.name()).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.providers.iter()
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|provider| provider.name() == name)
    }

    /// Runs `selection` (`provider::model`, or a bare model when only one
    /// provider is configured) with the given prompts.
    pub fn run(
        &self,
        system: &str,
        user: &str,
        selection: &str,
        workdir: &Path,
    ) -> Result<String, String> {
        let (provider_name, model) = split_selection(selection);
        let provider = match provider_name {
            Some(name) => self.get(name).ok_or_else(|| {
                format!("model selection '{selection}' names an unknown provider '{name}'")
            })?,
            None => self
                .providers
                .first()
                .ok_or("no providers are configured")?,
        };
        provider.run(system, user, model, workdir)
    }
}

/// Builds the composite selection value for a provider and raw model id.
pub fn selection(provider: &str, model: &str) -> String {
    format!("{provider}{SELECTION_SEPARATOR}{model}")
}

/// Splits a selection into its provider name (when present) and raw model id.
pub fn split_selection(selection: &str) -> (Option<&str>, &str) {
    match selection.split_once(SELECTION_SEPARATOR) {
        Some((provider, model)) if !provider.is_empty() => (Some(provider), model),
        _ => (None, selection),
    }
}

/// The single prompt handed to a harness invocation: standing system
/// instructions wrapped around the actual user task.
pub(crate) fn session_prompt(system: &str, user: &str) -> String {
    format!(
        "Follow these standing instructions for the whole session:\n<system_instructions>\n{system}\n</system_instructions>\n\n<user_task>\n{user}\n</user_task>"
    )
}

/// Builds every provider named in the config (defaulting to codex + opencode),
/// so a single run can mix them.
pub fn make_providers(cfg: &Config) -> Result<Providers, String> {
    let names = if cfg.providers.is_empty() {
        vec!["codex".to_string(), "opencode".to_string()]
    } else {
        cfg.providers.clone()
    };
    let mut providers: Vec<Arc<dyn Provider>> = Vec::new();
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }
        let provider: Arc<dyn Provider> = match name.as_str() {
            "mock" => Arc::new(MockProvider),
            "codex" => Arc::new(CodexProvider {
                bin: cfg.codex_bin.clone(),
                args: cfg.codex_args.clone(),
                timeout_secs: cfg.timeout_secs,
            }),
            "opencode" => Arc::new(OpencodeProvider {
                bin: cfg.opencode_bin.clone(),
                args: cfg.opencode_args.clone(),
                timeout_secs: cfg.timeout_secs,
            }),
            other => return Err(format!("unknown provider: {other}")),
        };
        providers.push(provider);
    }
    if providers.is_empty() {
        return Err("no providers are configured".into());
    }
    Ok(Providers::new(providers))
}

pub fn extract_json(text: &str) -> Option<String> {
    let bytes: Vec<char> = text.chars().collect();
    let start = bytes.iter().position(|&c| c == '{' || c == '[')?;
    let (open, close) = if bytes[start] == '{' {
        ('{', '}')
    } else {
        ('[', ']')
    };
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            o if o == open => depth += 1,
            cl if cl == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(bytes[start..=i].iter().collect());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests;
