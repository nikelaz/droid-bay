use crate::db::{Db, Run};
use crate::prompts::{
    SYSTEM_CRITIQUE, SYSTEM_FIX, SYSTEM_RESEARCH, USER_CRITIQUE, USER_FIX, USER_HUMAN_FIX,
    USER_RESEARCH,
};
use crate::provider::{extract_json, Providers};
use crate::template::render;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
fn workdir_for(db_path_rel: &str, run_id: &str) -> PathBuf {
    let data_dir = Path::new(db_path_rel).parent().unwrap_or(Path::new("data"));
    data_dir.join("runs").join(run_id)
}

fn base_ctx(run: &Run) -> Value {
    json!({
        "topic": run.topic,
        "summary": run.summary,
    })
}

fn log_prompt(db: &Db, run_id: &str, phase: &str, system: &str, user: &str) {
    db.append_message(
        run_id,
        phase,
        "prompt",
        &format!("[system prompt]\n{system}\n\n[user prompt]\n{user}"),
    );
}

pub fn spawn_run(db: Db, provider: Arc<Providers>, run_id: String, db_path: String) {
    std::thread::spawn(move || execute_run(db, provider, run_id, db_path));
}

fn execute_run(db: Db, provider: Arc<Providers>, run_id: String, db_path: String) {
    let workdir = workdir_for(&db_path, &run_id);

    if !db.cas_status(&run_id, "queued", "running_research") {
        return;
    }

    let run = match db.get_run(&run_id) {
        Ok(Some(r)) => r,
        _ => return,
    };

    db.append_message(
        &run_id,
        "research",
        "event",
        &format!(
            "Run started — research model: {}, critique model: {}, refiner model: {}.",
            run.model_research, run.model_critique, run.model_fix
        ),
    );

    let bibliography = match research_phase(&db, &provider, &run, &workdir) {
        Ok(b) => b,
        Err(e) => {
            db.fail_run(&run_id, &e);
            db.append_message(&run_id, "research", "event", &format!("Run failed: {e}"));
            return;
        }
    };
    db.set_result(&run_id, &bibliography);
    db.set_status(&run_id, "running_critique");

    let bibliography = critique_loop(&db, &provider, &run, &bibliography, &workdir);
    match bibliography {
        Ok(b) => {
            db.set_result(&run_id, &b);
            db.cas_status(&run_id, "running_critique", "review");
            db.append_message(
                &run_id,
                "review",
                "event",
                "Bibliography is ready for your review. Add per-source and/or overall feedback, then Accept the results or send them back to the fixer.",
            );
        }
        Err(e) => {
            db.fail_run(&run_id, &e);
            db.append_message(&run_id, "critique", "event", &format!("Run failed: {e}"));
        }
    }
}

fn research_phase(
    db: &Db,
    provider: &Providers,
    run: &Run,
    workdir: &Path,
) -> Result<String, String> {
    let system = SYSTEM_RESEARCH;
    let user_tpl = USER_RESEARCH;
    let user = render(user_tpl, &base_ctx(run))?;

    log_prompt(db, &run.id, "research", system, &user);
    let out = provider
        .run(system, &user, &run.model_research, workdir)
        .map_err(|e| format!("research model '{}' failed: {e}", run.model_research))?;
    db.append_message(&run.id, "research", "assistant", &out);

    let bibliography = parse_bibliography(&out)?;
    db.append_message(
        &run.id,
        "research",
        "event",
        &format!(
            "Research phase produced a bibliography with {} source(s).",
            bibliography["sources"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0)
        ),
    );
    Ok(serde_json::to_string_pretty(&bibliography).unwrap())
}

fn critique_loop(
    db: &Db,
    provider: &Providers,
    run: &Run,
    bibliography: &str,
    workdir: &Path,
) -> Result<String, String> {
    let system_critique = SYSTEM_CRITIQUE;
    let user_critique_tpl = USER_CRITIQUE;
    let system_fix = SYSTEM_FIX;
    let user_fix_tpl = USER_FIX;

    let mut bibliography = bibliography.to_string();

    for round in 1..=2 {
        let ctx = json!({ "topic": run.topic, "summary": run.summary,
                          "bibliography": bibliography });
        let user = render(user_critique_tpl, &ctx)?;
        log_prompt(db, &run.id, "critique", system_critique, &user);

        let out = provider
            .run(system_critique, &user, &run.model_critique, workdir)
            .map_err(|e| format!("critique model '{}' failed: {e}", run.model_critique))?;
        db.append_message(&run.id, "critique", "assistant", &out);

        let verdict = parse_verdict(&out)?;
        if verdict["verdict"].as_str() == Some("pass") {
            db.append_message(
                &run.id,
                "critique",
                "event",
                &format!("Critique passed on round {round}. No further fixes needed."),
            );
            return Ok(bibliography);
        }

        let issues = format_issues(&verdict);
        db.append_message(
            &run.id,
            "critique",
            "event",
            &format!(
                "Critique flagged {} issue(s) on round {round}; sending them to the fixer.",
                issues.lines().count()
            ),
        );

        db.set_status(&run.id, "running_fix");
        let fix_ctx = json!({ "topic": run.topic, "summary": run.summary,
                              "bibliography": bibliography, "issues": issues });
        let fix_user = render(user_fix_tpl, &fix_ctx)?;
        log_prompt(db, &run.id, "fix", system_fix, &fix_user);

        let fix_out = provider
            .run(system_fix, &fix_user, &run.model_fix, workdir)
            .map_err(|e| format!("fix model '{}' failed: {e}", run.model_fix))?;
        db.append_message(&run.id, "fix", "assistant", &fix_out);

        bibliography = serde_json::to_string_pretty(&parse_bibliography(&fix_out)?).unwrap();
        db.set_result(&run.id, &bibliography);
        db.set_status(&run.id, "running_critique");
        db.append_message(
            &run.id,
            "fix",
            "event",
            "Fixer applied the changes; re-running critique.",
        );
    }

    db.append_message(
        &run.id,
        "critique",
        "event",
        "Maximum critique rounds (2) reached; presenting the bibliography for human review.",
    );
    Ok(bibliography)
}

