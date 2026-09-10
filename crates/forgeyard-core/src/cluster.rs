use crate::tokens::Tokens;

/// Canonical worktree on the SSH host. spec/cluster.md
pub fn remote_path(project: &str) -> String {
    format!("/srv/forgeyard/{project}/repo")
}

pub fn ssh_target(tokens: &Tokens) -> Option<String> {
    let host = tokens.get("cluster.host")?;
    let user = tokens.get("cluster.user")?;
    let port = tokens.get("cluster.port").unwrap_or("22");
    Some(format!("{user}@{host}:{port}"))
}

/// One-time commands for the host. Password is not in this string.
pub fn bootstrap_commands(project: &str, clone_url: &str) -> String {
    let path = remote_path(project);
    format!(
        "sudo mkdir -p {path} && sudo chown \"$(whoami)\" {path}\n\
if [ ! -d {path}/.git ]; then git clone {clone_url} {path}; fi\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_is_srv_forgeyard_repo() {
        assert_eq!(remote_path("toy"), "/srv/forgeyard/toy/repo");
    }
    #[test]
    fn bootstrap_has_no_password() {
        let mut t = Tokens::default();
        t.set("cluster.password", "s3cret-pass").unwrap();
        t.set("cluster.host", "10.0.0.1").unwrap();
        t.set("cluster.user", "deploy").unwrap();
        let s = bootstrap_commands("toy", "https://github.com/acme/toy.git");
        assert!(s.contains("/srv/forgeyard/toy/repo"));
        assert!(!s.contains("s3cret-pass"));
        assert_eq!(ssh_target(&t).as_deref(), Some("deploy@10.0.0.1:22"));
    }
}
