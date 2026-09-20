use super::*;

#[test]
fn unknown_provider_is_rejected() {
    let cfg = Config {
        providers: vec!["codxe".into()],
        ..Config::default()
    };
    assert!(make_providers(&cfg)
        .err()
        .unwrap()
        .contains("unknown provider"));
}

#[test]
fn selection_prefix_is_parsed() {
    assert_eq!(
        split_selection("codex::gpt-5.5"),
        (Some("codex"), "gpt-5.5")
    );
    assert_eq!(
        split_selection("opencode::anthropic/claude-sonnet-4"),
        (Some("opencode"), "anthropic/claude-sonnet-4")
    );
    assert_eq!(split_selection("bare-model"), (None, "bare-model"));
    assert_eq!(selection("codex", "gpt-5.5"), "codex::gpt-5.5");
}

#[test]
fn registry_dispatches_to_the_named_provider() {
    struct Fake(&'static str);
    impl Provider for Fake {
        fn name(&self) -> &'static str {
            self.0
        }
        fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
            Ok(Vec::new())
        }
        fn run(&self, _: &str, _: &str, model: &str, _: &Path) -> Result<String, String> {
            Ok(format!("{}:{model}", self.0))
        }
    }
    let providers = Providers::new(vec![
        Arc::new(Fake("codex")),
        Arc::new(Fake("opencode")),
    ]);
    assert_eq!(
        providers
            .run("s", "u", "opencode::anthropic/x", Path::new("."))
            .unwrap(),
        "opencode:anthropic/x"
    );
    assert_eq!(
        providers
            .run("s", "u", "codex::gpt-5", Path::new("."))
            .unwrap(),
        "codex:gpt-5"
    );
    assert!(providers
        .run("s", "u", "missing::x", Path::new("."))
        .unwrap_err()
        .contains("unknown provider"));
}

