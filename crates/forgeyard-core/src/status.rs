use std::fs;
use std::path::Path;

use crate::bind::{read_project_file, ProjectFile, SourceKind};
use crate::error::{ForgeError, Result};
use crate::events::decode_event_line;
use crate::paths::project_dir;
use crate::state::{load_state, State};

/// Shared block for `forge status`, `fy status`, yard `/status`.
pub fn render_status(root: &Path, project: &str) -> Result<String> {
    let dir = project_dir(root, project);
    let proj_path = dir.join("PROJECT.toml");
    if !proj_path.exists() {
        return Err(ForgeError::Precondition(format!("unknown project: {project}")));
    }
    let proj = read_project_file(&proj_path)?;
    let state = load_state(root, project)?;
    let spec = spec_label(&dir, &state);
    let last = last_event_line(&dir);
    Ok(format_block(&proj, &state, &spec, &last))
}

pub fn format_block(proj: &ProjectFile, state: &State, spec: &str, last: &str) -> String {
    let repo = format!("{}/{}", proj.owner, proj.repo);
    let source = source_label(proj);
    let agent = format!("{}  {}", state.agent.as_str(), state.agent_status.as_str());
    let mut out = String::from("forgeyard\n");
    out.push_str(&row("project", &state.project));
    out.push_str(&row("repo", &repo));
    out.push_str(&row("source", &source));
    out.push_str(&row("workflow", &state.workflow));
    out.push_str(&row("step", &state.step));
    out.push_str(&row("agent", &agent));
    out.push_str(&row("run", &state.run_id));
    out.push_str(&row("spec", spec));
    out.push_str(&row("updated", &state.updated_at));
    out.push_str(&row("last", last));
    out
}

fn row(key: &str, val: &str) -> String {
    format!("{:<10}{}\n", format!("{key}:"), val)
}

fn source_label(p: &ProjectFile) -> String {
    match (&p.source_kind, &p.source_ref) {
        (SourceKind::Issue, Some(n)) => format!("issues/{n}"),
        (SourceKind::Pull, Some(n)) => format!("pull/{n}"),
        _ => "repo".into(),
    }
}

fn spec_label(dir: &Path, state: &State) -> String {
    let path = dir.join(if state.spec_path.is_empty() { "spec.md" } else { &state.spec_path });
    match fs::read_to_string(&path) {
        Ok(s) if !s.trim().is_empty() => "present".into(),
        _ => "missing".into(),
    }
}

fn last_event_line(dir: &Path) -> String {
    let path = dir.join("events.jsonl");
    let Ok(text) = fs::read_to_string(path) else { return "-".into(); };
    let Some(line) = text.lines().rev().find(|l| !l.trim().is_empty()) else { return "-".into(); };
    match decode_event_line(line) {
        Ok(ev) => {
            if ev.step.is_empty() { ev.hook.as_str().to_string() }
            else { format!("{} {}", ev.hook.as_str(), ev.step) }
        }
        Err(_) => "-".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Agent, AgentStatus};
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-st-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    const GOLDEN: &str = "\
forgeyard
project:  my-app
repo:     camorazrushimoe/my-app
source:   issues/14
workflow: spec-review
step:     review_in_progress
agent:    tech-pm  busy
run:      20260909T080012Z-ab12
spec:     present
updated:  2026-09-09T08:00:12Z
last:     start adversarial_review
";
    #[test]
    fn golden_matches_examples_status_txt() {
        let proj = ProjectFile {
            project: "my-app".into(), owner: "camorazrushimoe".into(), repo: "my-app".into(),
            source_url: "https://github.com/camorazrushimoe/my-app/issues/14".into(),
            source_kind: SourceKind::Issue, source_ref: Some("14".into()),
            bound_at: "2026-09-09T08:00:00Z".into(),
        };
        let mut state = State::idle("my-app");
        state.workflow = "spec-review".into();
        state.step = "review_in_progress".into();
        state.agent = Agent::TechPm;
        state.agent_status = AgentStatus::Busy;
        state.run_id = "20260909T080012Z-ab12".into();
        state.updated_at = "2026-09-09T08:00:12Z".into();
        assert_eq!(format_block(&proj, &state, "present", "start adversarial_review"), GOLDEN);
    }
    #[test]
    fn unknown_project_is_precondition() {
        let err = render_status(&tmp(), "nope").unwrap_err();
        assert_eq!(err.exit(), crate::Exit::Precondition);
    }
    #[test]
    fn render_from_disk_fixtures() {
        let root = tmp();
        let dir = project_dir(&root, "my-app");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("PROJECT.toml"), "project = \"my-app\"\nowner = \"camorazrushimoe\"\nrepo = \"my-app\"\nsource_url = \"https://github.com/camorazrushimoe/my-app/issues/14\"\nsource_kind = \"issue\"\nsource_ref = \"14\"\nbound_at = \"2026-09-09T08:00:00Z\"\n").unwrap();
        fs::write(dir.join("state.json"), "{\n  \"schema\": 1,\n  \"project\": \"my-app\",\n  \"workflow\": \"spec-review\",\n  \"step\": \"review_in_progress\",\n  \"agent\": \"tech-pm\",\n  \"agent_status\": \"busy\",\n  \"run_id\": \"20260909T080012Z-ab12\",\n  \"input\": \"https://github.com/camorazrushimoe/my-app/issues/14\",\n  \"spec_path\": \"spec.md\",\n  \"updated_at\": \"2026-09-09T08:00:12Z\"\n}\n").unwrap();
        fs::write(dir.join("spec.md"), "# spec\n").unwrap();
        fs::write(dir.join("events.jsonl"), "{\"ts\":\"2026-09-09T08:00:12Z\",\"project\":\"my-app\",\"hook\":\"start\",\"step\":\"adversarial_review\"}\n").unwrap();
        assert_eq!(render_status(&root, "my-app").unwrap(), GOLDEN);
    }
}
