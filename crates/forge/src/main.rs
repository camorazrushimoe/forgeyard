use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{
    bind_project, build_envelope, classify_stderr, current_project, factory_root, forge_run, hook_start, hook_stop,
    list_rows, load_meta, load_tokens, print_not_implemented, read_event_log, refresh_spec_cache, render_status,
    require_spec, validate_and_store, Agent, Exit, LiveSpecFetcher, RunOpts, StartOpts,
};

const FORGE_HELP: &str = "\
forge — forgeyard kernel

  forge bind URL
  forge project
  forge require-spec
  forge envelope
  forge hook start | hook stop
  forge run --project P --agent ROLE --step STEP
  forge outcome validate
  forge provider-classify
  forge spec-refresh
  forge status | log | tokens
";

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let cmd = if args.is_empty() { None } else { Some(args.remove(0)) };
    let root = factory_root();
    match cmd.as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            let _ = write!(io::stdout(), "{FORGE_HELP}");
            Exit::Ok.into()
        }
        Some("bind") => match args.first() {
            None => usage("forge bind <github-url>"),
            Some(url) => match bind_project(&root, url) {
                Ok(p) => { println!("{}/{}", p.owner, p.repo); Exit::Ok.into() }
                Err(e) => fail(&e),
            },
        },
        Some("project") => match current_project(&root) {
            Ok(p) => { println!("{p}"); Exit::Ok.into() }
            Err(e) => fail(&e),
        },
        Some("require-spec") => {
            let project = args.first().cloned().or_else(|| current_project(&root).ok());
            match project {
                None => usage("forge require-spec <project>"),
                Some(p) => match require_spec(&root, &p) {
                    Ok(()) => { println!("spec present"); Exit::Ok.into() }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("envelope") => {
            let project = flag(&mut args, "--project").or_else(|| args.first().cloned()).or_else(|| current_project(&root).ok());
            let agent = flag(&mut args, "--agent").and_then(|s| s.parse::<Agent>().ok()).unwrap_or(Agent::Forge);
            let step = flag(&mut args, "--step").unwrap_or_default();
            match project {
                None => usage("forge envelope --project P"),
                Some(p) => match load_tokens(&root) {
                    Ok(t) => { print!("{}", build_envelope(&p, agent, &step, "", &t, None)); Exit::Ok.into() }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("hook") => match args.first().map(|s| s.as_str()) {
            Some("start") => {
                args.remove(0);
                let project = flag(&mut args, "--project").or_else(|| current_project(&root).ok());
                let agent = flag(&mut args, "--agent").and_then(|s| s.parse().ok()).unwrap_or(Agent::Forge);
                let step = flag(&mut args, "--step").unwrap_or_else(|| "run".into());
                let workflow = flag(&mut args, "--workflow").unwrap_or_default();
                match project {
                    None => usage("forge hook start --project P --agent ROLE --step STEP"),
                    Some(p) => match hook_start(&root, StartOpts { project: &p, agent, workflow: &workflow, step: &step, input: "" }) {
                        Ok(id) => { println!("{id}"); Exit::Ok.into() }
                        Err(e) => fail(&e),
                    },
                }
            }
            Some("stop") => {
                args.remove(0);
                let project = flag(&mut args, "--project").or_else(|| current_project(&root).ok());
                let agent = flag(&mut args, "--agent").and_then(|s| s.parse().ok()).unwrap_or(Agent::Forge);
                let run_id = flag(&mut args, "--run-id").unwrap_or_default();
                let step = flag(&mut args, "--step").unwrap_or_default();
                let status = flag(&mut args, "--status").unwrap_or_else(|| "ok".into());
                let summary = flag(&mut args, "--summary").unwrap_or_default();
                match project {
                    None => usage("forge hook stop --project P --run-id ID --status ok"),
                    Some(p) => match hook_stop(&root, &p, agent, &run_id, &step, &status, &summary) {
                        Ok(()) => Exit::Ok.into(),
                        Err(e) => fail(&e),
                    },
                }
            }
            _ => usage("forge hook start | hook stop"),
        },
        Some("run") => {
            let project = flag(&mut args, "--project").or_else(|| current_project(&root).ok());
            let agent = flag(&mut args, "--agent").and_then(|s| s.parse().ok()).unwrap_or(Agent::TechPm);
            let step = flag(&mut args, "--step").unwrap_or_else(|| "run".into());
            let workflow = flag(&mut args, "--workflow").unwrap_or_default();
            let task = args.join(" ");
            match project {
                None => usage("forge run --project P --agent ROLE --step STEP"),
                Some(p) => match forge_run(&root, RunOpts { project: p, agent, step, workflow, task, pack_dir: None, path_prefix: None }) {
                    Ok(o) => {
                        println!("{} {}", o.status, o.summary);
                        if o.status == "ok" { Exit::Ok.into() } else { Exit::Precondition.into() }
                    }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("outcome") => match args.first().map(|s| s.as_str()) {
            Some("validate") => {
                args.remove(0);
                let project = flag(&mut args, "--project").or_else(|| current_project(&root).ok());
                let run_id = flag(&mut args, "--run-id").unwrap_or_default();
                let agent = flag(&mut args, "--agent").and_then(|s| s.parse().ok()).unwrap_or(Agent::TechPm);
                let step = flag(&mut args, "--step").unwrap_or_else(|| "review".into());
                let stdout_path = flag(&mut args, "--stdout");
                match (project, stdout_path) {
                    (Some(p), Some(path)) => {
                        let stdout = match std::fs::read_to_string(&path) {
                            Ok(s) => s,
                            Err(e) => { eprintln!("{e}"); return Exit::Io.into(); }
                        };
                        match validate_and_store(&root, &p, &run_id, agent, &step, &stdout) {
                            Ok(_) => { println!("ok"); Exit::Ok.into() }
                            Err(_) => { eprintln!("outcome_invalid"); Exit::Precondition.into() }
                        }
                    }
                    _ => usage("forge outcome validate --project P --run-id ID --agent ROLE --step STEP --stdout PATH"),
                }
            }
            _ => usage("forge outcome validate"),
        },
        Some("provider-classify") => {
            let path = flag(&mut args, "--stderr").or_else(|| args.first().cloned());
            match path {
                None => usage("forge provider-classify --stderr PATH"),
                Some(path) => {
                    let text = std::fs::read_to_string(&path).unwrap_or_default();
                    match classify_stderr(&text) {
                        Some(c) => { println!("{c}"); Exit::Ok.into() }
                        None => Exit::Precondition.into(),
                    }
                }
            }
        }
        Some("spec-refresh") => {
            let project = args.first().cloned().or_else(|| current_project(&root).ok());
            match project {
                None => usage("forge spec-refresh <project>"),
                Some(p) => match refresh_spec_cache(&root, &p, &LiveSpecFetcher) {
                    Ok(src) => {
                        println!("{} {} stale={}", src.branch, src.content_sha256, src.stale);
                        if src.stale { Exit::Precondition.into() } else { Exit::Ok.into() }
                    }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("status") => {
            let project = args.first().cloned().or_else(|| current_project(&root).ok());
            match project {
                None => usage("forge status <project>"),
                Some(p) => match render_status(&root, &p) {
                    Ok(block) => { print!("{block}"); Exit::Ok.into() }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("log") => {
            let project = args.first().cloned().or_else(|| current_project(&root).ok());
            match project {
                None => usage("forge log <project>"),
                Some(p) => match read_event_log(&root, &p) {
                    Ok(text) => { print!("{text}"); Exit::Ok.into() }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("tokens") => {
            if args.first().map(|s| s.as_str()) == Some("export") {
                let _ = args.remove(0);
                let name = flag(&mut args, "--name").unwrap_or_else(|| "llm.default".into());
                match load_tokens(&root) {
                    Ok(t) => { println!("{}", t.get(&name).unwrap_or("")); Exit::Ok.into() }
                    Err(e) => fail(&e),
                }
            } else {
                match load_tokens(&root) {
                    Ok(t) => {
                        let meta = load_meta(&root).unwrap_or_default();
                        print!("{}", list_rows(&t, &meta));
                        Exit::Ok.into()
                    }
                    Err(e) => fail(&e),
                }
            }
        }
        Some(other) => {
            print_not_implemented("forge", other);
            let _ = write!(io::stderr(), "{FORGE_HELP}");
            Exit::Usage.into()
        }
    }
}

fn flag(args: &mut Vec<String>, name: &str) -> Option<String> {
    if let Some(i) = args.iter().position(|a| a == name) {
        args.remove(i);
        if i < args.len() { return Some(args.remove(i)); }
    }
    None
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("usage: {msg}");
    Exit::Usage.into()
}

fn fail(e: &forgeyard_core::ForgeError) -> ExitCode {
    eprintln!("{e}");
    e.exit().into()
}
