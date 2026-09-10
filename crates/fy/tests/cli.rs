use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static N: AtomicU64 = AtomicU64::new(0);

fn bin() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_fy")) }

fn tmp() -> PathBuf {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("fy-h-{}-{}", std::process::id(), n));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn run(root: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(bin()).args(args).env("FORGEYARD_ROOT", root).env_remove("FORGEYARD_PROJECT").output().unwrap();
    (o.status.code().unwrap_or(255), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
}

#[test]
fn help_matches_spec() {
    let (code, out, _) = run(&tmp(), &["help"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("fy — forgeyard"));
    assert!(out.contains("fy do URL TEXT"));
    let (code2, out2, _) = run(&tmp(), &["--help"]);
    assert_eq!(code2, 0);
    assert_eq!(out, out2);
}

#[test]
fn unknown_prints_help_exit_2() {
    let (code, _, err) = run(&tmp(), &["wat"]);
    assert_eq!(code, 2);
    assert!(err.contains("fy — forgeyard"));
}

#[test]
fn do_writes_inbox_and_prints_queued() {
    let root = tmp();
    let (code, out, err) = run(&root, &["do", "https://github.com/acme/toy/issues/4", "add", "login"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert_eq!(out.trim(), "queued toy — watch will pick it up");
    let inbox = fs::read_to_string(root.join("projects/toy/inbox.md")).unwrap();
    assert!(inbox.contains("add login"));
    let log = fs::read_to_string(root.join("projects/toy/events.jsonl")).unwrap();
    assert!(log.contains("intake"));
}

#[test]
fn start_writes_pid_stop_kills() {
    let root = tmp();
    let o = Command::new(bin())
        .args(["start"])
        .env("FORGEYARD_ROOT", &root)
        .env("FORGEYARD_START_ONCE", "1")
        .env("FORGEYARD_WATCH_BIN", "sleep")
        .env("FORGEYARD_WATCH_ARG", "60")
        .env_remove("TELEGRAM_BOT_TOKEN")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    let pid_s = fs::read_to_string(root.join("watch.pid")).unwrap();
    let pid: u32 = pid_s.trim().parse().unwrap();
    let alive = Command::new("kill").args(["-0", &pid.to_string()]).status().unwrap().success();
    assert!(alive);
    let stop = Command::new(bin()).args(["stop"]).env("FORGEYARD_ROOT", &root).output().unwrap();
    assert_eq!(stop.status.code(), Some(0));
    std::thread::sleep(std::time::Duration::from_millis(80));
    let alive = Command::new("kill").args(["-0", &pid.to_string()]).status().unwrap().success();
    assert!(!alive);
}
