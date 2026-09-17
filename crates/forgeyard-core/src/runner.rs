use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::envelope::{build_envelope, cluster_configured, llm_endpoint, llm_key, needs_cluster};
use crate::error::{ForgeError, Result};
use crate::events::sanitize;
use crate::hook::{hook_start, hook_stop, StartOpts};
use crate::outcome::{outcome_fail_reason, parse_outcome, qa_comment_body, run_dir, validate_and_store, write_outcome_error, Outcome};
use crate::paths::project_dir;
use crate::provider::classify_stderr;
use crate::tokens::{load_tokens, Tokens};
use crate::types::Agent;

const MAX_STREAM: usize = 64 * 1024;

pub struct RunOpts {
    pub project: String,
    pub agent: Agent,
    pub step: String,
    pub workflow: String,
    pub task: String,
    pub pack_dir: Option<PathBuf>,
    pub path_prefix: Option<PathBuf>,
}

pub struct RunOutcome {
    pub run_id: String,
    pub status: String,
    pub summary: String,
}

pub fn forge_run(root: &Path, opts: RunOpts) -> Result<RunOutcome> {
    let tokens = load_tokens(root)?;
    let run_id = hook_start(root, StartOpts {
        project: &opts.project,
        agent: opts.agent,
        workflow: &opts.workflow,
        step: &opts.step,
        input: &opts.task,
    })?;
    write_request(root, &opts, &tokens, &run_id)?;
    let (status, summary) = match preflight(&opts, &tokens) {
        Err(reason) => {
            let _ = hook_stop(root, &opts.project, opts.agent, &run_id, &opts.step, "fail", &reason);
            return Ok(RunOutcome { run_id, status: "fail".into(), summary: reason });
        }
        Ok(()) => run_pi(root, &opts, &tokens, &run_id)?,
    };
    hook_stop(root, &opts.project, opts.agent, &run_id, &opts.step, &status, &summary)?;
    Ok(RunOutcome { run_id, status, summary })
}

fn role_model(tokens: &Tokens, agent: Agent) -> String {
    let key = match agent {
        Agent::TechPm => "llm.tech-pm",
        Agent::Developer => "llm.developer",
        Agent::Qa => "llm.qa",
        _ => "",
    };
    let raw = if key.is_empty() { "" } else { tokens.get(key).unwrap_or("") };
    if looks_like_model(raw) { return raw.to_string(); }
    String::new()
}

fn looks_like_model(v: &str) -> bool {
    let v = v.trim();
    if v.is_empty() || v.len() > 80 { return false; }
    if v.starts_with("sk-") || v.starts_with("ghp_") || v.starts_with("xox") { return false; }
    if v.contains("://") || v.starts_with('/') || v.contains(' ') { return false; }
    v.chars().any(|c| c.is_ascii_alphanumeric())
}

fn write_request(root: &Path, opts: &RunOpts, tokens: &Tokens, run_id: &str) -> Result<()> {
    let dir = run_dir(root, &opts.project, run_id);
    fs::create_dir_all(&dir)?;
    let endpoint = llm_endpoint(tokens).unwrap_or_default();
    let host = endpoint.split("://").nth(1).unwrap_or(&endpoint).split('/').next().unwrap_or(&endpoint).to_string();
    let model = role_model(tokens, opts.agent);
    let body = format!(
        "{{\"role\":\"{}\",\"step\":\"{}\",\"endpoint\":\"{}\",\"host\":\"{}\",\"model\":\"{}\",\"run_id\":\"{}\"}}\n",
        opts.agent.as_str(), esc(&opts.step), esc(&endpoint), esc(&host), esc(&model), esc(run_id)
    );
    fs::write(dir.join("request.json"), body)?;
    Ok(())
}

fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

fn preflight(opts: &RunOpts, tokens: &Tokens) -> std::result::Result<(), String> {
    if which_in(opts.path_prefix.as_deref(), "pi").is_none() { return Err("runner_missing".into()); }
    if llm_endpoint(tokens).is_none() { return Err("llm_endpoint_missing".into()); }
    if needs_cluster(opts.agent, &opts.step) && !cluster_configured(tokens) { return Err("cluster_missing".into()); }
    Ok(())
}

