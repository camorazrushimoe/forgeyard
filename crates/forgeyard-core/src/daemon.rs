use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::error::{ForgeError, Result};
use crate::tokens::load_tokens;

pub fn watch_pid_path(root: &Path) -> PathBuf { root.join("watch.pid") }
pub fn yard_pid_path(root: &Path) -> PathBuf { root.join("yard.pid") }
pub fn mcp_pid_path(root: &Path) -> PathBuf { root.join("mcp.pid") }
pub fn ngrok_pid_path(root: &Path) -> PathBuf { root.join("ngrok.pid") }
pub fn start_pid_path(root: &Path) -> PathBuf { root.join("fy-start.pid") }
pub fn watch_log_path(root: &Path) -> PathBuf { root.join("watch.log") }
const WATCH_LOG_CAP: u64 = 256 * 1024;

pub fn rotate_watch_log(root: &Path) {
    let p = watch_log_path(root);
    if let Ok(meta) = fs::metadata(&p) { if meta.len() > WATCH_LOG_CAP { let _ = fs::rename(&p, root.join("watch.log.1")); } }
}
pub fn read_pid(path: &Path) -> Option<u32> { fs::read_to_string(path).ok()?.trim().parse().ok() }
pub fn write_pid(path: &Path, pid: u32) -> Result<()> {
    if let Some(dir) = path.parent() { fs::create_dir_all(dir)?; }
    fs::write(path, format!("{pid}\n"))?; Ok(())
}
pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 { return false; }
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(s) => !s.split_whitespace().nth(2).map(|st| st == "Z").unwrap_or(true),
        Err(_) => false,
    }
}
pub fn kill_pid(pid: u32) {
    let _ = Command::new("kill").arg(pid.to_string()).status();
    thread::sleep(Duration::from_millis(50));
    if pid_alive(pid) { let _ = Command::new("kill").args(["-9", &pid.to_string()]).status(); }
}
pub fn stop_daemons(root: &Path) -> Result<()> {
    let me = std::process::id();
    for p in [watch_pid_path(root), yard_pid_path(root), mcp_pid_path(root), ngrok_pid_path(root), start_pid_path(root)] {
        if let Some(pid) = read_pid(&p) { if pid != me { kill_pid(pid); } let _ = fs::remove_file(&p); }
    }
    Ok(())
}
fn which(name: &str) -> bool {
    Command::new("sh").args(["-c", &format!("command -v {name} >/dev/null")]).status().map(|s| s.success()).unwrap_or(false)
}
pub fn runner_missing() -> bool { !which("pi") }
fn spawn_cmd(cmd: &str, args: &[&str], root: &Path) -> Result<u32> {
    spawn_cmd_env(cmd, args, root, &[])
}
fn spawn_cmd_env(cmd: &str, args: &[&str], root: &Path, extra: &[(&str, &str)]) -> Result<u32> {
    let mut c = Command::new(cmd);
    c.args(args).env("FORGEYARD_ROOT", root).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    for (k, v) in extra { c.env(k, v); }
    let child = c.spawn().map_err(|e| ForgeError::Precondition(format!("spawn {cmd}: {e}")))?;
    Ok(child.id())
}
fn spawn_watch(cmd: &str, args: &[&str], root: &Path) -> Result<u32> {
    rotate_watch_log(root);
    let log = OpenOptions::new().create(true).append(true).open(watch_log_path(root)).map_err(|e| ForgeError::Precondition(format!("watch.log: {e}")))?;
    let err = log.try_clone().map_err(|e| ForgeError::Precondition(format!("watch.log: {e}")))?;
    let child = Command::new(cmd).args(args).env("FORGEYARD_ROOT", root).stdin(Stdio::null()).stdout(Stdio::from(log)).stderr(Stdio::from(err)).spawn()
        .map_err(|e| ForgeError::Precondition(format!("spawn {cmd}: {e}")))?;
    Ok(child.id())
}
fn resolve_bin(env_key: &str, name: &str, fallback: &str) -> (String, String) {
    if let Ok(v) = std::env::var(env_key) {
        let arg = std::env::var(env_key.replace("_BIN", "_ARG")).unwrap_or_else(|_| if name == "watch" { "loop".into() } else { "check".into() });
        return (v, arg);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join(name);
            if cand.is_file() {
                let arg = if name == "watch" { "loop" } else { "check" };
                return (cand.to_string_lossy().into_owned(), arg.into());
            }
        }
    }
    if which(name) {
        let arg = if name == "watch" { "loop" } else { "check" };
        return (name.into(), arg.into());
    }
    (fallback.into(), "3600".into())
}
pub fn drain_hook_lines(path: &Path, offset: u64) -> (u64, Vec<String>) {
    let Ok(data) = fs::read(path) else { return (offset, vec![]) };
    if (data.len() as u64) <= offset { return (offset, vec![]); }
    let chunk = &data[offset as usize..];
    let mut lines = Vec::new();
    for line in String::from_utf8_lossy(chunk).split('\n') { if let Some(p) = pretty_event_line(line) { lines.push(p); } }
    (data.len() as u64, lines)
}
pub fn drain_text_lines(path: &Path, offset: u64) -> (u64, Vec<String>) {
    let Ok(data) = fs::read(path) else { return (offset, vec![]) };
    if (data.len() as u64) <= offset { return (offset, vec![]); }
    let chunk = &data[offset as usize..];
    let lines = String::from_utf8_lossy(chunk).split('\n').map(|l| l.trim()).filter(|l| !l.is_empty()).map(|l| crate::events::sanitize(l)).collect();
    (data.len() as u64, lines)
}

