use std::env;
use std::io::{self, Write};
use std::process::{Command, ExitCode};

use forgeyard_core::{
    current_project, dispatch_action, factory_root, list_bound_projects, load_binding, load_state,
    publish_after_implement, tick, Action, Agent, Exit, GhWatchIo, LivePublisher, Plan, RealGh,
    WatchExec,
};

struct ShellExec;
impl WatchExec for ShellExec {
    fn forge_run(&mut self, project: &str, agent: Agent, step: &str) {
        let status = Command::new("forge")
            .args(["run", "--project", project, "--agent", agent.as_str(), "--step", step])
            .status();
        if status.is_err() {
            eprintln!("watch: forge run skipped ({project} {agent} {step})");
        }
    }
    fn gh_merge(&mut self, repo: &str, branch: &str) {
        let status = Command::new("gh")
            .args(["pr", "merge", "--repo", repo, branch, "--merge"])
            .status();
        if status.is_err() {
            eprintln!("watch: gh pr merge skipped ({repo} {branch})");
        }
    }
}

fn tick_one(root: &std::path::Path, p: &str) -> Action {
    let busy = load_state(root, p).map(|s| s.is_busy()).unwrap_or(false);
    let gh = RealGh;
    let io = GhWatchIo::live(root, p, busy, &gh);
    let mut plan = forgeyard_core::watch::load_plan(root, p)
        .unwrap_or(Plan { default_branch: "main".into(), items: vec![] });
    let repo = load_binding(root, p)
        .map(|b| format!("{}/{}", b.owner, b.repo))
        .unwrap_or_else(|| p.to_string());
    let act = tick(&mut plan, &io);
    dispatch_action(&act, &plan, p, &repo, &mut ShellExec);
    if let Action::RunImplement { item } = &act {
        if let Some(b) = load_binding(root, p) {
            let repo = format!("{}/{}", b.owner, b.repo);
            let branch = plan.items.iter().find(|i| i.id == *item).map(|i| i.branch.as_str()).unwrap_or("");
            let base = if plan.default_branch.is_empty() { "main" } else { plan.default_branch.as_str() };
            if !branch.is_empty() {
                let _ = publish_after_implement(&mut LivePublisher, &repo, branch, base);
            }
        }
    }
    let _ = forgeyard_core::watch::save_plan(root, p, &plan);
    act
}

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
            let act = tick_one(&root, &p);
            println!("{act:?}");
            if matches!(act, Action::BlockedOnSpec) { Exit::Precondition.into() } else { Exit::Ok.into() }
        }
        Some("loop") => {
            let root = factory_root();
            loop {
                if let Ok(names) = list_bound_projects(&root) {
                    for p in names {
                        let _ = tick_one(&root, &p);
                    }
                }
                if env::var("FORGEYARD_WATCH_ONCE").ok().as_deref() == Some("1") { break; }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Exit::Ok.into()
        }
        Some(other) => {
            eprintln!("watch: `{other}` is not implemented yet");
            Exit::Usage.into()
        }
    }
}