#[cfg(unix)]
mod subprocess {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    // Serializing executable fixture creation avoids ETXTBSY when another test's
    // concurrent fork temporarily inherits a writable fixture descriptor.
    static SUBPROCESS_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn fake(
        script: &str,
    ) -> (
        tempfile::TempDir,
        CodexProvider,
        std::sync::MutexGuard<'static, ()>,
    ) {
        let guard = SUBPROCESS_TEST
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("codex");
        std::fs::write(&bin, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        let provider = CodexProvider {
            bin: bin.to_str().unwrap().into(),
            args: vec![],
            timeout_secs: 2,
        };
        (dir, provider, guard)
    }

    const OUTPUT_ARG: &str = r#"
while [ "$#" -gt 0 ]; do
    if [ "$1" = '--output-last-message' ]; then shift; output="$1"; fi
    shift
done
"#;

    #[test]
    fn large_prompt_uses_stdin_and_logs_are_preserved() {
        let (dir, provider, _guard) = fake(&format!(
            "{OUTPUT_ARG}\ncat > \"$output\"\necho diagnostic\necho warning >&2"
        ));
        let user = "long prompt ".repeat(100_000);
        for _ in 0..2 {
            let result = provider.run("system", &user, "model", dir.path()).unwrap();
            assert!(result.contains(&user));
        }
        let invocations: Vec<_> = std::fs::read_dir(dir.path().join("invocations"))
            .unwrap()
            .collect();
        assert_eq!(invocations.len(), 2);
        for entry in invocations {
            let path = entry.unwrap().path();
            assert_eq!(
                std::fs::read_to_string(path.join("stdout.log")).unwrap(),
                "diagnostic\n"
            );
            assert_eq!(
                std::fs::read_to_string(path.join("stderr.log")).unwrap(),
                "warning\n"
            );
        }
    }

    #[test]
    fn nonzero_exit_rejects_even_a_final_message() {
        let (dir, provider, _guard) = fake(&format!(
            "{OUTPUT_ARG}\necho partial > \"$output\"\necho failure >&2\nexit 7"
        ));
        let error = provider.run("s", "u", "m", dir.path()).unwrap_err();
        assert!(error.contains("exit status: 7"), "{error}");
        assert!(error.contains("failure"), "{error}");
    }

    #[test]
    fn missing_output_does_not_reuse_previous_output_or_stdout() {
        let (dir, provider, _guard) = fake(&format!(
            r#"{OUTPUT_ARG}
if [ ! -f called ]; then
    touch called
    echo first > "$output"
fi
echo 'not an answer'
"#
        ));
        assert_eq!(provider.run("s", "u", "m", dir.path()).unwrap(), "first");
        std::fs::write(dir.path().join("last_message.txt"), "stale").unwrap();
        assert!(provider
            .run("s", "u", "m", dir.path())
            .unwrap_err()
            .contains("cannot read final response"));
    }

    #[test]
    fn empty_final_message_is_rejected() {
        let (dir, provider, _guard) = fake(&format!("{OUTPUT_ARG}\nprintf '  ' > \"$output\""));
        assert!(provider
            .run("s", "u", "m", dir.path())
            .unwrap_err()
            .contains("empty final response"));
    }

    #[test]
    fn timeout_stops_descendants() {
        let (dir, mut provider, _guard) = fake("(sleep 2; touch survived) &\nwait");
        provider.timeout_secs = 1;
        assert!(provider
            .run("s", "u", "m", dir.path())
            .unwrap_err()
            .contains("timed out"));
        std::thread::sleep(std::time::Duration::from_millis(1300));
        assert!(!dir.path().join("survived").exists());
    }

    #[test]
    fn discovery_handles_handshake_pagination_and_notifications() {
        let (_dir, provider, _guard) = fake(
            r#"
read -r init
case "$init" in *initialize*) ;; *) exit 3;; esac
printf '%s\n' '{"id":1,"result":{}}'
read -r initialized
case "$initialized" in *initialized*) ;; *) exit 4;; esac
read -r first
printf '%s\n' '{"method":"notice"}' '{"id":2,"result":{"data":[{"model":"a","displayName":"A","isDefault":false},{"model":"hidden","displayName":"Hidden","hidden":true}],"nextCursor":"page2"}}'
read -r second
case "$second" in *page2*) ;; *) exit 5;; esac
printf '%s\n' '{"id":3,"result":{"data":[{"model":"b","displayName":"B","isDefault":true}],"nextCursor":null}}'
# Stay alive like a real app-server; the client must shut us down.
sleep 30
"#,
        );
        let models = provider.list_models().unwrap();
        assert_eq!(
            models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert!(models[1].is_default);
    }

    #[test]
    fn discovery_reports_rpc_errors() {
        let (_dir, provider, _guard) = fake("read -r init\nprintf '%s\\n' '{\"id\":1,\"error\":{\"message\":\"unavailable\"}}'\nsleep 30");
        assert!(provider.list_models().unwrap_err().contains("unavailable"));
    }

    #[test]
    fn discovery_has_a_deadline() {
        let (_dir, mut provider, _guard) = fake("sleep 30");
        provider.timeout_secs = 1;
        let started = std::time::Instant::now();
        assert!(provider.list_models().unwrap_err().contains("timed out"));
        assert!(started.elapsed().as_secs() < 3);
    }

    mod opencode {
        use super::*;

        fn fake(
            script: &str,
        ) -> (
            tempfile::TempDir,
            OpencodeProvider,
            std::sync::MutexGuard<'static, ()>,
        ) {
            let guard = SUBPROCESS_TEST
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let dir = tempfile::tempdir().unwrap();
            let bin = dir.path().join("opencode");
            std::fs::write(&bin, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
            let provider = OpencodeProvider {
                bin: bin.to_str().unwrap().into(),
                args: vec![],
                timeout_secs: 2,
            };
            (dir, provider, guard)
        }

        #[test]
        fn spawn_failure_names_the_binary() {
            let dir = tempfile::tempdir().unwrap();
            let provider = OpencodeProvider {
                bin: "/nonexistent/opencode".into(),
                args: vec![],
                timeout_secs: 2,
            };
            let error = provider.run("s", "u", "m", dir.path()).unwrap_err();
            assert!(error.contains("failed to spawn provider"), "{error}");
            assert!(error.contains("/nonexistent/opencode"), "{error}");
            assert!(error.contains("os error 2"), "{error}");
        }

        #[test]
        fn stdin_prompt_becomes_the_message_and_text_parts_the_answer() {
            let (dir, provider, _guard) = fake(
                r#"cat > stdin_received
for arg in "$@"; do printf '%s\n' "$arg"; done > argv_received
printf '%s\n' '{"type":"step_start","part":{"type":"step-start"}}'
printf '%s\n' '{"type":"text","part":{"id":"p1","type":"text","text":"first"}}'
printf '%s\n' '{"type":"tool_use","part":{"type":"tool"}}'
printf '%s\n' '{"type":"text","part":{"id":"p2","type":"text","text":"second"}}'
echo diagnostic
echo warning >&2"#,
            );
            let answer = provider
                .run("sys", "user task", "prov/model", dir.path())
                .unwrap();
            assert_eq!(answer, "first\nsecond");
            assert_eq!(
                std::fs::read_to_string(dir.path().join("stdin_received")).unwrap(),
                session_prompt("sys", "user task")
            );
            let argv = std::fs::read_to_string(dir.path().join("argv_received")).unwrap();
            assert!(argv.contains("--format\njson\n"), "{argv}");
            assert!(argv.contains("-m\nprov/model\n"), "{argv}");
            let invocations: Vec<_> = std::fs::read_dir(dir.path().join("invocations"))
                .unwrap()
                .collect();
            assert_eq!(invocations.len(), 1);
            let entry = invocations[0].as_ref().unwrap().path();
            assert!(std::fs::read_to_string(entry.join("stdout.log"))
                .unwrap()
                .contains("\"text\":\"first\""));
            assert_eq!(
                std::fs::read_to_string(entry.join("stderr.log")).unwrap(),
                "warning\n"
            );
        }

        #[test]
        fn error_events_are_reported_instead_of_exit_codes() {
            let (dir, provider, _guard) = fake(
                r#"printf '%s\n' '{"type":"error","error":{"type":"provider.no-route","message":"Model unavailable: bad/model"}}'
exit 1"#,
            );
            let error = provider.run("s", "u", "m", dir.path()).unwrap_err();
            assert!(error.contains("provider reported an error"), "{error}");
            assert!(error.contains("Model unavailable: bad/model"), "{error}");
            assert!(!error.contains("exit status"), "{error}");
        }

        #[test]
        fn recovered_interruptions_do_not_fail_a_successful_run() {
            // A retried step leaves a stale `aborted` event in the stream even
            // though opencode exits successfully with a full answer.
            let (dir, provider, _guard) = fake(
                r#"printf '%s\n' '{"type":"error","error":{"type":"aborted","message":"Step interrupted"}}'
printf '%s\n' '{"type":"text","part":{"id":"p1","type":"text","text":"answer"}}'"#,
            );
            assert_eq!(provider.run("s", "u", "m", dir.path()).unwrap(), "answer");
        }

        #[test]
        fn other_error_events_fail_even_on_success() {
            let (dir, provider, _guard) = fake(
                r#"printf '%s\n' '{"type":"text","part":{"id":"p1","type":"text","text":"answer"}}'
printf '%s\n' '{"type":"error","error":{"type":"provider.unknown","message":"boom"}}'"#,
            );
            let error = provider.run("s", "u", "m", dir.path()).unwrap_err();
            assert!(error.contains("provider reported an error: boom"), "{error}");
        }

        #[test]
        fn nonzero_exit_without_error_event_is_reported() {
            let (dir, provider, _guard) = fake("echo noise\nexit 7");
            let error = provider.run("s", "u", "m", dir.path()).unwrap_err();
            assert!(error.contains("exit status: 7"), "{error}");
        }

        #[test]
        fn empty_final_response_is_rejected() {
            let (dir, provider, _guard) =
                fake(r#"printf '%s\n' '{"type":"step_finish","part":{"type":"step-finish"}}'"#);
            assert!(provider
                .run("s", "u", "m", dir.path())
                .unwrap_err()
                .contains("empty final response"));
        }

        #[test]
        fn timeout_stops_descendants() {
            let (dir, mut provider, _guard) = fake("(sleep 2; touch survived) &\nwait");
            provider.timeout_secs = 1;
            assert!(provider
                .run("s", "u", "m", dir.path())
                .unwrap_err()
                .contains("timed out"));
            std::thread::sleep(std::time::Duration::from_millis(1300));
            assert!(!dir.path().join("survived").exists());
        }

        #[test]
        fn discovery_reads_lines_and_only_global_flags() {
            let (dir, mut provider, _guard) = fake(
                r#"here=$(dirname "$0")
for arg in "$@"; do printf '%s\n' "$arg"; done > "$here/args_seen"
printf '%s\n' a '' a b"#,
            );
            provider.args = vec![
                "--agent".into(),
                "build".into(),
                "--standalone".into(),
                "--log-level".into(),
                "debug".into(),
                "--auto".into(),
                "--server=http://127.0.0.1:9".into(),
            ];
            let models = provider.list_models().unwrap();
            assert_eq!(
                models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
                ["a", "b"]
            );
            assert!(models.iter().all(|m| !m.is_default));
            let argv = std::fs::read_to_string(dir.path().join("args_seen")).unwrap();
            assert_eq!(
                argv,
                "models\n--standalone\n--log-level\ndebug\n--server=http://127.0.0.1:9\n"
            );
        }

        #[test]
        fn discovery_failure_includes_stderr() {
            let (_dir, provider, _guard) = fake("echo unavailable >&2\nexit 3");
            let error = provider.list_models().unwrap_err();
            assert!(error.contains("exit status: 3"), "{error}");
            assert!(error.contains("unavailable"), "{error}");
        }

        #[test]
        fn discovery_without_models_is_an_error() {
            let (_dir, provider, _guard) = fake("exit 0");
            assert!(provider
                .list_models()
                .unwrap_err()
                .contains("listed no models"));
        }

        #[test]
        fn discovery_has_a_deadline() {
            let (_dir, mut provider, _guard) = fake("sleep 30");
            provider.timeout_secs = 1;
            let started = std::time::Instant::now();
            assert!(provider.list_models().unwrap_err().contains("timed out"));
            assert!(started.elapsed().as_secs() < 3);
        }
    }
}
