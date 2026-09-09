use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static N: AtomicU64 = AtomicU64::new(0);

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn tmp_root() -> PathBuf {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("fy-cli-{}-{}", std::process::id(), n));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn run(root: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(args)
        .env("FORGEYARD_ROOT", root)
        .env_remove("FORGEYARD_PROJECT")
        .output()
        .expect("spawn forge");
    (
        out.status.code().unwrap_or(255),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn unknown_argv_is_usage() {
    let (code, _, err) = run(&tmp_root(), &["nope"]);
    assert_eq!(code, 2);
    assert!(err.contains("not implemented") || err.contains("forge"));
}

#[test]
fn bind_then_project_prints_name() {
    let root = tmp_root();
    let (code, out, err) = run(
        &root,
        &["bind", "https://github.com/camorazrushimoe/my-app/issues/14"],
    );
    assert_eq!((code, err), (0, String::new()), "bind failed: {out}");
    assert!(out.contains("camorazrushimoe/my-app"));
    let (code, out, _) = run(&root, &["project"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "my-app");
}

#[test]
fn require_spec_missing_is_1() {
    let root = tmp_root();
    run(&root, &["bind", "https://github.com/camorazrushimoe/my-app"]);
    let (code, _, err) = run(&root, &["require-spec"]);
    assert_eq!(code, 1);
    assert!(err.contains("spec.md"));
}

#[test]
fn require_spec_ok_when_present() {
    let root = tmp_root();
    run(&root, &["bind", "https://github.com/camorazrushimoe/my-app"]);
    fs::write(root.join("projects/my-app/spec.md"), "# spec\n").unwrap();
    let (code, out, err) = run(&root, &["require-spec"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("present"));
}

#[test]
fn tokens_lists_names_not_values() {
    let (code, out, _) = run(&tmp_root(), &["tokens"]);
    assert_eq!(code, 0);
    assert!(out.contains("github.pat"));
    assert!(!out.contains("ghp_"));
}

#[test]
fn log_empty_ok() {
    let root = tmp_root();
    run(&root, &["bind", "https://github.com/camorazrushimoe/my-app"]);
    let (code, out, _) = run(&root, &["log"]);
    assert_eq!(code, 0);
    assert!(out.is_empty() || out.ends_with('\n'));
}
