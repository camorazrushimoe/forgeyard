use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{ForgeError, Result};
use crate::paths::project_dir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    Repo,
    Issue,
    Pull,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Repo => "repo",
            SourceKind::Issue => "issue",
            SourceKind::Pull => "pull",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "repo" => Some(SourceKind::Repo),
            "issue" => Some(SourceKind::Issue),
            "pull" => Some(SourceKind::Pull),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub owner: String,
    pub repo: String,
    pub source_url: String,
    pub source_kind: SourceKind,
    pub source_ref: Option<String>,
}

impl Binding {
    pub fn project(&self) -> &str {
        &self.repo
    }
}

/// Parse a GitHub HTTPS or SSH URL. SPEC.md §5.
pub fn parse_github_ref(input: &str) -> Result<Binding> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(ForgeError::Usage("missing github url".into()));
    }

    if let Some(rest) = raw.strip_prefix("git@github.com:") {
        return parse_owner_repo_path(rest, raw);
    }

    let url = raw
        .strip_prefix("https://github.com/")
        .or_else(|| raw.strip_prefix("http://github.com/"))
        .or_else(|| raw.strip_prefix("https://www.github.com/"))
        .ok_or_else(|| ForgeError::Usage(format!("not a github url: {raw}")))?;

    parse_owner_repo_path(url, raw)
}

fn parse_owner_repo_path(path: &str, original: &str) -> Result<Binding> {
    let path = path.trim_end_matches('/');
    let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return Err(ForgeError::Usage(format!(
            "url must contain owner/repo: {original}"
        )));
    }
    let owner = parts.remove(0).to_string();
    let mut repo = parts.remove(0).to_string();
    if let Some(stripped) = repo.strip_suffix(".git") {
        repo = stripped.to_string();
    }
    if !valid_name(&owner) || !valid_name(&repo) {
        return Err(ForgeError::Usage(format!(
            "invalid owner/repo in {original}"
        )));
    }

    let (source_kind, source_ref) = match parts.as_slice() {
        [] => (SourceKind::Repo, None),
        ["issues", n] if is_digits(n) => (SourceKind::Issue, Some((*n).to_string())),
        ["pull", n] if is_digits(n) => (SourceKind::Pull, Some((*n).to_string())),
        ["pulls", n] if is_digits(n) => (SourceKind::Pull, Some((*n).to_string())),
        _ => (SourceKind::Repo, None),
    };

    Ok(Binding {
        owner,
        repo,
        source_url: original.to_string(),
        source_kind,
        source_ref,
    })
}

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    pub project: String,
    pub owner: String,
    pub repo: String,
    pub source_url: String,
    pub source_kind: SourceKind,
    pub source_ref: Option<String>,
    pub bound_at: String,
}

/// Bind `input` under `root/projects/<repo>/PROJECT.toml`.
/// Same owner/repo is idempotent. Different owner/repo → conflict (exit 3).
pub fn bind_project(root: &Path, input: &str) -> Result<ProjectFile> {
    let parsed = parse_github_ref(input)?;
    let dir = project_dir(root, parsed.project());
    fs::create_dir_all(&dir)?;
    let path = dir.join("PROJECT.toml");

    if path.exists() {
        let existing = read_project_file(&path)?;
        if existing.owner != parsed.owner || existing.repo != parsed.repo {
            return Err(ForgeError::Conflict(format!(
                "already bound to {}/{}; refusing {}",
                existing.owner, existing.repo, input
            )));
        }
        return Ok(existing);
    }

    let file = ProjectFile {
        project: parsed.project().to_string(),
        owner: parsed.owner,
        repo: parsed.repo,
        source_url: parsed.source_url,
        source_kind: parsed.source_kind,
        source_ref: parsed.source_ref,
        bound_at: now_rfc3339(),
    };
    fs::write(&path, encode_project_file(&file))?;
    Ok(file)
}

pub fn read_project_file(path: &Path) -> Result<ProjectFile> {
    let text = fs::read_to_string(path)?;
    decode_project_file(&text)
}

