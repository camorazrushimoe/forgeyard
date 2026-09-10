use std::path::{Path, PathBuf};
use std::process::Command;

use crate::bind::{read_project_file, ProjectFile};
use crate::paths::project_dir;
use crate::watch::{PrVerdict, SpecVerdict, WatchIo};

/// Last matching verdict line wins. Ignores the rest of the prose.
pub fn latest_spec_verdict(text: &str) -> Option<SpecVerdict> {
    let mut out = None;
    for line in text.lines() {
        let l = line.trim();
        if l.eq_ignore_ascii_case("Verdict: approve") || l.eq_ignore_ascii_case("Verdict: approve.") {
            out = Some(SpecVerdict::Approve);
        } else if l.eq_ignore_ascii_case("Verdict: reject") || l.eq_ignore_ascii_case("Verdict: needs-changes") {
            out = Some(SpecVerdict::Reject);
        }
    }
    out
}

pub fn latest_pr_verdict(text: &str) -> Option<PrVerdict> {
    let mut out = None;
    for line in text.lines() {
        let l = line.trim();
        if l.eq_ignore_ascii_case("Verdict: merge") {
            out = Some(PrVerdict::Merge);
        } else if l.eq_ignore_ascii_case("Verdict: no-merge") || l.eq_ignore_ascii_case("Verdict: no_merge") {
            out = Some(PrVerdict::NoMerge);
        }
    }
    out
}

/// `gh pr list --json url,number` → open PR exists if the array is non-empty.
pub fn pr_list_has_open(json: &str) -> bool {
    let t = json.trim();
    if t.is_empty() || t == "[]" || t == "null" { return false; }
    t.contains("\"url\"") || t.contains("\"number\"")
}

/// `gh repo view --json defaultBranchRef` or raw name.
pub fn parse_default_branch(json: &str) -> String {
    let t = json.trim();
    if let Some(i) = t.find("\"name\"") {
        let rest = &t[i + 7..];
        if let Some(q1) = rest.find('"') {
            let rest = &rest[q1 + 1..];
            if let Some(q2) = rest.find('"') {
                let name = &rest[..q2];
                if !name.is_empty() { return name.to_string(); }
            }
        }
    }
    if !t.is_empty() && !t.starts_with('{') && !t.starts_with('[') && t.len() < 64 {
        return t.to_string();
    }
    "main".into()
}

pub fn load_binding(root: &Path, project: &str) -> Option<ProjectFile> {
    read_project_file(&project_dir(root, project).join("PROJECT.toml")).ok()
}

pub trait GhRunner {
    fn run(&self, args: &[&str]) -> String;
}

pub struct RealGh;
impl GhRunner for RealGh {
    fn run(&self, args: &[&str]) -> String {
        Command::new("gh").args(args).output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    }
}

/// WatchIo backed by disk spec + gh stdout. No LLM.
pub struct GhWatchIo<'a> {
    pub root: &'a Path,
    pub project: &'a str,
    pub busy: bool,
    pub gh: &'a dyn GhRunner,
    pub comments: String,
    pub pr_json: String,
    pub default_json: String,
}

impl<'a> GhWatchIo<'a> {
    pub fn live(root: &'a Path, project: &'a str, busy: bool, gh: &'a dyn GhRunner) -> Self {
        let bind = load_binding(root, project);
        let (comments, pr_json, default_json) = if let Some(b) = bind {
            let repo = format!("{}/{}", b.owner, b.repo);
            let comments = gh.run(&["api", &format!("repos/{repo}/issues/comments"), "--paginate"]);
            let pr_json = String::new(); // filled per-branch in pr_open
            let default_json = gh.run(&["repo", "view", &repo, "--json", "defaultBranchRef"]);
            (comments, pr_json, default_json)
        } else {
            (String::new(), String::new(), String::new())
        };
        GhWatchIo { root, project, busy, gh, comments, pr_json, default_json }
    }
}

impl WatchIo for GhWatchIo<'_> {
    fn busy(&self) -> bool { self.busy }
    fn spec_present(&self) -> bool {
        std::fs::read_to_string(project_dir(self.root, self.project).join("spec.md"))
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    }
    fn spec_verdict(&self) -> Option<SpecVerdict> {
        latest_spec_verdict(&self.comments)
    }
    fn pr_open(&self, branch: &str) -> bool {
        if let Some(b) = load_binding(self.root, self.project) {
            let repo = format!("{}/{}", b.owner, b.repo);
            let json = self.gh.run(&[
                "pr", "list", "--repo", &repo, "--head", branch, "--state", "open", "--json", "url,number",
            ]);
            return pr_list_has_open(&json);
        }
        pr_list_has_open(&self.pr_json)
    }
    fn pr_verdict(&self, _branch: &str) -> Option<PrVerdict> {
        latest_pr_verdict(&self.comments)
    }
    fn default_branch(&self) -> String {
        parse_default_branch(&self.default_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::bind_project;
    use crate::watch::{Action, Plan};
    use std::env; use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-gh-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p); fs::create_dir_all(&p).unwrap(); p
    }
    struct ScriptGh { comments: String, pr: String, def: String }
    impl GhRunner for ScriptGh {
        fn run(&self, args: &[&str]) -> String {
            if args.contains(&"comments") || args.iter().any(|a| a.contains("comments")) {
                return self.comments.clone();
            }
            if args.contains(&"pr") { return self.pr.clone(); }
            self.def.clone()
        }
    }
    #[test]
    fn latest_verdict_wins() {
        let t = "Verdict: approve\nnoise\nVerdict: reject\n";
        assert_eq!(latest_spec_verdict(t), Some(SpecVerdict::Reject));
        let t = "prose\nVerdict: no-merge\nVerdict: merge\n";
        assert_eq!(latest_pr_verdict(t), Some(PrVerdict::Merge));
    }
    #[test]
    fn empty_pr_list_is_closed() {
        assert!(!pr_list_has_open("[]"));
        assert!(pr_list_has_open("[{\"number\":1,\"url\":\"https://x\"}]"));
    }
    #[test]
    fn default_branch_from_json() {
        assert_eq!(parse_default_branch("{\"defaultBranchRef\":{\"name\":\"master\"}}"), "master");
    }
    #[test]
    fn approve_plus_spec_seeds_plan_not_tech_pm() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        fs::write(project_dir(&root, "toy").join("spec.md"), "# spec\n").unwrap();
        let gh = ScriptGh {
            comments: "Verdict: approve\n".into(),
            pr: "[]".into(),
            def: "{\"defaultBranchRef\":{\"name\":\"main\"}}".into(),
        };
        let io = GhWatchIo { root: &root, project: "toy", busy: false, gh: &gh, comments: gh.comments.clone(), pr_json: gh.pr.clone(), default_json: gh.def.clone() };
        let mut plan = Plan { default_branch: String::new(), items: vec![] };
        assert_eq!(crate::watch::tick(&mut plan, &io), Action::SeedPlan);
    }
}