#[derive(Debug, Default)]
pub struct StartReport {
    pub watch_pid: u32,
    pub yard_pid: Option<u32>,
    pub mcp_pid: Option<u32>,
    pub notices: Vec<String>,
}
fn resolve_named(env_key: &str, name: &str) -> Option<String> {
    if let Ok(v) = std::env::var(env_key) { return Some(v); }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() { let cand = dir.join(name); if cand.is_file() { return Some(cand.to_string_lossy().into_owned()); } }
    }
    if which(name) { return Some(name.into()); }
    None
}
pub fn start_mcp_and_tunnel(root: &Path, notices: &mut Vec<String>) -> Option<u32> {
    match crate::mcp::ensure_mcp_bind_token(root) {
        Ok(true) => notices.push("mcp: generated bind_token".into()),
        Ok(false) => {}
        Err(e) => notices.push(format!("mcp: down ({e})")),
    }
    let port = crate::mcp::resolve_mcp_port(root);
    let mcp_pid = match resolve_named("FORGEYARD_MCP_BIN", "mcp") {
        None => { notices.push("mcp: missing".into()); None }
        Some(bin) => match spawn_cmd(&bin, &[], root) {
            Ok(pid) => { let _ = write_pid(&mcp_pid_path(root), pid); notices.push(format!("mcp: local http://127.0.0.1:{port}/mcp")); Some(pid) }
            Err(_) => { notices.push("mcp: down".into()); None }
        },
    };
    let t = load_tokens(root).unwrap_or_default();
    let url = t.get("ngrok.url").unwrap_or("");
    let auth = t.get("ngrok.auth_token").unwrap_or("");
    if url.is_empty() && auth.is_empty() { return mcp_pid; }
    if url.is_empty() { notices.push("tunnel: skipped (no ngrok.url)".into()); return mcp_pid; }
    if auth.is_empty() { notices.push("tunnel: skipped (no ngrok.auth_token)".into()); return mcp_pid; }
    let host = url.trim().trim_end_matches('/').trim_start_matches("https://").trim_start_matches("http://");
    match resolve_named("FORGEYARD_NGROK_BIN", "ngrok") {
        None => notices.push("tunnel: missing; install ngrok".into()),
        Some(bin) => match spawn_cmd_env(&bin, &["http", &port.to_string(), "--url", host], root, &[("NGROK_AUTHTOKEN", auth)]) {
            Ok(pid) => { let _ = write_pid(&ngrok_pid_path(root), pid); notices.push(format!("mcp: public {url}/mcp")); }
            Err(_) => notices.push("tunnel: missing; install ngrok".into()),
        },
    }
    mcp_pid
}
pub fn start_children(root: &Path) -> Result<(u32, Option<u32>)> {
    let r = start_factory(root)?; Ok((r.watch_pid, r.yard_pid))
}
pub fn start_factory(root: &Path) -> Result<StartReport> {
    let mut report = StartReport::default();
    let (watch_bin, watch_arg) = resolve_bin("FORGEYARD_WATCH_BIN", "watch", "sleep");
    let wpid = spawn_watch(&watch_bin, &[&watch_arg], root)?;
    write_pid(&watch_pid_path(root), wpid)?;
    report.watch_pid = wpid;
    report.mcp_pid = start_mcp_and_tunnel(root, &mut report.notices);
    let t = load_tokens(root).unwrap_or_default();
    let token = t.get("telegram.bot_token").map(|s| s.to_string())
        .or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|s| !s.is_empty()));
    if token.is_some() {
        let (yard_bin, yard_arg) = resolve_bin("FORGEYARD_YARD_BIN", "yard", "sleep");
        let ypid = spawn_cmd(&yard_bin, &[&yard_arg], root)?;
        write_pid(&yard_pid_path(root), ypid)?; report.yard_pid = Some(ypid);
    }
    Ok(report)
}
pub fn pretty_event_line(raw: &str) -> Option<String> {
    if !raw.contains("\"hook\"") { return None; } Some(raw.trim().to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("fy-d-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p); fs::create_dir_all(&p).unwrap(); p
    }
    #[test]
    fn start_generates_token_and_watch_runs() {
        let root = tmp();
        std::env::set_var("FORGEYARD_WATCH_BIN", "sleep");
        std::env::set_var("FORGEYARD_WATCH_ARG", "60");
        std::env::remove_var("FORGEYARD_MCP_BIN");
        let report = start_factory(&root).unwrap();
        assert!(pid_alive(report.watch_pid));
        assert!(load_tokens(&root).unwrap().get("mcp.bind_token").unwrap().len() >= 16);
        stop_daemons(&root).unwrap();
    }
    #[test]
    fn start_writes_pid_stop_kills() {
        let root = tmp();
        std::env::set_var("FORGEYARD_WATCH_BIN", "sleep");
        std::env::set_var("FORGEYARD_WATCH_ARG", "60");
        let (wpid, yard) = start_children(&root).unwrap();
        assert!(yard.is_none()); assert!(pid_alive(wpid));
        stop_daemons(&root).unwrap();
        let mut gone = false;
        for _ in 0..20 { thread::sleep(Duration::from_millis(50)); if !pid_alive(wpid) { gone = true; break; } kill_pid(wpid); }
        assert!(gone, "watch {wpid} still alive");
    }
    #[test]
    fn pretty_only_hook_lines() {
        assert!(pretty_event_line("{\"hook\":\"intake\"}").is_some());
        assert!(pretty_event_line("noise").is_none());
    }
    #[test]
    fn drain_skips_noise() {
        let root = tmp(); let p = root.join("events.jsonl");
        fs::write(&p, "noise\n{\"hook\":\"intake\"}\n").unwrap();
        let (off, lines) = drain_hook_lines(&p, 0); assert_eq!(lines.len(), 1);
        let (_, more) = drain_hook_lines(&p, off); assert!(more.is_empty());
    }
    #[test]
    fn rotate_watch_log_renames_when_over_cap() {
        let root = tmp(); let p = watch_log_path(&root);
        fs::write(&p, vec![b'x'; (256 * 1024) + 1]).unwrap(); rotate_watch_log(&root);
        assert!(!p.exists()); assert!(root.join("watch.log.1").exists());
    }
    #[test]
    fn drain_text_redacts_tokens() {
        let root = tmp(); let p = root.join("watch.log");
        fs::write(&p, "toy SleepBusy busy\nsk-abcdefghijklmnopqrstuvwxyz leaked\n").unwrap();
        let (_, lines) = drain_text_lines(&p, 0);
        assert_eq!(lines[0], "toy SleepBusy busy"); assert!(lines[1].contains("REDACTED"));
    }
}
