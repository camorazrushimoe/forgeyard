//! Shared kernel for `fy`, `forge`, `watch`, and `yard`.

pub mod bind;
pub mod envelope;
pub mod error;
pub mod events;
pub mod hook;
pub mod intake;
pub mod lock;
pub mod paths;
pub mod project;
pub mod runner;
pub mod state;
pub mod status;
pub mod tokens;
pub mod types;

pub use bind::{bind_project, parse_github_ref, Binding, ProjectFile, SourceKind};
pub use error::{Exit, ForgeError, Result};
pub use envelope::build_envelope;
pub use events::{append_event, new_run_id, Event};
pub use hook::{hook_start, hook_stop, StartOpts};
pub use intake::fy_do;
pub use runner::{forge_run, RunOpts, RunOutcome};
pub use lock::{events_lock_path, state_lock_path, FileLock};
pub use paths::{default_factory_root, factory_root, install_root, project_dir};
pub use project::{current_project, list_bound_projects, read_event_log, require_spec};
pub use state::{become_busy, become_idle, load_state, save_state, State};
pub use status::render_status;
pub use tokens::{list_rows, load_meta, load_tokens, save_tokens, Meta, Tokens, CANONICAL};
pub use types::{Agent, AgentStatus, Hook};

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
