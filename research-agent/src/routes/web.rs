use std::path::Path;
use tower_http::{
    services::{ServeDir, ServeFile},
    set_status::SetStatus,
};

pub(super) fn static_files(directory: &str) -> ServeDir<SetStatus<ServeFile>> {
    ServeDir::new(directory)
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new(Path::new(directory).join("404.html")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn serves_exported_files_with_correct_types_and_blocks_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("out");
        std::fs::create_dir_all(root.join("_next/static/chunks")).unwrap();
        std::fs::write(root.join("index.html"), "<h1>Workspace</h1>").unwrap();
        std::fs::write(root.join("404.html"), "Not found").unwrap();
        std::fs::write(
            root.join("_next/static/chunks/app.js"),
            "console.log('app')",
        )
        .unwrap();
        std::fs::write(root.join("_next/static/app.css"), "body{}").unwrap();
        std::fs::write(dir.path().join("secret"), "private").unwrap();
        let router = Router::new().fallback_service(static_files(root.to_str().unwrap()));
        for (path, status, content_type) in [
            ("/", StatusCode::OK, "text/html"),
            ("/_next/static/chunks/app.js", StatusCode::OK, "javascript"),
            ("/_next/static/app.css", StatusCode::OK, "text/css"),
            ("/missing", StatusCode::NOT_FOUND, "text/html"),
            ("/../secret", StatusCode::NOT_FOUND, "text/html"),
            ("/%2e%2e/secret", StatusCode::NOT_FOUND, "text/html"),
        ] {
            let response = router
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), status, "{path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .contains(content_type),
                "{path}"
            );
            let body = to_bytes(response.into_body(), 1024).await.unwrap();
            assert!(!String::from_utf8_lossy(&body).contains("private"));
        }
        let response = router
            .oneshot(
                Request::builder()
                    .method("HEAD")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty());
    }
}
