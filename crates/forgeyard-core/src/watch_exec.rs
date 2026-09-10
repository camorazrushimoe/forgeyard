use crate::types::Agent;
use crate::watch::{Action, Plan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecOp {
    ForgeRun { agent: Agent, step: &'static str },
    GhMerge { branch: String },
}

pub trait WatchExec {
    fn forge_run(&mut self, project: &str, agent: Agent, step: &str);
    fn gh_merge(&mut self, project: &str, branch: &str);
}

pub fn ops_for(action: &Action, plan: &Plan) -> Option<ExecOp> {
    match action {
        Action::RunTechPm => Some(ExecOp::ForgeRun { agent: Agent::TechPm, step: "review" }),
        Action::RunImplement { .. } => Some(ExecOp::ForgeRun { agent: Agent::Developer, step: "implement" }),
        Action::RunQa { .. } => Some(ExecOp::ForgeRun { agent: Agent::Qa, step: "qa-on-cluster" }),
        Action::MergePr { item } => {
            let branch = plan.items.iter().find(|i| i.id == *item).map(|i| i.branch.clone()).unwrap_or_default();
            Some(ExecOp::GhMerge { branch })
        }
        _ => None,
    }
}

pub fn dispatch(action: &Action, plan: &Plan, project: &str, exec: &mut dyn WatchExec) {
    match ops_for(action, plan) {
        Some(ExecOp::ForgeRun { agent, step }) => exec.forge_run(project, agent, step),
        Some(ExecOp::GhMerge { branch }) => exec.gh_merge(project, &branch),
        None => {}
    }
}

#[derive(Default)]
pub struct RecordingExec {
    pub calls: Vec<String>,
}

impl WatchExec for RecordingExec {
    fn forge_run(&mut self, project: &str, agent: Agent, step: &str) {
        self.calls.push(format!("forge run --project {project} --agent {} --step {step}", agent.as_str()));
    }
    fn gh_merge(&mut self, project: &str, branch: &str) {
        self.calls.push(format!("gh pr merge --repo {project} {branch}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::{ItemStatus, PlanItem};

    fn plan() -> Plan {
        Plan {
            default_branch: "main".into(),
            items: vec![PlanItem {
                id: "1".into(), title: "t".into(), branch: "fy/main-1".into(),
                status: ItemStatus::Implementing, crashes: 0, cycles: 1,
            }],
        }
    }

    #[test]
    fn implement_calls_forge_not_gh() {
        let p = plan();
        let mut ex = RecordingExec::default();
        dispatch(&Action::RunImplement { item: "1".into() }, &p, "toy", &mut ex);
        assert_eq!(ex.calls, vec!["forge run --project toy --agent developer --step implement"]);
    }

    #[test]
    fn merge_calls_gh_not_forge() {
        let p = plan();
        let mut ex = RecordingExec::default();
        dispatch(&Action::MergePr { item: "1".into() }, &p, "toy", &mut ex);
        assert_eq!(ex.calls, vec!["gh pr merge --repo toy fy/main-1"]);
    }

    #[test]
    fn sleep_is_noop() {
        let p = plan();
        let mut ex = RecordingExec::default();
        dispatch(&Action::SleepBusy, &p, "toy", &mut ex);
        assert!(ex.calls.is_empty());
    }
}
