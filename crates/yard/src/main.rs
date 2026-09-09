use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{factory_root, panel_preflight, Exit};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = factory_root();
    match args.first().map(|s| s.as_str()) {
        Some("help") | Some("--help") | Some("-h") | None => {
            let _ = writeln!(io::stdout(), "yard — telegram panel. no LLM.");
            Exit::Ok.into()
        }
        Some("check") => match panel_preflight(&root) {
            Ok(()) => { println!("yard ready"); Exit::Ok.into() }
            Err(e) => { eprintln!("{e}"); e.exit().into() }
        },
        Some(other) => {
            eprintln!("yard: `{other}` is not implemented yet");
            Exit::Usage.into()
        }
    }
}
