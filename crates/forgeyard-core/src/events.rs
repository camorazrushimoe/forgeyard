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