pub fn encode_project_file(p: &ProjectFile) -> String {
    let source_ref = p.source_ref.clone().unwrap_or_default();
    format!(
        "project = \"{}\"\nowner = \"{}\"\nrepo = \"{}\"\nsource_url = \"{}\"\nsource_kind = \"{}\"\nsource_ref = \"{}\"\nbound_at = \"{}\"\n",
        escape(&p.project),
        escape(&p.owner),
        escape(&p.repo),
        escape(&p.source_url),
        p.source_kind.as_str(),
        escape(&source_ref),
        escape(&p.bound_at),
    )
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix(secs)
}

fn format_unix(mut secs: u64) -> String {
    let rem = secs % 86400;
    secs /= 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let mut year: i32 = 1970;
    loop {
        let ydays: u64 = if is_leap(year) { 366 } else { 365 };
        if secs >= ydays {
            secs -= ydays;
            year += 1;
        } else {
            break;
        }
    }
    let mdays = [
        31u64,
        if is_leap(year) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1u32;
    for d in mdays {
        if secs >= d {
            secs -= d;
            month += 1;
        } else {
            break;
        }
    }
    let day = secs + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn decode_project_file(text: &str) -> Result<ProjectFile> {
    let mut project = String::new();
    let mut owner = String::new();
    let mut repo = String::new();
    let mut source_url = String::new();
    let mut source_kind = SourceKind::Repo;
    let mut source_ref = None;
    let mut bound_at = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = unquote(v.trim());
        match k {
            "project" => project = v,
            "owner" => owner = v,
            "repo" => repo = v,
            "source_url" => source_url = v,
            "source_kind" => {
                source_kind = SourceKind::parse(&v).unwrap_or(SourceKind::Repo);
            }
            "source_ref" => {
                source_ref = if v.is_empty() { None } else { Some(v) };
            }
            "bound_at" => bound_at = v,
            _ => {}
        }
    }
    if owner.is_empty() || repo.is_empty() {
        return Err(ForgeError::Precondition(
            "PROJECT.toml missing owner/repo".into(),
        ));
    }
    if project.is_empty() {
        project = repo.clone();
    }
    Ok(ProjectFile {
        project,
        owner,
        repo,
        source_url,
        source_kind,
        source_ref,
        bound_at,
    })
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-bind-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parse_https_repo() {
        let b = parse_github_ref("https://github.com/acme/toy").unwrap();
        assert_eq!(b.owner, "acme");
        assert_eq!(b.repo, "toy");
        assert_eq!(b.source_kind, SourceKind::Repo);
    }

    #[test]
    fn parse_git_suffix_and_ssh() {
        let a = parse_github_ref("https://github.com/acme/toy.git").unwrap();
        let b = parse_github_ref("git@github.com:acme/toy.git").unwrap();
        assert_eq!(a.owner, b.owner);
        assert_eq!(a.repo, b.repo);
        assert_eq!(a.source_kind, SourceKind::Repo);
    }

    #[test]
    fn parse_issue_and_pull() {
        let i = parse_github_ref("https://github.com/acme/toy/issues/4").unwrap();
        assert_eq!(i.source_kind, SourceKind::Issue);
        assert_eq!(i.source_ref.as_deref(), Some("4"));
        let p = parse_github_ref("https://github.com/acme/toy/pull/9").unwrap();
        assert_eq!(p.source_kind, SourceKind::Pull);
        assert_eq!(p.source_ref.as_deref(), Some("9"));
    }

    #[test]
    fn bind_writes_project_toml() {
        let root = tmp();
        let f = bind_project(&root, "https://github.com/acme/toy/issues/4").unwrap();
        assert_eq!(f.project, "toy");
        assert_eq!(f.owner, "acme");
        let text = fs::read_to_string(root.join("projects/toy/PROJECT.toml")).unwrap();
        assert!(text.contains("owner = \"acme\""));
        assert!(text.contains("source_kind = \"issue\""));
        assert!(text.contains("source_ref = \"4\""));
    }

    #[test]
    fn bind_same_owner_repo_is_ok() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let again = bind_project(&root, "https://github.com/acme/toy/issues/1").unwrap();
        assert_eq!(again.owner, "acme");
        assert_eq!(again.repo, "toy");
    }

    #[test]
    fn bind_different_owner_is_conflict() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let err = bind_project(&root, "https://github.com/other/toy").unwrap_err();
        assert_eq!(err.exit(), crate::Exit::Conflict);
    }

    #[test]
    fn unix_epoch_formats() {
        assert_eq!(format_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix(1_000_000_000), "2001-09-09T01:46:40Z");
    }
}
