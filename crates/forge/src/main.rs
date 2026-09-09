use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{print_not_implemented, Exit};

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
        Some(other) => {
            print_not_implemented("forge", other);
            let _ = write!(io::stderr(), "{FORGE_HELP}");
            Exit::Usage.into()
        }
    }
}
