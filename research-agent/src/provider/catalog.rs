use super::{selection, CatalogModel, Providers};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
pub(crate) struct CatalogSnapshot {
    pub models: Vec<CatalogModel>,
    pub stale: bool,
    pub warning: Option<String>,
}

struct Cache {
    snapshot: Option<CatalogSnapshot>,
    error: Option<String>,
    retry_at: Instant,
}

pub(crate) struct ModelCatalog {
    providers: Arc<Providers>,
    allowlist: Vec<String>,
    cache: Mutex<Cache>,
}

impl ModelCatalog {
    pub fn new(providers: Arc<Providers>, allowlist: Vec<String>) -> Self {
        Self {
            providers,
            allowlist,
            cache: Mutex::new(Cache {
                snapshot: None,
                error: None,
                retry_at: Instant::now(),
            }),
        }
    }

    /// Queries every provider, keeping the ones that answered and reporting the
    /// rest as warnings so one broken harness never hides the others.
    fn discover(&self) -> (Vec<CatalogModel>, Vec<String>) {
        let mut models = Vec::new();
        let mut warnings = Vec::new();
        for provider in self.providers.iter() {
            match provider.list_models() {
                Ok(list) => {
                    for model in list {
                        if !self.allowed(provider.name(), &model.id) {
                            continue;
                        }
                        models.push(CatalogModel {
                            id: selection(provider.name(), &model.id),
                            provider: provider.name().to_string(),
                            model: model.id,
                            display_name: model.display_name,
                            is_default: model.is_default,
                        });
                    }
                }
                Err(error) => warnings.push(format!("{}: {error}", provider.name())),
            }
        }
        if !models.iter().any(|model| model.is_default) {
            if let Some(first) = models.first_mut() {
                first.is_default = true;
            }
        }
        (models, warnings)
    }

    /// The allowlist accepts either a composite selection (`codex::gpt-5.5`)
    /// or a bare model id shared by every provider.
    fn allowed(&self, provider: &str, model: &str) -> bool {
        self.allowlist.is_empty()
            || self
                .allowlist
                .iter()
                .any(|entry| entry == model || entry == &selection(provider, model))
    }

