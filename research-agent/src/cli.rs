pub(crate) struct Args {
    pub(crate) port: u16,
    pub(crate) db_path: String,
    pub(crate) config_path: String,
    pub(crate) web_dir: String,
    pub(crate) provider: Option<String>,
}

impl Args {
    pub(crate) fn parse() -> Option<Self> {
        let mut port = 8080u16;
        let mut db_path = "data/agent.db".to_string();
        let mut config_path = "config.json".to_string();
        let mut web_dir = "web/out".to_string();
        let mut provider = None;
        let mut args = std::env::args().skip(1);
        while let Some(a) = args.next() {
            match a.as_str() {
                "--port" | "-p" => port = args.next().and_then(|v| v.parse().ok()).unwrap_or(8080),
                "--db" => db_path = args.next().unwrap_or(db_path),
                "--config" => config_path = args.next().unwrap_or(config_path),
                "--web" => web_dir = args.next().unwrap_or(web_dir),
                "--provider" => {
                    let Some(value) = args.next() else {
                        eprintln!("--provider requires a name (codex, opencode, mock)");
                        return None;
                    };
                    provider = Some(value);
                }
                "--help" | "-h" => {
                    println!("research-agent — an LLM research pipeline with a web UI");
                    println!("usage: research-agent [--port 8080] [--db data/agent.db] [--config config.json] [--web web/out] [--provider NAME]");
                    println!("  --provider NAME  restrict the run to a single provider (default: all configured providers)");
                    return None;
                }
                other => eprintln!("unknown argument: {other}"),
            }
        }
        Some(Self {
            port,
            db_path,
            config_path,
            web_dir,
            provider,
        })
    }
}
