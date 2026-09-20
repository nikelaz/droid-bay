mod feedback;
mod models;
mod runs;
mod web;

use crate::app::App;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/models", get(models::models))
        .route("/api/runs", get(runs::list_runs).post(runs::create_run))
        .route(
            "/api/runs/{id}",
            get(runs::run_detail).delete(runs::delete_run),
        )
        .route("/api/runs/{id}/feedback", post(feedback::post_feedback))
        .route("/api", any(not_found))
        .route("/api/{*path}", any(not_found))
        .fallback_service(web::static_files(&app.web_dir))
        .with_state(app)
}

fn response(status: StatusCode, value: Value) -> Response {
    (status, Json(value)).into_response()
}

async fn not_found() -> Response {
    response(StatusCode::NOT_FOUND, json!({ "error": "not found" }))
}
