use std::fs;
use std::path::Path;

use crate::error::{ForgeError, Result};
use crate::paths::project_dir;

pub fn list_bound_projects(root: &Path) -> Result<Vec<String>> {
    let dir = root.join("projects");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for ent in fs::read_dir(&dir)? {
        let ent = ent?;
        if !ent.file_type()?.is_dir() {
            continue;
        }
        if ent.path().join("PROJECT.toml").exists() {
            if let Some(n) = ent.file_name().to_str() {
                names.push(n.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn current_project(root: &Path) -> Result<String> {
    if let Ok(p) = std::env::var("FORGEYARD_PROJECT") {
        let p = p.trim().to_string();
        if !p.is_empty() {
            return Ok(p);
        }
    }
    let names = list_bound_projects(root)?;
    match names.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(ForgeError::Precondition("no project bound".into())),
        _ => Err(ForgeError::Usage(
            "multiple projects; pass name or set FORGEYARD_PROJECT".into(),
        )),
    }
}

pub fn require_spec(root: &Path, project: &str) -> Result<()> {
    let path = project_dir(root, project).join("spec.md");
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            return Err(ForgeError::Precondition(format!("spec.md missing for {project}")));
        }
    };
    if text.trim().is_empty() {
        return Err(ForgeError::Precondition(format!("spec.md empty for {project}")));
    }
    Ok(())
}

pub fn read_event_log(root: &Path, project: &str) -> Result<String> {
    let path = project_dir(root, project).join("events.jsonl");
    if !path.exists() {
        return Ok(String::new());
    }
    Ok(fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-proj-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn require_spec_missing_is_precondition() {
        let err = require_spec(&tmp(), "x").unwrap_err();
        assert_eq!(err.exit(), crate::Exit::Precondition);
    }
    #[test]
    fn require_spec_empty_is_precondition() {
        let root = tmp();
        let dir = project_dir(&root, "x");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("spec.md"), "  \n").unwrap();
        let err = require_spec(&root, "x").unwrap_err();
        assert_eq!(err.exit(), crate::Exit::Precondition);
    }
    #[test]
    fn require_spec_ok_when_nonempty() {
        let root = tmp();
        let dir = project_dir(&root, "x");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("spec.md"), "# hi\n").unwrap();
        assert!(require_spec(&root, "x").is_ok());
    }
}
