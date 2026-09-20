use super::response;
use crate::app::App;
use crate::{db::NewRun, pipeline};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) async fn list_runs(State(app): State<Arc<App>>) -> Response {
    match app.db.list_runs() {
        Ok(runs) => response(StatusCode::OK, json!({ "runs": runs })),
        Err(e) => response(StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e })),
    }
}

pub(super) async fn create_run(State(app): State<Arc<App>>, Json(body): Json<NewRun>) -> Response {
    if body.topic.trim().is_empty() {
        return response(
            StatusCode::BAD_REQUEST,
            json!({ "error": "topic must not be empty" }),
        );
    }
    let catalog = match app.model_catalog().await {
        Ok(catalog) => catalog,
        Err(error) => return response(StatusCode::SERVICE_UNAVAILABLE, json!({"error":error})),
    };
    for model in [&body.model_research, &body.model_critique, &body.model_fix] {
        if !model.is_empty()
            && !catalog
                .models
                .iter()
                .any(|available| &available.id == model)
        {
            return response(
                StatusCode::BAD_REQUEST,
                json!({"error":format!("model is not available: {model}")}),
            );
        }
    }
    // The image model is only meaningful when images will be generated. It is
    // validated independently because it is left empty otherwise.
    if body.generate_images
        && !body.model_image.is_empty()
        && !catalog
            .models
            .iter()
            .any(|available| available.id == body.model_image)
    {
        return response(
            StatusCode::BAD_REQUEST,
            json!({"error":format!("model is not available: {}", body.model_image)}),
        );
    }
    let default_model = catalog
        .models
        .iter()
        .find(|model| model.is_default)
        .or_else(|| catalog.models.first())
        .expect("catalog is nonempty")
        .id
        .clone();
    let new = NewRun {
        topic: body.topic.trim().to_string(),
        summary: body.summary,
        channel_url: body.channel_url,
        assets: body.assets,
        concept_count: body.concept_count.clamp(1, 12),
        model_research: if body.model_research.is_empty() {
            default_model.clone()
        } else {
            body.model_research
        },
        model_critique: if body.model_critique.is_empty() {
            default_model.clone()
        } else {
            body.model_critique
        },
        model_fix: if body.model_fix.is_empty() {
            default_model.clone()
        } else {
            body.model_fix
        },
        model_image: if body.generate_images && body.model_image.is_empty() {
            default_model
        } else {
            body.model_image
        },
        generate_images: body.generate_images,
    };
    let id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = app.db.create_run(&new, &id) {
        return response(StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e }));
    }
    pipeline::spawn_run(
        app.db.clone(),
        app.providers.clone(),
        id.clone(),
        app.db_path.clone(),
    );
    response(StatusCode::CREATED, json!({ "id": id }))
}

pub(super) async fn run_detail(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let run = match app.db.get_run(&id) {
        Ok(Some(run)) => run,
        Ok(None) => return response(StatusCode::NOT_FOUND, json!({ "error": "run not found" })),
        Err(e) => return response(StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e })),
    };
    let messages = match app.db.list_messages(&id) {
        Ok(messages) => messages,
        Err(e) => return response(StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e })),
    };
    let feedback = app
        .db
        .latest_feedback(&id)
        .map(|f| json!({ "stage": f.stage, "overall": f.overall, "items": f.items }));
    let result = run
        .result
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);
    response(
        StatusCode::OK,
        json!({
            "run": { "id": run.id, "created_at": run.created_at, "updated_at": run.updated_at,
                "topic": run.topic, "summary": run.summary, "channel_url": run.channel_url, "assets": run.assets,
                "concept_count": run.concept_count, "model_research": run.model_research,
                "model_critique": run.model_critique, "model_fix": run.model_fix, "model_image": run.model_image,
                "generate_images": run.generate_images,
                "status": run.status, "error": run.error },
            "result": result, "messages": messages, "feedback": feedback,
        }),
    )
}

pub(super) async fn delete_run(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    match app.db.delete_run(&id) {
        Ok(()) => response(StatusCode::OK, json!({ "ok": true })),
        Err(e) => response(StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e })),
    }
}
