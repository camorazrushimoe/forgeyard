use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{print_not_implemented, Exit, FY_HELP};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    match cmd.as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            let _ = write!(io::stdout(), "{FY_HELP}");
            Exit::Ok.into()
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
