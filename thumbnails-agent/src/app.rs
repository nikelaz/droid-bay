use crate::{
    db::Db,
    provider::{
        catalog::{CatalogSnapshot, ModelCatalog},
        Providers,
    },
};
use std::sync::Arc;

pub(crate) struct App {
    pub(crate) db: Db,
    pub(crate) providers: Arc<Providers>,
    pub(crate) models: Arc<ModelCatalog>,
    pub(crate) db_path: String,
    pub(crate) web_dir: String,
}

impl App {
    pub(crate) async fn model_catalog(&self) -> Result<CatalogSnapshot, String> {
        let models = self.models.clone();
        tokio::task::spawn_blocking(move || models.get())
            .await
            .map_err(|e| format!("model discovery task failed: {e}"))?
    }
}
