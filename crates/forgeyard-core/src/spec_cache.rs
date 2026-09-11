use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::{ForgeError, Result};
use crate::events::{append_event, Event};
use crate::gh_facts::{load_binding as load_bind, parse_default_branch};
use crate::paths::project_dir;
use crate::sha256::sha256_hex;
use crate::types::{Agent, Hook};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSource {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub commit_sha: String,
    pub fetched_at: String,
    pub content_sha256: String,
    pub stale: bool,
    pub stale_reason: String,
}

impl SpecSource {
    pub fn usable(&self) -> bool {
        !self.content_sha256.is_empty() && !self.stale
    }
}

pub trait SpecFetcher {
    fn default_ref(&self, owner: &str, repo: &str) -> std::result::Result<(String, String), String>;
    fn fetch_spec(&self, owner: &str, repo: &str, rev: &str) -> std::result::Result<String, String>;
}

pub struct LiveSpecFetcher;

impl SpecFetcher for LiveSpecFetcher {
    fn default_ref(&self, owner: &str, repo: &str) -> std::result::Result<(String, String), String> {
        let repo_s = format!("{owner}/{repo}");
        let view = run_gh(&["repo", "view", &repo_s, "--json", "defaultBranchRef"]);
        let branch = parse_default_branch(&view);
        if branch.is_empty() {
            return Err("default branch unresolved".into());
        }
        let sha_out = run_gh(&["api", &format!("repos/{repo_s}/commits/{branch}"), "--jq", ".sha"]);
        let sha = sha_out.trim().to_string();
        if sha.len() < 7 || sha.contains('{') {
            return Err(format!("commit sha unresolved for {branch}"));
        }
        Ok((branch, sha))
    }