    /// Called on a blocking worker; the mutex coalesces concurrent refreshes.
    pub fn get(&self) -> Result<CatalogSnapshot, String> {
        let mut cache = self.cache.lock().map_err(|_| "model cache lock poisoned")?;
        if Instant::now() >= cache.retry_at {
            let (models, warnings) = self.discover();
            if models.is_empty() {
                let error = if warnings.is_empty() {
                    "no models are available (check the model allowlist and provider setup)".into()
                } else {
                    warnings.join("; ")
                };
                eprintln!("model discovery: {error}");
                if let Some(snapshot) = &mut cache.snapshot {
                    snapshot.stale = true;
                    snapshot.warning = Some(format!(
                        "Model refresh failed; using the last available catalog: {error}"
                    ));
                }
                cache.error = Some(error);
                cache.retry_at = Instant::now() + Duration::from_secs(30);
            } else {
                let warning = if warnings.is_empty() {
                    None
                } else {
                    Some(format!(
                        "Some providers are unavailable — {}",
                        warnings.join("; ")
                    ))
                };
                cache.snapshot = Some(CatalogSnapshot {
                    models,
                    stale: false,
                    warning,
                });
                cache.error = None;
                cache.retry_at = Instant::now() + Duration::from_secs(300);
            }
        }
        cache.snapshot.clone().ok_or_else(|| {
            cache
                .error
                .clone()
                .unwrap_or_else(|| "model catalog unavailable".into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ModelInfo, Provider};
    use std::{
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct CatalogProvider {
        name: &'static str,
        calls: AtomicUsize,
        fail_after_first: bool,
    }

    impl CatalogProvider {
        fn new(name: &'static str, fail_after_first: bool) -> Arc<Self> {
            Arc::new(Self {
                name,
                calls: AtomicUsize::new(0),
                fail_after_first,
            })
        }
    }

    impl Provider for CatalogProvider {
        fn name(&self) -> &'static str {
            self.name
        }
        fn run(&self, _: &str, _: &str, _: &str, _: &Path) -> Result<String, String> {
            unreachable!()
        }
        fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
            if self.fail_after_first && self.calls.fetch_add(1, Ordering::SeqCst) > 0 {
                return Err("offline".into());
            }
            Ok(vec![
                ModelInfo {
                    id: "a".into(),
                    display_name: "A".into(),
                    is_default: true,
                },
                ModelInfo {
                    id: "b".into(),
                    display_name: "B".into(),
                    is_default: false,
                },
            ])
        }
    }

    fn providers(list: Vec<Arc<CatalogProvider>>) -> Arc<Providers> {
        Arc::new(Providers::new(
            list.into_iter().map(|p| p as Arc<dyn Provider>).collect(),
        ))
    }

    #[test]
    fn caches_and_preserves_last_success_on_refresh_failure() {
        let provider = CatalogProvider::new("test", true);
        let catalog = ModelCatalog::new(providers(vec![provider.clone()]), vec![]);
        assert!(!catalog.get().unwrap().stale);
        assert!(!catalog.get().unwrap().stale);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        catalog.cache.lock().unwrap().retry_at = Instant::now();
        let stale = catalog.get().unwrap();
        assert!(stale.stale);
        assert_eq!(stale.models.len(), 2);
        assert!(stale.warning.unwrap().contains("offline"));
        catalog.get().unwrap();
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn allowlist_filters_and_selects_fallback_default() {
        let catalog = ModelCatalog::new(providers(vec![CatalogProvider::new("test", false)]), vec!["b".into()]);
        let snapshot = catalog.get().unwrap();
        assert_eq!(snapshot.models.len(), 1);
        assert_eq!(snapshot.models[0].id, "test::b");
        assert_eq!(snapshot.models[0].model, "b");
        assert!(snapshot.models[0].is_default);
    }

    #[test]
    fn composite_allowlist_entries_are_accepted() {
        let catalog = ModelCatalog::new(
            providers(vec![CatalogProvider::new("test", false)]),
            vec!["test::a".into()],
        );
        let snapshot = catalog.get().unwrap();
        assert_eq!(snapshot.models.len(), 1);
        assert_eq!(snapshot.models[0].model, "a");
    }

    #[test]
    fn empty_allowlist_match_is_an_error() {
        let catalog = ModelCatalog::new(
            providers(vec![CatalogProvider::new("test", false)]),
            vec!["missing".into()],
        );
        assert!(catalog.get().err().unwrap().contains("no models"));
    }

    #[test]
    fn first_failure_is_reported_and_throttled() {
        let provider = CatalogProvider::new("test", true);
        // Start with a failing call so no snapshot is cached.
        provider.calls.store(1, Ordering::SeqCst);
        let catalog = ModelCatalog::new(providers(vec![provider.clone()]), vec![]);
        assert!(catalog.get().err().unwrap().contains("offline"));
        assert!(catalog.get().err().unwrap().contains("offline"));
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn aggregates_models_from_every_provider() {
        let catalog = ModelCatalog::new(
            providers(vec![
                CatalogProvider::new("codex", false),
                CatalogProvider::new("opencode", false),
            ]),
            vec![],
        );
        let snapshot = catalog.get().unwrap();
        assert_eq!(snapshot.models.len(), 4);
        assert!(snapshot.warning.is_none());
        assert!(snapshot.models.iter().any(|m| m.id == "codex::a"));
        assert!(snapshot.models.iter().any(|m| m.id == "opencode::b"));
    }

    #[test]
    fn partial_provider_failure_keeps_the_others_and_warns() {
        let offline = CatalogProvider::new("codex", true);
        offline.calls.store(1, Ordering::SeqCst);
        // A provider that always errors: reuse fail_after_first with a primed call.
        let catalog = ModelCatalog::new(
            providers(vec![offline, CatalogProvider::new("opencode", false)]),
            vec![],
        );
        let snapshot = catalog.get().unwrap();
        assert_eq!(snapshot.models.len(), 2);
        assert!(snapshot.models.iter().all(|m| m.provider == "opencode"));
        assert!(snapshot.warning.unwrap().contains("codex"));
    }
}
