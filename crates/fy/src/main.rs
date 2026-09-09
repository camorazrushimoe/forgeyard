use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{
    factory_root, print_not_implemented, render_status, Exit, FY_HELP,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    match cmd.as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            let _ = write!(io::stdout(), "{FY_HELP}");
            Exit::Ok.into()
        }
        Some("status") => {
            let project = args.next().or_else(|| env::var("FORGEYARD_PROJECT").ok());
            match project {
                None => {
                    eprintln!("usage: fy status <project>");
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
        Some(other) => {
            let rest = match args.next() {
                Some(a) => format!("{other} {a}…"),
                None => other.to_string(),
            };
            print_not_implemented("fy", &rest);
            let _ = write!(io::stderr(), "{FY_HELP}");
            Exit::Usage.into()
        }
    }
}
