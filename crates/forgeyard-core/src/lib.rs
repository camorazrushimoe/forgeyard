//! Shared kernel for `fy`, `forge`, `watch`, and `yard`.

pub mod bind;
pub mod cluster;
pub mod daemon;
pub mod envelope;
pub mod error;
pub mod events;
pub mod gh_facts;
pub mod hook;
pub mod intake;
pub mod lock;
pub mod onboard;
pub mod panel;
pub mod paths;
pub mod project;
pub mod publish;
pub mod runner;
pub mod state;
pub mod status;
pub mod tokens;
pub mod types;
pub mod watch;
pub mod watch_exec;

pub use cluster::{bootstrap_commands, remote_path, ssh_target};
pub use daemon::{drain_hook_lines, pretty_event_line, runner_missing, start_children, stop_daemons};
pub use bind::{bind_project, parse_github_ref, Binding, ProjectFile, SourceKind};
pub use error::{Exit, ForgeError, Result};
pub use envelope::build_envelope;
pub use events::{append_event, new_run_id, Event};
pub use gh_facts::{load_binding, GhWatchIo, RealGh};
pub use hook::{hook_start, hook_stop, StartOpts};
pub use intake::fy_do;
pub use runner::{forge_run, RunOpts, RunOutcome};
pub use lock::{events_lock_path, state_lock_path, FileLock};
pub use onboard::{run_wizard, LiveProbe};
pub use panel::{handle as panel_handle, preflight as panel_preflight, PanelState, PANEL_HELP};
pub use paths::{default_factory_root, factory_root, install_root, project_dir};
pub use project::{current_project, list_bound_projects, read_event_log, require_spec};
pub use publish::{publish_after_implement, LivePublisher};
pub use state::{become_busy, become_idle, load_state, save_state, State};
pub use status::render_status;
pub use tokens::{list_rows, load_meta, load_tokens, save_tokens, Meta, Tokens, CANONICAL};
pub use types::{Agent, AgentStatus, Hook};
pub use watch::{tick, Action, Plan, WatchIo};
pub use watch_exec::{dispatch as dispatch_action, RecordingExec, WatchExec};

pub const FY_HELP: &str = "\
fy — forgeyard

  fy help              this text
  fy onboard           telegram, llm, github, ssh cluster
  fy start             run watch + yard, print the event log
  fy stop              stop a running fy start
  fy status            same block as forge status
  fy bind URL          attach a GitHub repo / issue / PR
  fy do URL TEXT       queue work (see spec/intake.md)
";

pub fn print_not_implemented(bin: &str, rest: &str) {
    eprintln!("{bin}: `{rest}` is not implemented yet");
}
