use std::env;
use std::path::PathBuf;

/// Default factory root: `$FORGEYARD_ROOT` or `~/.forgeyard/factory`. SPEC.md §4.
pub fn factory_root() -> PathBuf {
    if let Ok(v) = env::var("FORGEYARD_ROOT") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_factory_root()
}

pub fn default_factory_root() -> PathBuf {
    home_dir().join(".forgeyard").join("factory")
}

pub fn install_root() -> PathBuf {
    home_dir().join(".forgeyard")
}

pub fn project_dir(root: &std::path::Path, project: &str) -> PathBuf {
    root.join("projects").join(project)
}

fn home_dir() -> PathBuf {
    if let Ok(h) = env::var("HOME") {
        if !h.trim().is_empty() {
            return PathBuf::from(h);
        }
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_root_ends_with_forgeyard_factory() {
        let p = default_factory_root();
        assert!(p.ends_with(".forgeyard/factory") || p.ends_with(".forgeyard\\factory"));
        let s = p.to_string_lossy();
        assert!(s.contains(".forgeyard"));
        assert!(s.ends_with("factory"));
    }

    #[test]
    fn env_overrides_root() {
        env::set_var("FORGEYARD_ROOT", "/tmp/fy-test-root");
        assert_eq!(factory_root(), PathBuf::from("/tmp/fy-test-root"));
        env::remove_var("FORGEYARD_ROOT");
    }

    #[test]
    fn project_dir_nests_under_projects() {
        let p = project_dir(std::path::Path::new("/r"), "toy");
        assert_eq!(p, PathBuf::from("/r/projects/toy"));
    }
}
