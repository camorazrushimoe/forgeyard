use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{print_not_implemented, Exit};

fn main() -> ExitCode {
    let cmd = env::args().nth(1);
    match cmd.as_deref() {
        Some("help") | Some("--help") | Some("-h") | None => {
            let _ = writeln!(io::stdout(), "watch — deterministic loop. no LLM.");
            Exit::Ok.into()
        }
        Some(other) => {
            print_not_implemented("watch", other);
            Exit::Usage.into()
        }
    }
}
