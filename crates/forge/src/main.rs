use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{
    bind_project, current_project, factory_root, list_rows, load_meta, load_tokens,
    print_not_implemented, read_event_log, render_status, require_spec, Exit,
};

const FORGE_HELP: &str = "\
forge — forgeyard kernel

  forge bind URL
  forge project
  forge require-spec
  forge envelope
  forge hook start | hook stop
  forge run --project P --agent ROLE --step STEP
  forge status | log | tokens
";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    let root = factory_root();
    match cmd.as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            let _ = write!(io::stdout(), "{FORGE_HELP}");
            Exit::Ok.into()
        }
        Some("bind") => match args.next() {
            None => {
                eprintln!("usage: forge bind <github-url>");
                Exit::Usage.into()
            }
            Some(url) => match bind_project(&root, &url) {
                Ok(p) => {
                    println!("{}/{}", p.owner, p.repo);
                    Exit::Ok.into()
                }
                Err(e) => fail(&e),
            },
        },
        Some("project") => match current_project(&root) {
            Ok(p) => {
                println!("{p}");
                Exit::Ok.into()
            }
            Err(e) => fail(&e),
        },
        Some("require-spec") => {
            let project = args.next().or_else(|| current_project(&root).ok());
            match project {
                None => {
                    eprintln!("usage: forge require-spec <project>");
                    Exit::Usage.into()
                }
                Some(p) => match require_spec(&root, &p) {
                    Ok(()) => {
                        println!("spec present");
                        Exit::Ok.into()
                    }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("envelope") => match args.next().or_else(|| current_project(&root).ok()) {
            Some(p) => {
                println!("envelope project={p}");
                Exit::Ok.into()
            }
            None => {
                eprintln!("usage: forge envelope <project>");
                Exit::Usage.into()
            }
        },
        Some("status") => {
            let project = args.next().or_else(|| current_project(&root).ok());
            match project {
                None => {
                    eprintln!("usage: forge status <project>");
                    Exit::Usage.into()
                }
                Some(p) => match render_status(&root, &p) {
                    Ok(block) => {
                        print!("{block}");
                        Exit::Ok.into()
                    }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("log") => {
            let project = args.next().or_else(|| current_project(&root).ok());
            match project {
                None => {
                    eprintln!("usage: forge log <project>");
                    Exit::Usage.into()
                }
                Some(p) => match read_event_log(&root, &p) {
                    Ok(text) => {
                        print!("{text}");
                        Exit::Ok.into()
                    }
                    Err(e) => fail(&e),
                },
            }
        }
        Some("tokens") => match load_tokens(&root) {
            Ok(t) => {
                let meta = load_meta(&root).unwrap_or_default();
                print!("{}", list_rows(&t, &meta));
                Exit::Ok.into()
            }
            Err(e) => fail(&e),
        },
        Some(other) => {
            print_not_implemented("forge", other);
            let _ = write!(io::stderr(), "{FORGE_HELP}");
            Exit::Usage.into()
        }
    }
}

fn fail(e: &forgeyard_core::ForgeError) -> ExitCode {
    eprintln!("{e}");
    e.exit().into()
}
