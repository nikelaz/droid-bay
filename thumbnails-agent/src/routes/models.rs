use super::response;
use crate::app::App;
use axum::{extract::State, http::StatusCode, response::Response};
use serde_json::json;
use std::sync::Arc;

pub(super) async fn models(State(app): State<Arc<App>>) -> Response {
    match app.model_catalog().await {
        Ok(catalog) => response(StatusCode::OK, json!(catalog)),
        Err(error) => response(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    }
}
