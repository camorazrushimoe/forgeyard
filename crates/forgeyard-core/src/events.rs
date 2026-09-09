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

/// `YYYYMMDDTHHMMSSZ-` + 4 hex. SPEC.md §7.
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
        format!("{}\u2026", &s[..n])
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

fn redact_prefixed(s: &str, prefix: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(prefix) {
        out.push_str(&rest[..i]);
        out.push_str(prefix);
        out.push_str("REDACTED");
        rest = &rest[i + prefix.len()..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
            .unwrap_or(rest.len());
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn redact_kv(s: &str, key: &str) -> String {
    let needle = format!("{key}=");
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.to_ascii_lowercase().find(&needle) {
        out.push_str(&rest[..i]);
        out.push_str(&rest[i..i + needle.len()]);
        out.push_str("REDACTED");
        rest = &rest[i + needle.len()..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '"' || c == ',')
            .unwrap_or(rest.len());
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn sanitize_line(line: String) -> String {
    if line.contains("ghp_") || line.contains("sk-") || line.contains("github_pat_") {
        sanitize(&line)
    } else {
        line
    }
}

fn unix_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn format_compact(mut secs: u64) -> String {
    let rem = secs % 86400;
    secs /= 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let mut year: i32 = 1970;
    loop {
        let ydays: u64 = if is_leap(year) { 366 } else { 365 };
        if secs >= ydays {
            secs -= ydays;
            year += 1;
        } else {
            break;
        }
    }
    let mdays = [31u64, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for d in mdays {
        if secs >= d {
            secs -= d;
            month += 1;
        } else {
            break;
        }
    }
    let day = secs + 1;
    format!("{year:04}{month:02}{day:02}T{hour:02}{min:02}{sec:02}Z")
}

fn now_rfc3339() -> String {
    let c = format_compact(unix_secs());
    format!("{}-{}-{}T{}:{}:{}Z", &c[0..4], &c[4..6], &c[6..8], &c[9..11], &c[11..13], &c[13..15])
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn four_hex() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("{:04x}", (nanos ^ std::process::id()) & 0xffff)
}

pub fn decode_event_line(line: &str) -> Result<Event> {
    let line = line.trim();
    let hook = str_field(line, "hook")?.parse::<Hook>().map_err(ForgeError::Precondition)?;
    Ok(Event {
        ts: str_field(line, "ts").unwrap_or_default(),
        project: str_field(line, "project")?,
        hook,
        run_id: str_field(line, "run_id").unwrap_or_default(),
        agent: str_field(line, "agent").ok().and_then(|s| s.parse().ok()).unwrap_or(Agent::Forge),
        step: str_field(line, "step").unwrap_or_default(),
        status: str_field(line, "status").unwrap_or_default(),
        input: str_field(line, "input").unwrap_or_default(),
        summary: str_field(line, "summary").unwrap_or_default(),
        pid: num_field(line, "pid"),
        duration_s: num_field(line, "duration_s").map(|n| n as u64),
        artifact: str_field(line, "artifact").unwrap_or_default(),
    })
}

fn str_field(text: &str, key: &str) -> Result<String> {
    let pat = format!("\"{key}\"");
    let Some(i) = text.find(&pat) else {
        return Err(ForgeError::Precondition(format!("missing {key}")));
    };
    let rest = &text[i + pat.len()..];
    let Some(colon) = rest.find(':') else {
        return Err(ForgeError::Precondition(format!("bad {key}")));
    };
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return Err(ForgeError::Precondition(format!("bad string {key}")));
    }
    let mut out = String::new();
    let bytes = rest[1..].as_bytes();
    let mut j = 0;
    while j < bytes.len() {
        match bytes[j] {
            b'"' => return Ok(out),
            b'\\' if j + 1 < bytes.len() => {
                out.push(bytes[j + 1] as char);
                j += 2;
            }
            c => {
                out.push(c as char);
                j += 1;
            }
        }
    }
    Err(ForgeError::Precondition(format!("unterminated {key}")))
}

fn num_field(text: &str, key: &str) -> Option<u32> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    n.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-ev-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    const LINE0: &str = r#"{"ts":"2026-09-09T08:00:00Z","project":"my-app","run_id":"20260909T080012Z-ab12","agent":"forge","hook":"bind","step":"bound","status":"ok","input":"https://github.com/camorazrushimoe/my-app/issues/14","summary":"bound owner/repo=camorazrushimoe/my-app"}"#;

    #[test]
    fn decode_example_bind_line() {
        let e = decode_event_line(LINE0).unwrap();
        assert_eq!(e.project, "my-app");
        assert_eq!(e.hook, Hook::Bind);
        assert_eq!(e.run_id, "20260909T080012Z-ab12");
        assert_eq!(e.agent, Agent::Forge);
    }

    #[test]
    fn run_id_shape() {
        let id = new_run_id();
        assert!(id.contains('T') && id.contains('Z'));
        let parts: Vec<_> = id.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1].len(), 4);
    }

    #[test]
    fn token_in_summary_is_redacted() {
        let mut ev = Event::new("toy", Hook::TokenSet);
        ev.summary = "set llm.default=sk-abcdefghijklmnopqrstuvwxyz".into();
        let line = encode_event(&ev);
        assert!(!line.contains("sk-abcdefghijklmnopqrstuvwxyz"));
        assert!(line.contains("REDACTED"));
    }

    #[test]
    fn append_is_append_only_and_capped() {
        let root = tmp();
        let mut ev = Event::new("toy", Hook::Bind);
        ev.run_id = "20260909T080012Z-ab12".into();
        ev.summary = "ok".into();
        append_event(&root, &ev).unwrap();
        append_event(&root, &ev).unwrap();
        let text = std::fs::read_to_string(events_path(&project_dir(&root, "toy"))).unwrap();
        assert_eq!(text.lines().count(), 2);
    }
}
