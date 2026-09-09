use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{
    bind_project, factory_root, list_rows, load_meta, load_tokens, print_not_implemented,
    render_status, Exit,
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
            Some(url) => match bind_project(&factory_root(), &url) {
                Ok(p) => {
                    println!("bound {}/{}", p.owner, p.repo);
                    Exit::Ok.into()
                }
                Err(e) => {
                    eprintln!("{e}");
                    e.exit().into()
                }
            },
        },
        Some("status") => {
            let project = args.next().or_else(|| std::env::var("FORGEYARD_PROJECT").ok());
            match project {
                None => {
                    eprintln!("usage: forge status <project>");
                    Exit::Usage.into()
                }
                Some(p) => match render_status(&factory_root(), &p) {
                    Ok(block) => {
                        print!("{block}");
                        Exit::Ok.into()
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        e.exit().into()
                    }
                },
            }
        }
        Some("tokens") => match load_tokens(&factory_root()) {
            Ok(t) => {
                let meta = load_meta(&factory_root()).unwrap_or_default();
                print!("{}", list_rows(&t, &meta));
                Exit::Ok.into()
            }
            Err(e) => {
                eprintln!("{e}");
                e.exit().into()
            }
        },
        Some(other) => {
            print_not_implemented("forge", other);
            let _ = write!(io::stderr(), "{FORGE_HELP}");
            Exit::Usage.into()
        }
    }
}
