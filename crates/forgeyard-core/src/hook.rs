use std::path::Path;

use crate::error::Result;
use crate::events::{append_event, new_run_id, Event};
use crate::state::{become_busy, become_idle};
use crate::types::{Agent, Hook};

pub struct StartOpts<'a> {
    pub project: &'a str,
    pub agent: Agent,
    pub workflow: &'a str,
    pub step: &'a str,
    pub input: &'a str,
}

pub fn hook_start(root: &Path, opts: StartOpts<'_>) -> Result<String> {
    let run_id = new_run_id();
    become_busy(root, opts.project, opts.agent, opts.workflow, opts.step, &run_id, opts.input)?;
    let mut ev = Event::new(opts.project, Hook::Start);
    ev.run_id = run_id.clone();
    ev.agent = opts.agent;
    ev.step = opts.step.to_string();
    ev.status = "ok".into();
    ev.pid = Some(std::process::id());
    append_event(root, &ev)?;
    Ok(run_id)
}

pub fn hook_stop(
    root: &Path,
    project: &str,
    agent: Agent,
    run_id: &str,
    step: &str,
    status: &str,
    summary: &str,
) -> Result<()> {
    become_idle(root, project)?;
    let mut ev = Event::new(project, Hook::Stop);
    ev.run_id = run_id.to_string();
    ev.agent = agent;
    ev.step = step.to_string();
    ev.status = status.to_string();
    ev.summary = summary.to_string();
    append_event(root, &ev)?;
    Ok(())
}
