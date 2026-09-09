use std::fs;
use std::path::Path;

use crate::bind::{bind_project, parse_github_ref};
use crate::error::Result;
use crate::events::{append_event, Event};
use crate::paths::project_dir;
use crate::types::{Agent, Hook};

pub fn fy_do(root: &Path, url: &str, text: &str) -> Result<String> {
    let binding = parse_github_ref(url)?;
    let project = binding.project().to_string();
    bind_project(root, url)?;
    let dir = project_dir(root, &project);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("inbox.md"), format!("url: {url}\ntext: {text}\n"))?;
    let mut ev = Event::new(&project, Hook::Intake);
    ev.agent = Agent::Human;
    ev.input = url.to_string();
    ev.summary = text.to_string();
    ev.status = "queued".into();
    append_event(root, &ev)?;
    Ok(format!("queued {project} — watch will pick it up"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-do-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn do_writes_inbox_and_event() {
        let root = tmp();
        let msg = fy_do(&root, "https://github.com/acme/toy/issues/4", "add login").unwrap();
        assert_eq!(msg, "queued toy — watch will pick it up");
        let inbox = fs::read_to_string(project_dir(&root, "toy").join("inbox.md")).unwrap();
        assert!(inbox.contains("add login"));
        let log = fs::read_to_string(project_dir(&root, "toy").join("events.jsonl")).unwrap();
        assert!(log.contains("\"hook\":\"intake\""));
    }
}