pub fn spawn_human_fix(db: Db, provider: Arc<Providers>, run_id: String, db_path: String) {
    std::thread::spawn(move || {
        let workdir = workdir_for(&db_path, &run_id);
        if let Err(e) = human_fix_phase(&db, &provider, &run_id, &workdir) {
            db.fail_run(&run_id, &e);
            db.append_message(&run_id, "human_fix", "event", &format!("Run failed: {e}"));
            return;
        }
        db.cas_status(&run_id, "running_fix", "review");
        db.append_message(
            &run_id,
            "review",
            "event",
            "Fixer applied your feedback; the updated bibliography is ready for review again.",
        );
    });
}

fn human_fix_phase(
    db: &Db,
    provider: &Providers,
    run_id: &str,
    workdir: &Path,
) -> Result<(), String> {
    let run = db
        .get_run(run_id)
        .map_err(|e| e.to_string())?
        .ok_or("run not found")?;
    let bibliography = run.result.clone().ok_or("run has no bibliography to fix")?;

    let feedback = db.latest_feedback(run_id).ok_or("no feedback stored")?;
    let mut parts: Vec<String> = Vec::new();
    if !feedback.overall.trim().is_empty() {
        parts.push(format!("Overall feedback: {}", feedback.overall.trim()));
    }
    if let Some(map) = feedback.sources.as_object() {
        for (sid, text) in map {
            if let Some(t) = text.as_str() {
                if !t.trim().is_empty() {
                    parts.push(format!("Feedback for source {sid}: {}", t.trim()));
                }
            }
        }
    }
    if parts.is_empty() {
        return Err("no usable feedback content".into());
    }
    let feedback_text = parts.join("\n");

    let system = SYSTEM_FIX;
    let user_tpl = USER_HUMAN_FIX;
    let ctx = json!({ "topic": run.topic, "summary": run.summary,
                      "bibliography": bibliography, "feedback": feedback_text });
    let user = render(user_tpl, &ctx)?;

    log_prompt(db, run_id, "human_fix", system, &user);
    let out = provider
        .run(system, &user, &run.model_fix, workdir)
        .map_err(|e| format!("fix model '{}' failed: {e}", run.model_fix))?;
    db.append_message(run_id, "human_fix", "assistant", &out);

    let bibliography = parse_bibliography(&out)?;
    db.set_result(
        run_id,
        &serde_json::to_string_pretty(&bibliography).unwrap(),
    );
    Ok(())
}

pub fn parse_bibliography(text: &str) -> Result<Value, String> {
    let raw = extract_json(text).ok_or("no JSON object found in model output")?;
    let mut v: Value =
        serde_json::from_str(&raw).map_err(|e| format!("model output is not valid JSON: {e}"))?;

    if v.is_array() {
        v = json!({ "sources": v });
    }
    let sources = v
        .get("sources")
        .and_then(|s| s.as_array())
        .ok_or("model output does not contain a 'sources' array")?
        .clone();

    let mut cleaned = Vec::new();
    for (i, s) in sources.into_iter().enumerate() {
        let mut s = s;
        if !s.is_object() {
            return Err(format!("source #{i} is not a JSON object"));
        }
        let obj = s.as_object_mut().unwrap();
        let id = obj
            .get("id")
            .and_then(|x| x.as_str())
            .filter(|x| !x.trim().is_empty())
            .map(|x| x.trim().to_string())
            .unwrap_or_else(|| format!("s{}", i + 1));
        obj.insert("id".into(), json!(id));
        cleaned.push(s);
    }
    Ok(json!({ "sources": cleaned }))
}

fn parse_verdict(text: &str) -> Result<Value, String> {
    let raw = extract_json(text).ok_or("critique produced no JSON verdict")?;
    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("critique output is not valid JSON: {e}"))?;
    if v.get("verdict").and_then(|x| x.as_str()).is_none() {
        return Err("critique output lacks a 'verdict' field".into());
    }
    Ok(v)
}

fn format_issues(verdict: &Value) -> String {
    let empty = Vec::new();
    let issues = verdict
        .get("issues")
        .and_then(|x| x.as_array())
        .unwrap_or(&empty);
    if issues.is_empty() {
        "The reviewer did not list specific issues; double-check every source's relevance to the topic.".into()
    } else {
        issues
            .iter()
            .enumerate()
            .map(|(i, x)| format!("{}. {}", i + 1, x.as_str().unwrap_or("(unreadable issue)")))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