fn review_task(root: &Path, opts: &RunOpts) -> String {
    let mut task = opts.task.clone();
    if opts.agent == Agent::TechPm {
        if let Ok(spec) = fs::read_to_string(project_dir(root, &opts.project).join("spec.md")) {
            if !spec.trim().is_empty() {
                task.push_str("\n\n# spec.md\n");
                task.push_str(&spec);
            }
        }
    }
    task
}

pub fn outcome_text(stdout: &str) -> String {
    if parse_outcome(stdout).is_some() {
        return stdout.to_string();
    }
    let mut pulled = String::new();
    for line in stdout.lines() {
        for key in ["text", "delta"] {
            if let Some(v) = json_str(line, key) {
                pulled.push_str(&v);
                pulled.push('\n');
            }
        }
    }
    if parse_outcome(&pulled).is_some() { pulled } else { stdout.to_string() }
}

fn json_str(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let mut from = 0;
    let i = loop {
        let rest = &text[from..];
        let rel = rest.find(&pat)?;
        let abs = from + rel;
        let after = text[abs + pat.len()..].trim_start();
        if after.starts_with(':') {
            break abs;
        }
        from = abs + pat.len();
    };
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if !rest.starts_with('"') { return None; }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => { if let Some(n) = chars.next() { out.push(n); } }
            _ => out.push(c),
        }
    }
    None
}

fn run_pi(root: &Path, opts: &RunOpts, tokens: &Tokens, run_id: &str) -> Result<(String, String)> {
    let task = review_task(root, opts);
    let prompt = build_envelope(&opts.project, opts.agent, &opts.step, &task, tokens, opts.pack_dir.as_deref());
    let endpoint = llm_endpoint(tokens).unwrap();
    let key = llm_key(tokens);
    let model = role_model(tokens, opts.agent);
    let pi = which_in(opts.path_prefix.as_deref(), "pi").ok_or_else(|| ForgeError::Precondition("runner_missing".into()))?;
    let mut cmd = Command::new(pi);
    cmd.arg("-p").arg("--mode").arg("json");
    if opts.agent == Agent::TechPm {
        cmd.arg("--no-tools");
    }
    if !model.is_empty() {
        cmd.arg("--model").arg(&model);
    }
    cmd.arg(&prompt)
        .env("OPENAI_BASE_URL", &endpoint)
        .env("OPENAI_API_KEY", &key)
        .env("FORGEYARD_PROJECT", &opts.project)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if !model.is_empty() {
        cmd.env("PI_MODEL", &model);
    }
    if needs_cluster(opts.agent, &opts.step) {
        let host = tokens.get("cluster.host").unwrap_or("");
        let user = tokens.get("cluster.user").unwrap_or("");
        let port = tokens.get("cluster.port").unwrap_or("22");
        cmd.env("FORGEYARD_SSH", format!("{user}@{host}:{port}"));
        cmd.env("FORGEYARD_REMOTE", format!("/srv/forgeyard/{}/repo", opts.project));
    }
    let out = cmd.output().map_err(|e| ForgeError::Precondition(format!("pi spawn failed: {e}")))?;
    let stdout = bound_text(&String::from_utf8_lossy(&out.stdout));
    let stderr = bound_text(&sanitize(&String::from_utf8_lossy(&out.stderr)));
    let dir = run_dir(root, &opts.project, run_id);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("stdout.jsonl"), &stdout)?;
    fs::write(dir.join("stderr.log"), &stderr)?;
    let extracted = outcome_text(&stdout);
    let rc = out.status.code().unwrap_or(1);
    let (status, summary) = if !out.status.success() {
        ("crash".into(), format!("pi_exit_{rc}"))
    } else if let Some(class) = classify_stderr(&stderr) {
        ("fail".into(), class)
    } else {
        match validate_and_store(root, &opts.project, run_id, opts.agent, &opts.step, &extracted) {
            Ok(outcome) => match publish_qa_if_needed(root, opts, run_id, &outcome) {
                Ok(()) => ("ok".into(), "ok".into()),
                Err(reason) => ("fail".into(), reason),
            },
            Err(_) => {
                write_outcome_error(root, &opts.project, run_id, &outcome_fail_reason(&extracted), &extracted);
                ("fail".into(), "outcome_invalid".into())
            }
        }
    };
    if opts.step.contains("implement") {
        if let Some(gh) = which_in(opts.path_prefix.as_deref(), "gh") {
            let _ = Command::new(gh).args(["pr", "list", "--json", "number,url,title"]).output();
        }
    }
    Ok((status, summary))
}

