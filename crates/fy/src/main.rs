use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{
    bind_project, current_project, drain_hook_lines, factory_root, fy_do, list_bound_projects,
    project_dir, render_status, run_wizard, runner_missing, start_children, stop_daemons, Exit,
    FY_HELP, LiveProbe,
};

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let cmd = if args.is_empty() { None } else { Some(args.remove(0)) };
    let root = factory_root();
    match cmd.as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            let _ = write!(io::stdout(), "{FY_HELP}");
            Exit::Ok.into()
        }
        Some("bind") => match args.first() {
            None => { eprintln!("usage: fy bind <github-url>"); Exit::Usage.into() }
            Some(url) => match bind_project(&root, url) {
                Ok(p) => { println!("{}/{}", p.owner, p.repo); Exit::Ok.into() }
                Err(e) => fail(&e),
            },
        },
        Some("do") => {
            if args.is_empty() {
                eprintln!("usage: fy do <github-url> <prompt...>");
                return Exit::Usage.into();
            }
            let url = args.remove(0);
            let text = args.join(" ");
            if text.trim().is_empty() {
                eprintln!("usage: fy do <github-url> <prompt...>");
                return Exit::Usage.into();
            }
            match fy_do(&root, &url, &text) {
                Ok(line) => { println!("{line}"); Exit::Ok.into() }
                Err(e) => fail(&e),
            }
        }
        Some("start") => {
            if runner_missing() { println!("runner: missing"); }
            let _ = std::fs::write(root.join("fy-start.pid"), format!("{}\n", std::process::id()));
            match start_children(&root) {
                Ok((w, y)) => {
                    println!("watch pid {w}");
                    if let Some(yp) = y { println!("yard pid {yp}"); } else { println!("yard skipped"); }
                    if env::var("FORGEYARD_START_ONCE").ok().as_deref() == Some("1") {
                        return Exit::Ok.into();
                    }
                    let mut offsets: std::collections::BTreeMap<std::path::PathBuf, u64> = Default::default();
                    loop {
                        if let Ok(names) = list_bound_projects(&root) {
                            for n in names {
                                let path = project_dir(&root, &n).join("events.jsonl");
                                let off = *offsets.get(&path).unwrap_or(&0);
                                let (new_off, lines) = drain_hook_lines(&path, off);
                                offsets.insert(path, new_off);
                                for line in lines { println!("{line}"); }
                            }
                        }
                        if env::var("FORGEYARD_TAIL_ONCE").ok().as_deref() == Some("1") {
                            return Exit::Ok.into();
                        }
                        std::thread::sleep(std::time::Duration::from_millis(400));
                    }
                }
                Err(e) => fail(&e),
            }
        }
        Some("stop") => match stop_daemons(&root) {
            Ok(()) => { println!("stopped"); Exit::Ok.into() }
            Err(e) => fail(&e),
        },
        Some("onboard") => {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            let mut out = io::stdout();
            let mut err = io::stderr();
            match run_wizard(&root, &LiveProbe, &mut input, &mut out, &mut err) {
                Ok(()) => Exit::Ok.into(),
                Err(e) => fail(&e),
            }
        }
        Some("status") => {
            let project = args.first().cloned().or_else(|| current_project(&root).ok());
            match project {
                None => { eprintln!("usage: fy status <project>"); Exit::Usage.into() }
                Some(p) => match render_status(&root, &p) {
                    Ok(block) => { print!("{block}"); Exit::Ok.into() }
                    Err(e) => fail(&e),
                },
            }
        }
        Some(_other) => {
            let _ = write!(io::stderr(), "{FY_HELP}");
            Exit::Usage.into()
        }
    }
}

fn fail(e: &forgeyard_core::ForgeError) -> ExitCode {
    eprintln!("{e}");
    e.exit().into()
}
