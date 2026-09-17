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
