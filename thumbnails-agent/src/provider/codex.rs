use super::{
    process::{stderr_tail, ManagedChild},
    session_prompt, ModelInfo, Provider,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Seek, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

pub(super) struct CodexProvider {
    pub bin: String,
    pub args: Vec<String>,
    pub timeout_secs: u64,
}

impl Provider for CodexProvider {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn run(&self, system: &str, user: &str, model: &str, workdir: &Path) -> Result<String, String> {
        std::fs::create_dir_all(workdir).map_err(|e| format!("cannot create workdir: {e}"))?;
        let workdir = workdir
            .canonicalize()
            .map_err(|e| format!("cannot resolve workdir: {e}"))?;
        let invocation = workdir
            .join("invocations")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&invocation)
            .map_err(|e| format!("cannot create invocation directory: {e}"))?;
        let output = invocation.join("last_message.txt");
        let stdout = File::create(invocation.join("stdout.log")).map_err(|e| e.to_string())?;
        let stderr_path = invocation.join("stderr.log");
        let stderr = File::create(&stderr_path).map_err(|e| e.to_string())?;
        // A file-backed stdin avoids pipe backpressure before the timeout starts.
        let mut input = tempfile::tempfile().map_err(|e| e.to_string())?;
        write!(input, "{}", session_prompt(system, user))
            .map_err(|e| format!("cannot write prompt: {e}"))?;
        input.rewind().map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&self.bin);
        cmd.arg("exec")
            .args(&self.args)
            .args(["-m", model])
            .arg("--output-last-message")
            .arg(&output);
        if !self.args.iter().any(|arg| arg == "--skip-git-repo-check") {
            cmd.arg("--skip-git-repo-check");
        }
        cmd.arg("-")
            .current_dir(&workdir)
            .stdin(input)
            .stdout(stdout)
            .stderr(stderr);
        let result = (|| {
            let mut child = ManagedChild::spawn(&mut cmd)?;
            let status = child.wait(Duration::from_secs(self.timeout_secs))?;
            if !status.success() {
                return Err(format!("provider exited with {status}"));
            }
            let text = std::fs::read_to_string(&output)
                .map_err(|e| format!("cannot read final response: {e}"))?;
            if text.trim().is_empty() {
                return Err("provider produced an empty final response".into());
            }
            Ok(text.trim().to_owned())
        })();
        result.map_err(|e| {
            let tail = File::open(&stderr_path)
                .map(|mut file| stderr_tail(&mut file))
                .unwrap_or_default();
            format!("{e}\nlogs: {}\nstderr tail:\n{tail}", invocation.display())
        })
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let mut stderr = tempfile::tempfile().map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&self.bin);
        cmd.arg("app-server");
        // Only global configuration flags are compatible with app-server.
        // Exec-specific switches (sandbox, output, etc.) must not be forwarded.
        let mut args = self.args.iter();
        while let Some(arg) = args.next() {
            if matches!(arg.as_str(), "-c" | "--config" | "--enable" | "--disable") {
                let value = args
                    .next()
                    .ok_or_else(|| format!("missing value for {arg}"))?;
                cmd.arg(arg).arg(value);
            } else if ["--config=", "--enable=", "--disable="]
                .iter()
                .any(|prefix| arg.starts_with(prefix))
            {
                cmd.arg(arg);
            }
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr.try_clone().map_err(|e| e.to_string())?);
        let result: Result<Vec<ModelInfo>, String> = (|| {
            let mut child = ManagedChild::spawn(&mut cmd)?;
            let mut input = child.0.stdin.take().ok_or("missing app-server stdin")?;
            let output = child.0.stdout.take().ok_or("missing app-server stdout")?;
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(output).lines() {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
            let deadline = Instant::now() + Duration::from_secs(self.timeout_secs.min(20));
            let receive = |id: u64| -> Result<Value, String> {
                loop {
                    let remaining = deadline
                        .checked_duration_since(Instant::now())
                        .ok_or("model discovery timed out")?;
                    let line = rx
                        .recv_timeout(remaining)
                        .map_err(|e| format!("model discovery interrupted or timed out: {e}"))?
                        .map_err(|e| format!("reading model discovery response: {e}"))?;
                    let message: Value = serde_json::from_str(&line)
                        .map_err(|e| format!("invalid app-server response: {e}"))?;
                    if message.get("id").and_then(Value::as_u64) != Some(id) {
                        continue;
                    }
                    if let Some(error) = message.get("error") {
                        return Err(format!("model discovery failed: {error}"));
                    }
                    return message
                        .get("result")
                        .cloned()
                        .ok_or_else(|| "missing app-server result".into());
                }
            };
            let send = |input: &mut std::process::ChildStdin,
                        message: Value|
             -> Result<(), String> {
                writeln!(input, "{message}").map_err(|e| format!("writing app-server request: {e}"))
            };
            send(
                &mut input,
                json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"research_agent","version":env!("CARGO_PKG_VERSION")}}}),
            )?;
            receive(1)?;
            send(&mut input, json!({"method":"initialized","params":{}}))?;
            let mut cursor: Option<String> = None;
            let mut cursors = HashSet::new();
            let mut models = Vec::new();
            let mut ids = HashSet::new();
            for id in 2..102 {
                send(
                    &mut input,
                    json!({"id":id,"method":"model/list","params":{"limit":100,"includeHidden":false,"cursor":cursor}}),
                )?;
                let page: ModelPage = serde_json::from_value(receive(id)?)
                    .map_err(|e| format!("invalid model catalog: {e}"))?;
                for model in page.data {
                    if !model.hidden && !model.model.is_empty() && ids.insert(model.model.clone()) {
                        models.push(ModelInfo {
                            id: model.model,
                            display_name: model.display_name,
                            is_default: model.is_default,
                        });
                    }
                }
                match page.next_cursor {
                    None => return Ok(models),
                    Some(next) if cursors.insert(next.clone()) => cursor = Some(next),
                    Some(_) => return Err("model discovery returned a repeated cursor".into()),
                }
            }
            Err("model discovery exceeded pagination limit".into())
        })();
        result.map_err(|e| format!("{e}\nstderr tail:\n{}", stderr_tail(&mut stderr)))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelPage {
    data: Vec<DiscoveredModel>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveredModel {
    model: String,
    display_name: String,
    #[serde(default)]
    is_default: bool,
    #[serde(default)]
    hidden: bool,
}