    fn fetch_spec(&self, owner: &str, repo: &str, rev: &str) -> std::result::Result<String, String> {
        let path = format!("repos/{owner}/{repo}/contents/spec.md");
        let out = Command::new("gh")
            .args(["api", "-H", "Accept: application/vnd.github.raw", &format!("{path}?ref={rev}")])
            .output()
            .map_err(|e| format!("gh api failed: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(format!("spec.md fetch failed: {}", err.trim()));
        }
        String::from_utf8(out.stdout).map_err(|_| "spec.md not utf-8".into())
    }
}

fn run_gh(args: &[&str]) -> String {
    Command::new("gh")
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

pub fn spec_source_path(dir: &Path) -> std::path::PathBuf {
    dir.join("spec-source.json")
}

pub fn load_spec_source(root: &Path, project: &str) -> Option<SpecSource> {
    let path = spec_source_path(&project_dir(root, project));
    let text = fs::read_to_string(path).ok()?;
    decode_spec_source(&text)
}

pub fn current_spec_sha(root: &Path, project: &str) -> Option<String> {
    if let Some(src) = load_spec_source(root, project) {
        if !src.content_sha256.is_empty() {
            return Some(src.content_sha256);
        }
    }
    let text = fs::read_to_string(project_dir(root, project).join("spec.md")).ok()?;
    if text.trim().is_empty() {
        return None;
    }
    Some(sha256_hex(text.as_bytes()))
}

pub fn spec_cache_present(root: &Path, project: &str) -> bool {
    fs::read_to_string(project_dir(root, project).join("spec.md"))
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

pub fn spec_cache_stale(root: &Path, project: &str) -> bool {
    match load_spec_source(root, project) {
        Some(s) => s.stale,
        None => false,
    }
}

pub fn refresh_spec_cache(root: &Path, project: &str, fetcher: &dyn SpecFetcher) -> Result<SpecSource> {
    let bind = load_bind(root, project).ok_or_else(|| {
        ForgeError::Precondition(format!("no binding for {project}"))
    })?;
    match fetcher.default_ref(&bind.owner, &bind.repo) {
        Ok((branch, commit)) => match fetcher.fetch_spec(&bind.owner, &bind.repo, &commit) {
            Ok(body) => commit_cache(root, project, &bind.owner, &bind.repo, &branch, &commit, &body),
            Err(reason) => mark_stale_or_block(root, project, &bind.owner, &bind.repo, &reason),
        },
        Err(reason) => mark_stale_or_block(root, project, &bind.owner, &bind.repo, &reason),
    }
}

fn commit_cache(
    root: &Path,
    project: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    commit: &str,
    body: &str,
) -> Result<SpecSource> {
    if body.trim().is_empty() {
        return mark_stale_or_block(root, project, owner, repo, "empty spec.md");
    }
    let dir = project_dir(root, project);
    fs::create_dir_all(&dir)?;
    let content_sha = sha256_hex(body.as_bytes());
    let src = SpecSource {
        owner: owner.into(),
        repo: repo.into(),
        branch: branch.into(),
        commit_sha: commit.into(),
        fetched_at: now_stamp(),
        content_sha256: content_sha,
        stale: false,
        stale_reason: String::new(),
    };
    let tmp_spec = dir.join("spec.md.tmp");
    let tmp_src = dir.join("spec-source.json.tmp");
    fs::write(&tmp_spec, body)?;
    fs::write(&tmp_src, encode_spec_source(&src))?;
    fs::rename(tmp_spec, dir.join("spec.md"))?;
    fs::rename(tmp_src, spec_source_path(&dir))?;
    Ok(src)
}

fn mark_stale_or_block(
    root: &Path,
    project: &str,
    owner: &str,
    repo: &str,
    reason: &str,
) -> Result<SpecSource> {
    let dir = project_dir(root, project);
    let existing = fs::read_to_string(dir.join("spec.md")).ok();
    if existing.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false) {
        let mut src = load_spec_source(root, project).unwrap_or(SpecSource {
            owner: owner.into(),
            repo: repo.into(),
            branch: String::new(),
            commit_sha: String::new(),
            fetched_at: String::new(),
            content_sha256: sha256_hex(existing.unwrap().as_bytes()),
            stale: true,
            stale_reason: reason.into(),
        });
        src.stale = true;
        src.stale_reason = reason.into();
        src.fetched_at = now_stamp();
        fs::create_dir_all(&dir)?;
        fs::write(spec_source_path(&dir), encode_spec_source(&src))?;
        let mut ev = Event::new(project, Hook::Run);
        ev.agent = Agent::Watch;
        ev.summary = format!("spec_refresh_stale:{reason}");
        ev.status = "stale".into();
        let _ = append_event(root, &ev);
        return Ok(src);
    }
    Err(ForgeError::Precondition(format!("spec refresh failed: {reason}")))
}

pub fn encode_spec_source(s: &SpecSource) -> String {
    format!(
        "{{\"owner\":\"{}\",\"repo\":\"{}\",\"branch\":\"{}\",\"commit_sha\":\"{}\",\"fetched_at\":\"{}\",\"content_sha256\":\"{}\",\"stale\":{},\"stale_reason\":\"{}\"}}\n",
        esc(&s.owner), esc(&s.repo), esc(&s.branch), esc(&s.commit_sha),
        esc(&s.fetched_at), esc(&s.content_sha256),
        if s.stale { "true" } else { "false" },
        esc(&s.stale_reason),
    )
}

pub fn decode_spec_source(text: &str) -> Option<SpecSource> {
    Some(SpecSource {
        owner: json_str(text, "owner").unwrap_or_default(),
        repo: json_str(text, "repo").unwrap_or_default(),
        branch: json_str(text, "branch").unwrap_or_default(),
        commit_sha: json_str(text, "commit_sha").unwrap_or_default(),
        fetched_at: json_str(text, "fetched_at").unwrap_or_default(),
        content_sha256: json_str(text, "content_sha256").unwrap_or_default(),
        stale: text.contains("\"stale\":true"),
        stale_reason: json_str(text, "stale_reason").unwrap_or_default(),
    )
}

fn json_str(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let bytes = rest[1..].as_bytes();
    let mut j = 0;
    while j < bytes.len() {
        match bytes[j] {
            b'"' => return Some(out),
            b'\\' if j + 1 < bytes.len() => {
                out.push(bytes[j + 1] as char);
                j += 2;
            }
            c => {
                out.push(c as char);
                j += 1;
            }
        }
    }
    None
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}

fn now_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::bind_project;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-spec-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    struct FakeFetch {
        branch: String,
        sha: String,
        body: std::result::Result<String, String>,
    }
    impl SpecFetcher for FakeFetch {
        fn default_ref(&self, _o: &str, _r: &str) -> std::result::Result<(String, String), String> {
            Ok((self.branch.clone(), self.sha.clone()))
        }
        fn fetch_spec(&self, _o: &str, _r: &str, _rev: &str) -> std::result::Result<String, String> {
            match &self.body {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(e.clone()),
            }
        }
    }
    #[test]
    fn refresh_writes_cache_and_provenance() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let f = FakeFetch { branch: "trunk".into(), sha: "abc1234def".into(), body: Ok("# spec\n".into()) };
        let src = refresh_spec_cache(&root, "toy", &f).unwrap();
        assert_eq!(src.branch, "trunk");
        assert!(!src.stale);
        assert_eq!(src.content_sha256, sha256_hex(b"# spec\n"));
        let disk = fs::read_to_string(project_dir(&root, "toy").join("spec.md")).unwrap();
        assert_eq!(disk, "# spec\n");
        let loaded = load_spec_source(&root, "toy").unwrap();
        assert_eq!(loaded.commit_sha, "abc1234def");
    }
    #[test]
    fn failed_refresh_keeps_cache_and_marks_stale() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let dir = project_dir(&root, "toy");
        fs::write(dir.join("spec.md"), "# old\n").unwrap();
        let f = FakeFetch { branch: "main".into(), sha: "x".into(), body: Err("unreachable".into()) };
        let src = refresh_spec_cache(&root, "toy", &f).unwrap();
        assert!(src.stale);
        assert_eq!(fs::read_to_string(dir.join("spec.md")).unwrap(), "# old\n");
        let log = fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(log.contains("spec_refresh_stale"));
    }
}
