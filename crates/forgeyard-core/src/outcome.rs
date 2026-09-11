use std::fs;
use std::path::Path;

use crate::error::{ForgeError, Result};
use crate::paths::project_dir;
use crate::spec_cache::current_spec_sha;
use crate::types::Agent;
use crate::watch::{PrVerdict, SpecVerdict};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    SpecReview { spec_sha256: String, verdict: SpecVerdict, summary: String },
    Implementation { branch: String, summary: String },
    Qa { pr: u32, verdict: PrVerdict, summary: String },
}

impl Outcome {
    pub fn kind(&self) -> &'static str {
        match self {
            Outcome::SpecReview { .. } => "spec_review",
            Outcome::Implementation { .. } => "implementation",
            Outcome::Qa { .. } => "qa",
        }
    }
}

pub fn run_dir(root: &Path, project: &str, run_id: &str) -> std::path::PathBuf {
    project_dir(root, project).join("runs").join(run_id)
}

pub fn parse_outcome(text: &str) -> Option<Outcome> {
    let raw = extract_outcome_json(text)?;
    parse_outcome_object(&raw)
}

pub fn extract_outcome_json(text: &str) -> Option<String> {
    let mut found = None;
    for line in text.lines() {
        let t = line.trim();
        if t.contains("\"kind\"") && t.contains('{') {
            if let Some(obj) = first_object(t) {
                if obj.contains("\"kind\"") {
                    found = Some(obj);
                }
            }
        }
    }
    if found.is_none() {
        if let Some(obj) = last_object_with_kind(text) {
            found = Some(obj);
        }
    }
    found
}

fn first_object(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn last_object_with_kind(text: &str) -> Option<String> {
    let mut last = None;
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(obj) = first_object(&text[i..]) {
                if obj.contains("\"kind\"") {
                    last = Some(obj.clone());
                }
                i += obj.len().max(1);
                continue;
            }
        }
        i += 1;
    }
    last
}

pub fn parse_outcome_object(raw: &str) -> Option<Outcome> {
    let kind = json_str(raw, "kind")?;
    match kind.as_str() {
        "spec_review" => {
            let verdict = match json_str(raw, "verdict")?.as_str() {
                "approve" => SpecVerdict::Approve,
                "needs_changes" | "needs-changes" | "reject" => SpecVerdict::Reject,
                _ => return None,
            };
            Some(Outcome::SpecReview {
                spec_sha256: json_str(raw, "spec_sha256").unwrap_or_default(),
                verdict,
                summary: json_str(raw, "summary").unwrap_or_default(),
            })
        }
        "implementation" => Some(Outcome::Implementation {
            branch: json_str(raw, "branch")?,
            summary: json_str(raw, "summary").unwrap_or_default(),
        }),
        "qa" => {
            let verdict = match json_str(raw, "verdict")?.as_str() {
                "merge" => PrVerdict::Merge,
                "no_merge" | "no-merge" => PrVerdict::NoMerge,
                _ => return None,
            };
            Some(Outcome::Qa {
                pr: json_u32(raw, "pr")?,
                verdict,
                summary: json_str(raw, "summary").unwrap_or_default(),
            })
        }
        _ => None,
    }
}

pub fn encode_outcome(o: &Outcome) -> String {
    match o {
        Outcome::SpecReview { spec_sha256, verdict, summary } => {
            let v = match verdict {
                SpecVerdict::Approve => "approve",
                SpecVerdict::Reject => "needs_changes",
            };
            format!(
                "{{\"kind\":\"spec_review\",\"spec_sha256\":\"{}\",\"verdict\":\"{}\",\"summary\":\"{}\"}}\n",
                esc(spec_sha256), v, esc(summary)
            )
        }
        Outcome::Implementation { branch, summary } => format!(
            "{{\"kind\":\"implementation\",\"branch\":\"{}\",\"summary\":\"{}\"}}\n",
            esc(branch), esc(summary)
        ),
        Outcome::Qa { pr, verdict, summary } => {
            let v = match verdict {
                PrVerdict::Merge => "merge",
                PrVerdict::NoMerge => "no_merge",
            };
            format!(
                "{{\"kind\":\"qa\",\"pr\":{pr},\"verdict\":\"{v}\",\"summary\":\"{}\"}}\n",
                esc(summary)
            )
        }
    }
}

pub fn expected_kind(agent: Agent, step: &str) -> Option<&'static str> {
    match agent {
        Agent::TechPm => Some("spec_review"),
        Agent::Developer => Some("implementation"),
        Agent::Qa => Some("qa"),
        _ => {
            if step.contains("review") { Some("spec_review") }
            else if step.contains("implement") { Some("implementation") }
            else if step.contains("qa") { Some("qa") }
            else { None }
        }
    }
}

