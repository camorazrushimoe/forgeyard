use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{
    bind_project, current_project, factory_root, fy_do, render_status, Exit, FY_HELP,
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
            None => {
                eprintln!("usage: fy bind <github-url>");
                Exit::Usage.into()
            }
            Some(url) => match bind_project(&root, url) {
                Ok(p) => {
                    println!("{}/{}", p.owner, p.repo);
                    Exit::Ok.into()
                }
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
                Ok(line) => {
                    println!("{line}");
                    Exit::Ok.into()
                }
                Err(e) => fail(&e),
            }
        }
        Some("status") => {
            let project = args.first().cloned().or_else(|| current_project(&root).ok());
            match project {
                None => {
                    eprintln!("usage: fy status <project>");
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
