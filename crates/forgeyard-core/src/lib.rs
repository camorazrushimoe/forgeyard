//! Shared kernel for `fy`, `forge`, `watch`, and `yard`.
//!
//! Disk writes and the watch machine land in later issues. This crate is the
//! typed vocabulary the binaries share.

pub mod bind;
pub mod error;
pub mod events;
pub mod lock;
pub mod paths;
pub mod state;
pub mod status;
pub mod tokens;
pub mod types;

pub use bind::{bind_project, parse_github_ref, Binding, ProjectFile, SourceKind};
pub use error::{Exit, ForgeError, Result};
pub use events::{append_event, new_run_id, Event};
pub use lock::{events_lock_path, state_lock_path, FileLock};
pub use paths::{default_factory_root, factory_root, install_root, project_dir};
pub use state::{become_busy, become_idle, load_state, save_state, State};
pub use status::render_status;
pub use tokens::{list_rows, load_meta, load_tokens, save_tokens, Meta, Tokens, CANONICAL};
pub use types::{Agent, AgentStatus, Hook};

/// Exact `fy help` text from spec/cli.md.
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