pub fn validate_outcome(agent: Agent, step: &str, outcome: &Outcome, spec_sha: Option<&str>) -> Result<()> {
    let want = expected_kind(agent, step).ok_or_else(|| {
        ForgeError::Precondition("outcome not required for this role".into())
    })?;
    if outcome.kind() != want {
        return Err(ForgeError::Precondition(format!("outcome kind {} != {want}", outcome.kind())));
    }
    if let Outcome::SpecReview { spec_sha256, .. } = outcome {
        if spec_sha256.is_empty() {
            return Err(ForgeError::Precondition("missing spec_sha256".into()));
        }
        if let Some(cur) = spec_sha {
            if spec_sha256 != cur {
                return Err(ForgeError::Precondition("stale-spec outcome".into()));
            }
        }
    }
    if let Outcome::Implementation { branch, .. } = outcome {
        if branch.trim().is_empty() {
            return Err(ForgeError::Precondition("missing branch".into()));
        }
    }
    Ok(())
}

pub fn validate_and_store(
    root: &Path, project: &str, run_id: &str, agent: Agent, step: &str, stdout: &str,
) -> Result<Outcome> {
    let parsed = parse_outcome(stdout).ok_or_else(|| ForgeError::Precondition("outcome_invalid".into()))?;
    let sha = current_spec_sha(root, project);
    validate_outcome(agent, step, &parsed, sha.as_deref())?;
    let dir = run_dir(root, project, run_id);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("outcome.json"), encode_outcome(&parsed))?;
    Ok(parsed)
}

pub fn load_outcome(root: &Path, project: &str, run_id: &str) -> Option<Outcome> {
    let text = fs::read_to_string(run_dir(root, project, run_id).join("outcome.json")).ok()?;
    parse_outcome_object(text.trim())
}

pub fn latest_outcomes(root: &Path, project: &str) -> Vec<(String, Outcome)> {
    let dir = project_dir(root, project).join("runs");
    let Ok(rd) = fs::read_dir(&dir) else { return vec![]; };
    let mut ids: Vec<String> = rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().into_string().ok()).collect();
    ids.sort();
    let mut out = Vec::new();
    for id in ids {
        if let Some(o) = load_outcome(root, project, &id) {
            out.push((id, o));
        }
    }
    out
}

pub fn latest_spec_outcome(root: &Path, project: &str, spec_sha: &str) -> Option<SpecVerdict> {
    latest_outcomes(root, project).into_iter().rev().find_map(|(_, o)| match o {
        Outcome::SpecReview { spec_sha256, verdict, .. } if spec_sha256 == spec_sha => Some(verdict),
        _ => None,
    })
}

pub fn latest_qa_outcome(root: &Path, project: &str, pr: Option<u32>) -> Option<PrVerdict> {
    latest_outcomes(root, project).into_iter().rev().find_map(|(_, o)| match o {
        Outcome::Qa { pr: n, verdict, .. } => {
            if let Some(want) = pr { if n == want { Some(verdict) } else { None } } else { Some(verdict) }
        }
        _ => None,
    })
}

pub fn qa_comment_body(outcome: &Outcome, run_id: &str) -> Option<String> {
    match outcome {
        Outcome::Qa { verdict, summary, .. } => {
            let line = match verdict {
                PrVerdict::Merge => "Verdict: merge",
                PrVerdict::NoMerge => "Verdict: no-merge",
            };
            Some(format!("{line}\nrun_id: {run_id}\n{summary}\n"))
        }
        _ => None,
    }
}

fn json_str(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if !rest.starts_with('"') { return None; }
    let mut out = String::new();
    let bytes = rest[1..].as_bytes();
    let mut j = 0;
    while j < bytes.len() {
        match bytes[j] {
            b'"' => return Some(out),
            b'\\' if j + 1 < bytes.len() => { out.push(bytes[j + 1] as char); j += 2; }
            c => { out.push(c as char); j += 1; }
        }
    }
    None
}

fn json_u32(text: &str, key: &str) -> Option<u32> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    n.parse().ok()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
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
        let p = env::temp_dir().join(format!("fy-out-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn parse_embedded_jsonl() {
        let t = "{\"type\":\"delta\"}\n{\"kind\":\"qa\",\"pr\":3,\"verdict\":\"merge\",\"summary\":\"ok\"}\n";
        match parse_outcome(t).unwrap() {
            Outcome::Qa { pr, verdict, .. } => { assert_eq!(pr, 3); assert_eq!(verdict, PrVerdict::Merge); }
            other => panic!("{other:?}"),
        }
    }
    #[test]
    fn stale_spec_rejected() {
        let o = Outcome::SpecReview { spec_sha256: "aaa".into(), verdict: SpecVerdict::Approve, summary: "x".into() };
        assert!(validate_outcome(Agent::TechPm, "review", &o, Some("bbb")).is_err());
        assert!(validate_outcome(Agent::TechPm, "review", &o, Some("aaa")).is_ok());
    }
    #[test]
    fn store_and_read_latest() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let stdout = r#"{"kind":"qa","pr":9,"verdict":"no_merge","summary":"fail"}"#;
        validate_and_store(&root, "toy", "20260101T000000Z-aaaa", Agent::Qa, "qa-on-cluster", stdout).unwrap();
        assert_eq!(latest_qa_outcome(&root, "toy", Some(9)), Some(PrVerdict::NoMerge));
        assert_eq!(latest_qa_outcome(&root, "toy", Some(1)), None);
    }
}
