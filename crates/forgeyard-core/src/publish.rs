use std::process::Command;

use crate::gh_facts::pr_list_has_open;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOp {
    Push { repo: String, branch: String },
    PrCreate { repo: String, branch: String, base: String },
}

pub trait Publisher {
    fn push(&mut self, repo: &str, branch: &str);
    fn pr_create(&mut self, repo: &str, branch: &str, base: &str);
    fn pr_open(&self, repo: &str, branch: &str) -> bool;
}

/// After a successful implement: push the branch, create a PR if GitHub has none.
/// Watch does not SSH. PAT stays on the laptop (gh/git local config).
pub fn publish_after_implement(pubr: &mut dyn Publisher, repo: &str, branch: &str, base: &str) -> Vec<PublishOp> {
    let mut done = vec![PublishOp::Push { repo: repo.into(), branch: branch.into() }];
    pubr.push(repo, branch);
    if pubr.pr_open(repo, branch) {
        return done;
    }
    pubr.pr_create(repo, branch, base);
    done.push(PublishOp::PrCreate { repo: repo.into(), branch: branch.into(), base: base.into() });
    done
}

#[derive(Default)]
pub struct RecordingPublisher {
    pub ops: Vec<PublishOp>,
    pub already_open: bool,
}

impl Publisher for RecordingPublisher {
    fn push(&mut self, repo: &str, branch: &str) {
        self.ops.push(PublishOp::Push { repo: repo.into(), branch: branch.into() });
    }
    fn pr_create(&mut self, repo: &str, branch: &str, base: &str) {
        self.ops.push(PublishOp::PrCreate { repo: repo.into(), branch: branch.into(), base: base.into() });
    }
    fn pr_open(&self, _repo: &str, _branch: &str) -> bool { self.already_open }
}

pub struct LivePublisher;

impl Publisher for LivePublisher {
    fn push(&mut self, _repo: &str, branch: &str) {
        let _ = Command::new("git").args(["push", "-u", "origin", branch]).status();
    }
    fn pr_create(&mut self, repo: &str, branch: &str, base: &str) {
        let _ = Command::new("gh").args([
            "pr", "create", "--repo", repo, "--head", branch, "--base", base,
            "--title", branch, "--body", "forgeyard implement",
        ]).status();
    }
    fn pr_open(&self, repo: &str, branch: &str) -> bool {
        let out = Command::new("gh").args([
            "pr", "list", "--repo", repo, "--head", branch, "--state", "open", "--json", "url,number",
        ]).output().ok();
        out.map(|o| pr_list_has_open(&String::from_utf8_lossy(&o.stdout))).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_pr_pushes_and_creates() {
        let mut p = RecordingPublisher { already_open: false, ops: vec![] };
        let done = publish_after_implement(&mut p, "acme/toy", "fy/main-1", "main");
        assert_eq!(done, vec![
            PublishOp::Push { repo: "acme/toy".into(), branch: "fy/main-1".into() },
            PublishOp::PrCreate { repo: "acme/toy".into(), branch: "fy/main-1".into(), base: "main".into() },
        ]);
    }

    #[test]
    fn open_pr_skips_create() {
        let mut p = RecordingPublisher { already_open: true, ops: vec![] };
        let done = publish_after_implement(&mut p, "acme/toy", "fy/main-1", "main");
        assert_eq!(done, vec![PublishOp::Push { repo: "acme/toy".into(), branch: "fy/main-1".into() }]);
        assert!(!p.ops.iter().any(|o| matches!(o, PublishOp::PrCreate { .. })));
    }
}
