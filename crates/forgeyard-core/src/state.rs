use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use crate::error::{ForgeError, Result};
use crate::lock::{state_lock_path, FileLock};
use crate::paths::project_dir;
use crate::types::{Agent, AgentStatus};

/// Project busy lock + current step. SPEC.md §6. schema = 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub schema: u32,
    pub project: String,
    pub workflow: String,
    pub step: String,
    pub agent: Agent,
    pub agent_status: AgentStatus,
    pub run_id: String,
    pub input: String,
    pub spec_path: String,
    pub updated_at: String,
}

impl State {
    pub fn idle(project: &str) -> Self {
        State {
            schema: 1,
            project: project.to_string(),
            workflow: String::new(),
            step: String::new(),
            agent: Agent::Forge,
            agent_status: AgentStatus::Idle,
            run_id: String::new(),
            input: String::new(),
            spec_path: "spec.md".into(),
            updated_at: String::new(),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.agent_status == AgentStatus::Busy
    }
}

pub fn state_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join("state.json")
}

pub fn load_state(root: &Path, project: &str) -> Result<State> {
    let dir = project_dir(root, project);
    let path = state_path(&dir);
    let _lock = FileLock::acquire(&state_lock_path(&dir))?;
    if !path.exists() {
        return Ok(State::idle(project));
    }
    read_state_unlocked(&path)
}

pub fn save_state(root: &Path, state: &State) -> Result<()> {
    let dir = project_dir(root, &state.project);
    fs::create_dir_all(&dir)?;
    let _lock = FileLock::acquire(&state_lock_path(&dir))?;
    atomic_write(&state_path(&dir), &encode_state(state)?)
}

/// Mark project busy. Second start while busy → exit 4.
pub fn become_busy(
    root: &Path,
    project: &str,
    agent: Agent,
    workflow: &str,
    step: &str,
    run_id: &str,
    input: &str,
) -> Result<State> {
    let dir = project_dir(root, project);
    fs::create_dir_all(&dir)?;
    let path = state_path(&dir);
    let _lock = FileLock::acquire(&state_lock_path(&dir))?;
    let mut st = if path.exists() {
        read_state_unlocked(&path)?
    } else {
        State::idle(project)
    };
    if st.is_busy() {
        return Err(ForgeError::Busy(format!(
            "project {project} busy agent={} run={}",
            st.agent, st.run_id
        )));
    }
    st.schema = 1;
    st.project = project.to_string();
    st.agent = agent;
    st.agent_status = AgentStatus::Busy;
    st.workflow = workflow.to_string();
    st.step = step.to_string();
    st.run_id = run_id.to_string();
    st.input = input.to_string();
    st.updated_at = now_rfc3339();
    atomic_write(&path, &encode_state(&st)?)?;
    Ok(st)
}

pub fn become_idle(root: &Path, project: &str) -> Result<State> {
    let dir = project_dir(root, project);
    let path = state_path(&dir);
    let _lock = FileLock::acquire(&state_lock_path(&dir))?;
    let mut st = if path.exists() {
        read_state_unlocked(&path)?
    } else {
        State::idle(project)
    };
    st.agent_status = AgentStatus::Idle;
    st.updated_at = now_rfc3339();
    atomic_write(&path, &encode_state(&st)?)?;
    Ok(st)
}

fn read_state_unlocked(path: &Path) -> Result<State> {
    let mut f = File::open(path)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    decode_state(&buf)
}

