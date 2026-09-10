use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::error::{ForgeError, Result};
use crate::tokens::load_tokens;

pub fn watch_pid_path(root: &Path) -> PathBuf { root.join("watch.pid") }
pub fn yard_pid_path(root: &Path) -> PathBuf { root.join("yard.pid") }
pub fn start_pid_path(root: &Path) -> PathBuf { root.join("fy-start.pid") }

pub fn read_pid(path: &Path) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

pub fn write_pid(path: &Path, pid: u32) -> Result<()> {
    if let Some(dir) = path.parent() { fs::create_dir_all(dir)?; }
    fs::write(path, format!("{pid}\n"))?;
    Ok(())
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
    if pid_alive(pid) {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
    }
}

pub fn stop_daemons(root: &Path) -> Result<()> {
    let me = std::process::id();
    for p in [watch_pid_path(root), yard_pid_path(root), start_pid_path(root)] {
        if let Some(pid) = read_pid(&p) {
            if pid != me { kill_pid(pid); }
            let _ = fs::remove_file(&p);
        }
    }
    Ok(())
}

fn which(name: &str) -> bool {
    Command::new("sh").args(["-c", &format!("command -v {name} >/dev/null")]).status().map(|s| s.success()).unwrap_or(false)
}

pub fn runner_missing() -> bool { !which("pi") }

fn spawn_cmd(cmd: &str, args: &[&str], root: &Path) -> Result<u32> {
    let child = Command::new(cmd)
        .args(args)
        .env("FORGEYARD_ROOT", root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| ForgeError::Precondition(format!("spawn {cmd}: {e}")))?;
    Ok(child.id())
}

pub fn start_children(root: &Path) -> Result<(u32, Option<u32>)> {
    let watch_bin = std::env::var("FORGEYARD_WATCH_BIN").unwrap_or_else(|_| "sleep".into());
    let watch_arg = std::env::var("FORGEYARD_WATCH_ARG").unwrap_or_else(|_| "3600".into());
    let wpid = spawn_cmd(&watch_bin, &[&watch_arg], root)?;
    write_pid(&watch_pid_path(root), wpid)?;
    let yard = {
        let t = load_tokens(root).unwrap_or_default();
        let token = t.get("telegram.bot_token").map(|s| s.to_string())
            .or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|s| !s.is_empty()));
        if token.is_some() {
            let yard_bin = std::env::var("FORGEYARD_YARD_BIN").unwrap_or_else(|_| "sleep".into());
            let yard_arg = std::env::var("FORGEYARD_YARD_ARG").unwrap_or_else(|_| "3600".into());
            let ypid = spawn_cmd(&yard_bin, &[&yard_arg], root)?;
            write_pid(&yard_pid_path(root), ypid)?;
            Some(ypid)
        } else {
            None
        }
    };
    Ok((wpid, yard))
}

pub fn pretty_event_line(raw: &str) -> Option<String> {
    if !raw.contains("\"hook\"") { return None; }
    Some(raw.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-d-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn start_writes_pid_stop_kills() {
        let root = tmp();
        std::env::set_var("FORGEYARD_WATCH_BIN", "sleep");
        std::env::set_var("FORGEYARD_WATCH_ARG", "60");
        let (wpid, yard) = start_children(&root).unwrap();
        assert!(yard.is_none());
        assert!(pid_alive(wpid));
        assert_eq!(read_pid(&watch_pid_path(&root)), Some(wpid));
        stop_daemons(&root).unwrap();
        let mut gone = false;
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(50));
            if !pid_alive(wpid) { gone = true; break; }
            kill_pid(wpid);
        }
        assert!(gone, "watch {wpid} still alive");
    }
    #[test]
    fn pretty_only_hook_lines() {
        assert!(pretty_event_line("{\"hook\":\"intake\"}").is_some());
        assert!(pretty_event_line("noise").is_none());
    }
}
