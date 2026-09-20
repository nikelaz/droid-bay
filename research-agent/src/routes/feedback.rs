use super::response;
use crate::app::App;
use crate::pipeline;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) async fn post_feedback(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    if app.db.get_run(&id).ok().flatten().is_none() {
        return response(StatusCode::NOT_FOUND, json!({ "error": "run not found" }));
    }
    match body.get("action").and_then(Value::as_str) {
        Some("accept") => {
            if app.db.cas_status(&id, "review", "accepted") {
                app.db.append_message(
                    &id,
                    "review",
                    "event",
                    "Results accepted by the user. Run complete.",
                );
                response(StatusCode::OK, json!({ "ok": true, "status": "accepted" }))
            } else {
                response(
                    StatusCode::CONFLICT,
                    json!({ "error": "run is not in the review state" }),
                )
            }
        }
        Some("revise") => revise_run(app, id, body),
        action => response(
            StatusCode::BAD_REQUEST,
            json!({ "error": format!("unknown action: {action:?}") }),
        ),
    }
}

fn revise_run(app: Arc<App>, id: String, body: Value) -> Response {
    let overall = body
        .get("overall")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let source_feedback = body
        .get("sources")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_sources = source_feedback
        .values()
        .any(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()));
    if overall.is_empty() && !has_sources {
        return response(
            StatusCode::BAD_REQUEST,
            json!({ "error": "provide overall feedback and/or per-source feedback" }),
        );
    }
    if !app.db.cas_status(&id, "review", "running_fix") {
        return response(
            StatusCode::CONFLICT,
            json!({ "error": "run is not in the review state" }),
        );
    }
    let source_feedback = Value::Object(source_feedback);
    app.db.save_feedback(&id, &overall, &source_feedback);
    let mut text = if overall.is_empty() {
        String::new()
    } else {
        format!("Overall: {overall}")
    };
    for (source_id, value) in source_feedback.as_object().unwrap() {
        if let Some(feedback) = value
            .as_str()
            .filter(|feedback| !feedback.trim().is_empty())
        {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&format!("Source {source_id}: {}", feedback.trim()));
        }
    }
    app.db.append_message(&id, "human_feedback", "user", &text);
    pipeline::spawn_human_fix(
        app.db.clone(),
        app.providers.clone(),
        id,
        app.db_path.clone(),
    );
    response(
        StatusCode::OK,
        json!({ "ok": true, "status": "running_fix" }),
    )
}
