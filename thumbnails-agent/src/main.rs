mod app;
mod cli;
mod config;
mod db;
mod pipeline;
mod prompts;
mod provider;
mod routes;
mod template;

use app::App;
use cli::Args;
use config::Config;
use db::Db;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(Args {
        port,
        db_path,
        config_path,
        web_dir,
        provider,
    }) = Args::parse()
    else {
        return Ok(());
    };
    if !std::path::Path::new(&web_dir).join("index.html").is_file() {
        return Err(std::io::Error::other(format!(
            "static UI not found in '{web_dir}'; run `npm ci --prefix web && npm run build --prefix web`, or use --web to select an exported site"
        )).into());
    }
    let mut cfg = Config::load(&config_path);
    // The CLI flag restricts the run to a single provider (e.g. the mock
    // provider in tests); by default every configured provider is available.
    if let Some(name) = provider {
        cfg.providers = vec![name];
    }
    let db = Db::open(&db_path).map_err(std::io::Error::other)?;
    let providers = Arc::new(provider::make_providers(&cfg).map_err(std::io::Error::other)?);
    let models = Arc::new(provider::catalog::ModelCatalog::new(
        providers.clone(),
        cfg.models,
    ));
    let app = Arc::new(App {
        db,
        providers,
        models,
        db_path: db_path.clone(),
        web_dir,
    });
    let router = routes::router(app.clone());
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!(
        "thumbnails-agent: listening on http://{}",
        listener.local_addr()?
    );
    eprintln!(
        "thumbnails-agent: providers=[{}], db='{}'",
        app.providers.names().join(", "),
        db_path,
    );
    axum::serve(listener, router).await?;
    Ok(())
}
