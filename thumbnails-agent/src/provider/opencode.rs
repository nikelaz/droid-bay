use super::{
    process::{stderr_tail, ManagedChild},
    session_prompt, ModelInfo, Provider,
};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Seek, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

pub(super) struct OpencodeProvider {
    pub bin: String,
    pub args: Vec<String>,
    pub timeout_secs: u64,
}

/// The assistant's answer from `run --format json` output: one JSON event per
/// line whose `text` events carry message parts. A repeated part id replaces
/// the earlier fragment (streaming updates) in place.
struct RunOutput {
    text: String,
    error: Option<(String, String)>,
}

fn parse_output(path: &Path) -> Result<RunOutput, String> {
    let file = File::open(path).map_err(|e| format!("cannot read final response: {e}"))?;
    let mut parts: Vec<(String, String)> = Vec::new();
    let mut error: Option<(String, String)> = None;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| format!("cannot read final response: {e}"))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Tolerate stray non-JSON output between events.
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match message.get("type").and_then(Value::as_str) {
            Some("text") => {
                let Some(part) = message.get("part") else { continue };
                let id = part
                    .get("id")
                    .or_else(|| part.get("partID"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let text = part.get("text").and_then(Value::as_str).unwrap_or_default();
                match parts.iter_mut().find(|(pid, _)| pid.as_str() == id) {
                    Some(entry) => entry.1 = text.to_owned(),
                    None => parts.push((id.to_owned(), text.to_owned())),
                }
            }
            Some("error") => {
                let payload = message.get("error").unwrap_or(&Value::Null);
                let kind = payload
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned();
                let message = payload
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .to_owned();
                error.get_or_insert((kind, message));
            }
            _ => {}
        }
    }
    Ok(RunOutput {
        text: parts
            .into_iter()
            .map(|(_, text)| text)
            .collect::<Vec<_>>()
            .join("\n"),
        error,
    })
}

impl Provider for OpencodeProvider {
    fn name(&self) -> &'static str {
        "opencode"
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
        let stdout_path = invocation.join("stdout.log");
        let stdout = File::create(&stdout_path).map_err(|e| e.to_string())?;
        let stderr_path = invocation.join("stderr.log");
        let stderr = File::create(&stderr_path).map_err(|e| e.to_string())?;
        // A file-backed stdin avoids pipe backpressure before the timeout starts.
        let mut input = tempfile::tempfile().map_err(|e| e.to_string())?;
        write!(input, "{}", session_prompt(system, user))
            .map_err(|e| format!("cannot write prompt: {e}"))?;
        input.rewind().map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&self.bin);
        cmd.arg("run")
            .args(&self.args)
            // Placed after the user args so a fixed value cannot be overridden.
            .args(["--format", "json", "-m", model])
            .current_dir(&workdir)
            .stdin(input)
            .stdout(stdout)
            .stderr(stderr);
        let result = (|| {
            let mut child = ManagedChild::spawn(&mut cmd)?;
            let status = child.wait(Duration::from_secs(self.timeout_secs))?;
            let output = parse_output(&stdout_path)?;
            // A structured error event explains a failure better than a bare
            // exit code. An `aborted` event is a transient interruption that
            // opencode may recover from; only fatal when the run itself failed.
            if let Some((kind, message)) = &output.error {
                if !status.success() || kind != "aborted" {
                    return Err(format!("provider reported an error: {message}"));
                }
            }
            if !status.success() {
                return Err(format!("provider exited with {status}"));
            }
            let text = output.text.trim();
            if text.is_empty() {
                return Err("provider produced an empty final response".into());
            }
            Ok(text.to_owned())
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
        cmd.arg("models");
        // Only global configuration flags are compatible with the models
        // subcommand; run-specific switches (agent, format, etc.) must not be
        // forwarded.
        let mut args = self.args.iter().peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--server" | "--log-level" => {
                    let value = args
                        .next()
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    cmd.arg(arg).arg(value);
                }
                "--standalone" | "--print-logs" => {
                    cmd.arg(arg);
                }
                other => {
                    if ["--server=", "--log-level="]
                        .iter()
                        .any(|prefix| other.starts_with(prefix))
                    {
                        cmd.arg(other);
                    }
                }
            }
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(stderr.try_clone().map_err(|e| e.to_string())?);
        let result: Result<Vec<ModelInfo>, String> = (|| {
            let mut child = ManagedChild::spawn(&mut cmd)?;
            let output = child.0.stdout.take().ok_or("missing models stdout")?;
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(output).lines() {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
            let deadline = Instant::now() + Duration::from_secs(self.timeout_secs.min(60));
            let mut models: Vec<ModelInfo> = Vec::new();
            let mut ids = HashSet::new();
            loop {
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or("model discovery timed out")?;
                match rx.recv_timeout(remaining) {
                    Ok(Ok(line)) => {
                        let id = line.trim();
                        if !id.is_empty() && ids.insert(id.to_owned()) {
                            models.push(ModelInfo {
                                id: id.to_owned(),
                                display_name: id.to_owned(),
                                is_default: false,
                            });
                        }
                    }
                    Ok(Err(e)) => return Err(format!("reading model catalog: {e}")),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        return Err("model discovery timed out".into())
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let status = child.wait(remaining)?;
                        if !status.success() {
                            return Err(format!(
                                "model discovery failed: provider exited with {status}"
                            ));
                        }
                        if models.is_empty() {
                            return Err("opencode listed no models".into());
                        }
                        return Ok(models);
                    }
                }
            }
        })();
        result.map_err(|e| format!("{e}\nstderr tail:\n{}", stderr_tail(&mut stderr)))
    }
}
