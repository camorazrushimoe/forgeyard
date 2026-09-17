use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{ForgeError, Result};
use crate::lock::{events_lock_path, FileLock};
use crate::paths::project_dir;
use crate::types::{Agent, Hook};

const MAX_LINE: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub ts: String,
    pub project: String,
    pub hook: Hook,
    pub run_id: String,
    pub agent: Agent,
    pub step: String,
    pub status: String,
    pub input: String,
    pub summary: String,
    pub pid: Option<u32>,
    pub duration_s: Option<u64>,
    pub artifact: String,
}

impl Event {
    pub fn new(project: &str, hook: Hook) -> Self {
        Event {
            ts: now_rfc3339(),
            project: project.to_string(),
            hook,
            run_id: String::new(),
            agent: Agent::Forge,
            step: String::new(),
            status: String::new(),
            input: String::new(),
            summary: String::new(),
            pid: None,
            duration_s: None,
            artifact: String::new(),
        }
    }
}

pub fn events_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join("events.jsonl")
}

pub fn new_run_id() -> String {
    let stamp = format_compact(unix_secs());
    format!("{stamp}-{}", four_hex())
}

pub fn append_event(root: &Path, ev: &Event) -> Result<()> {
    let dir = project_dir(root, &ev.project);
    std::fs::create_dir_all(&dir)?;
    let _lock = FileLock::acquire(&events_lock_path(&dir))?;
    let mut line = sanitize_line(encode_event(ev));
    if !line.ends_with('\n') {
        line.push('\n');
    }
    if line.len() > MAX_LINE {
        line.truncate(MAX_LINE - 1);
        line.push('\n');
    }
    let mut f = OpenOptions::new().create(true).append(true).open(events_path(&dir))?;
    f.write_all(line.as_bytes())?;
    f.sync_all()?;
    Ok(())
}

/// Consecutive tech-pm `hook: stop` failures at the tail of the log.
/// A later ok stop resets the streak to 0.
pub const REVIEW_FAIL_CAP: u32 = 3;

pub fn consecutive_review_fails(root: &Path, project: &str) -> u32 {
    let text = std::fs::read_to_string(events_path(&project_dir(root, project))).unwrap_or_default();
    let mut n = 0u32;
    for line in text.lines().rev() {
        let Ok(ev) = decode_event_line(line) else { continue };
        if ev.hook != Hook::Stop || ev.agent != Agent::TechPm {
            continue;
        }
        let bad = ev.status == "fail" || ev.status == "crash" || ev.summary == "outcome_invalid";
        if bad {
            n += 1;
            continue;
        }
        break;
    }
    n
}

pub fn encode_event(ev: &Event) -> String {
    let mut s = String::from("{");
    push_str(&mut s, "ts", &ev.ts);
    push_str(&mut s, "project", &ev.project);
    if !ev.run_id.is_empty() {
        push_str(&mut s, "run_id", &ev.run_id);
    }
    push_str(&mut s, "agent", ev.agent.as_str());
    push_str(&mut s, "hook", ev.hook.as_str());
    if !ev.step.is_empty() {
        push_str(&mut s, "step", &ev.step);
    }
    if !ev.status.is_empty() {
        push_str(&mut s, "status", &ev.status);
    }
    if let Some(p) = ev.pid {
        s.push_str(&format!(",\"pid\":{p}"));
    }
    if let Some(d) = ev.duration_s {
        s.push_str(&format!(",\"duration_s\":{d}"));
    }
    if !ev.artifact.is_empty() {
        push_str(&mut s, "artifact", &ev.artifact);
    }
    if !ev.input.is_empty() {
        push_str(&mut s, "input", &truncate(&sanitize(&ev.input), 240));
    }
    if !ev.summary.is_empty() {
        push_str(&mut s, "summary", &truncate(&sanitize(&ev.summary), 400));
    }
    s.push('}');
    s
}

fn push_str(s: &mut String, k: &str, v: &str) {
    if s.len() > 1 {
        s.push(',');
    }
    s.push('"');
    s.push_str(k);
    s.push_str("\":\"");
    s.push_str(&esc(v));
    s.push('"');
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}...", &s[..n])
    }
}

pub fn sanitize(s: &str) -> String {
    let mut out = s.to_string();
    for prefix in ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_", "sk-", "xoxb-", "xoxp-"] {
        out = redact_prefixed(&out, prefix);
    }
    out = redact_kv(&out, "password");
    out = redact_kv(&out, "token");
    out = redact_kv(&out, "pat");
    out
}