fn publish_qa_if_needed(root: &Path, opts: &RunOpts, run_id: &str, outcome: &Outcome) -> std::result::Result<(), String> {
    let Outcome::Qa { pr, .. } = outcome else { return Ok(()); };
    let Some(body) = qa_comment_body(outcome, run_id) else { return Ok(()); };
    let Some(bind) = crate::gh_facts::load_binding(root, &opts.project) else { return Ok(()); };
    let repo = format!("{}/{}", bind.owner, bind.repo);
    let gh = which_in(opts.path_prefix.as_deref(), "gh").ok_or_else(|| "qa_comment_publish".to_string())?;
    match Command::new(gh).args(["pr", "comment", &pr.to_string(), "--repo", &repo, "--body", &body]).status() {
        Ok(s) if s.success() => Ok(()),
        _ => Err("qa_comment_publish".into()),
    }
}

fn bound_text(s: &str) -> String {
    if s.len() <= MAX_STREAM { s.to_string() } else { format!("{}...\n[truncated]\n", &s[..MAX_STREAM]) }
}

fn which_in(prefix: Option<&Path>, name: &str) -> Option<PathBuf> {
    if let Some(dir) = prefix {
        let p = dir.join(name);
        return if p.is_file() { Some(p) } else { None };
    }
    if let Ok(paths) = env::var("PATH") {
        for dir in env::split_paths(&paths) {
            let p = dir.join(name);
            if p.is_file() { return Some(p); }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::bind_project;
    use crate::events::events_path;
    use crate::sha256::sha256_hex;
    use crate::state::load_state;
    use crate::tokens::save_tokens;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("fy-run-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    fn write_fake(dir: &Path, name: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        let p = dir.join(name);
        fs::write(&p, body).unwrap();
        let mut perm = fs::metadata(&p).unwrap().permissions();
        perm.set_mode(0o755);
        fs::set_permissions(&p, perm).unwrap();
    }
    #[test]
    fn missing_pi_is_runner_missing() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let mut t = Tokens::default();
        t.set("llm.endpoint", "http://127.0.0.1:1/v1").unwrap();
        save_tokens(&root, &t, "t").unwrap();
        fs::create_dir_all(root.join("empty-bin")).unwrap();
        let out = forge_run(&root, RunOpts {
            project: "toy".into(), agent: Agent::TechPm, step: "review".into(),
            workflow: "spec-review".into(), task: "x".into(), pack_dir: None,
            path_prefix: Some(root.join("empty-bin")),
        }).unwrap();
        assert_eq!(out.status, "fail");
        assert_eq!(out.summary, "runner_missing");
        assert!(!load_state(&root, "toy").unwrap().is_busy());
        let log = fs::read_to_string(events_path(&project_dir(&root, "toy"))).unwrap();
        assert!(!log.contains("ghp_"));
        assert!(log.contains("runner_missing"));
    }
    #[test]
    fn outcome_text_pulls_kind_from_pi_event() {
        let raw = r#"{"type":"session"}
{"type":"message_end","message":{"content":[{"type":"text","text":"{\"kind\":\"qa\",\"pr\":3,\"verdict\":\"merge\",\"summary\":\"ok\"}"}]}}"#;
        let flat = outcome_text(raw);
        match parse_outcome(&flat).unwrap() {
            Outcome::Qa { pr, .. } => assert_eq!(pr, 3),
            other => panic!("{other:?}"),
        }
    }
}
