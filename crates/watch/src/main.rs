use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use forgeyard_core::{
    current_project, factory_root, load_state, project_dir, tick, Action, Exit, Plan, WatchIo,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("help") | Some("--help") | Some("-h") | None => {
            let _ = writeln!(io::stdout(), "watch — deterministic loop. no LLM.");
            Exit::Ok.into()
        }
        Some("tick") => {
            let root = factory_root();
            let project = args.get(1).cloned().or_else(|| current_project(&root).ok());
            let Some(p) = project else {
                eprintln!("usage: watch tick <project>");
                return Exit::Usage.into();
            };
            let busy = load_state(&root, &p).map(|s| s.is_busy()).unwrap_or(false);
            let spec_ok = std::fs::read_to_string(project_dir(&root, &p).join("spec.md"))
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            struct Io { busy: bool, spec: bool }
            impl WatchIo for Io {
                fn busy(&self) -> bool { self.busy }
                fn spec_present(&self) -> bool { self.spec }
                fn spec_verdict(&self) -> Option<forgeyard_core::watch::SpecVerdict> { None }
                fn pr_open(&self, _: &str) -> bool { false }
                fn pr_verdict(&self, _: &str) -> Option<forgeyard_core::watch::PrVerdict> { None }
                fn default_branch(&self) -> String { "main".into() }
            }
            let mut plan = forgeyard_core::watch::load_plan(&root, &p)
                .unwrap_or(Plan { default_branch: "main".into(), items: vec![] });
            let act = tick(&mut plan, &Io { busy, spec: spec_ok });
            let _ = forgeyard_core::watch::save_plan(&root, &p, &plan);
            println!("{act:?}");
            if matches!(act, Action::BlockedOnSpec) { Exit::Precondition.into() } else { Exit::Ok.into() }
        }
        Some(other) => {
            eprintln!("watch: `{other}` is not implemented yet");
            Exit::Usage.into()
        }
    }
}
