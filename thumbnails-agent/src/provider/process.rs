use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

/// Tail of a provider's stderr for error diagnostics.
pub(super) fn stderr_tail(file: &mut File) -> String {
    let mut text = String::new();
    // Bound diagnostic reads even when the harness produces a very large log.
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(len.saturating_sub(8192)));
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_ok() {
        text = String::from_utf8_lossy(&bytes)
            .lines()
            .rev()
            .take(25)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
    }
    text
}

/// Owns the child and its process group, including cleanup on early returns.
pub(super) struct ManagedChild(pub Child);

impl ManagedChild {
    pub fn spawn(cmd: &mut Command) -> Result<Self, String> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd.spawn().map(Self).map_err(|e| {
            format!(
                "failed to spawn provider `{}`: {e}",
                cmd.get_program().to_string_lossy()
            )
        })
    }

    pub fn wait(&mut self, timeout: Duration) -> Result<ExitStatus, String> {
        let started = Instant::now();
        loop {
            if let Some(status) = self
                .0
                .try_wait()
                .map_err(|e| format!("waiting for provider failed: {e}"))?
            {
                return Ok(status);
            }
            if started.elapsed() >= timeout {
                return Err(format!("provider timed out after {}s", timeout.as_secs()));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        #[cfg(unix)]
        // SAFETY: the child starts a new process group whose ID is its PID.
        // A negative PID targets that group, including ordinary descendants.
        unsafe {
            libc::kill(-(self.0.id() as i32), libc::SIGKILL);
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &self.0.id().to_string(), "/T", "/F"])
                .output();
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