fn atomic_write(path: &Path, bytes: &str) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(bytes.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn encode_state(s: &State) -> Result<String> {
    Ok(format!(
        "{{\n  \"schema\": {},\n  \"project\": \"{}\",\n  \"workflow\": \"{}\",\n  \"step\": \"{}\",\n  \"agent\": \"{}\",\n  \"agent_status\": \"{}\",\n  \"run_id\": \"{}\",\n  \"input\": \"{}\",\n  \"spec_path\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
        s.schema,
        esc(&s.project),
        esc(&s.workflow),
        esc(&s.step),
        s.agent.as_str(),
        s.agent_status.as_str(),
        esc(&s.run_id),
        esc(&s.input),
        esc(&s.spec_path),
        esc(&s.updated_at),
    ))
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn decode_state(text: &str) -> Result<State> {
    let schema = num_field(text, "schema").unwrap_or(1);
    let project = str_field(text, "project")?;
    Ok(State {
        schema,
        project,
        workflow: str_field(text, "workflow").unwrap_or_default(),
        step: str_field(text, "step").unwrap_or_default(),
        agent: str_field(text, "agent")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(Agent::Forge),
        agent_status: match str_field(text, "agent_status").unwrap_or_default().as_str() {
            "busy" => AgentStatus::Busy,
            _ => AgentStatus::Idle,
        },
        run_id: str_field(text, "run_id").unwrap_or_default(),
        input: str_field(text, "input").unwrap_or_default(),
        spec_path: str_field(text, "spec_path").unwrap_or_else(|_| "spec.md".into()),
        updated_at: str_field(text, "updated_at").unwrap_or_default(),
    })
}

fn str_field(text: &str, key: &str) -> Result<String> {
    let pat = format!("\"{key}\"");
    let Some(i) = text.find(&pat) else {
        return Err(ForgeError::Precondition(format!(
            "state.json missing {key}"
        )));
    };
    let rest = &text[i + pat.len()..];
    let Some(colon) = rest.find(':') else {
        return Err(ForgeError::Precondition(format!("bad field {key}")));
    };
    let rest = rest[colon + 1..].trim_start();
    if rest.starts_with('"') {
        let mut out = String::new();
        let bytes = rest[1..].as_bytes();
        let mut j = 0;
        while j < bytes.len() {
            match bytes[j] {
                b'"' => return Ok(out),
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
    }
    Err(ForgeError::Precondition(format!("bad string field {key}")))
}

fn num_field(text: &str, key: &str) -> Option<u32> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    n.parse().ok()
}

fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-state-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    const EXAMPLE: &str = r#"{
  "schema": 1,
  "project": "my-app",
  "workflow": "spec-review",
  "step": "review_in_progress",
  "agent": "tech-pm",
  "agent_status": "busy",
  "run_id": "20260909T080012Z-ab12",
  "input": "https://github.com/camorazrushimoe/my-app/issues/14",
  "spec_path": "spec.md",
  "updated_at": "2026-09-09T08:00:12Z"
}
"#;

    #[test]
    fn decode_example_state() {
        let s = decode_state(EXAMPLE).unwrap();
        assert_eq!(s.schema, 1);
        assert_eq!(s.project, "my-app");
        assert_eq!(s.agent, Agent::TechPm);
        assert!(s.is_busy());
        assert_eq!(s.run_id, "20260909T080012Z-ab12");
    }

    #[test]
    fn roundtrip_write_read() {
        let root = tmp();
        let mut s = State::idle("toy");
        s.workflow = "spec-review".into();
        s.step = "review_in_progress".into();
        save_state(&root, &s).unwrap();
        let loaded = load_state(&root, "toy").unwrap();
        assert_eq!(loaded.project, "toy");
        assert_eq!(loaded.workflow, "spec-review");
        assert!(!loaded.is_busy());
    }

    #[test]
    fn second_busy_is_conflict_busy() {
        let root = tmp();
        become_busy(&root, "toy", Agent::TechPm, "spec-review", "review", "r1", "").unwrap();
        let err = become_busy(&root, "toy", Agent::Developer, "x", "y", "r2", "").unwrap_err();
        assert_eq!(err.exit(), crate::Exit::Busy);
        become_idle(&root, "toy").unwrap();
        assert!(become_busy(&root, "toy", Agent::Qa, "qa", "run", "r3", "").is_ok());
    }
}
