use std::fs;
use std::path::Path;

use crate::cluster::remote_path;
use crate::tokens::Tokens;
use crate::types::Agent;

pub fn build_envelope(
    project: &str,
    agent: Agent,
    step: &str,
    task: &str,
    tokens: &Tokens,
    pack_dir: Option<&Path>,
) -> String {
    let mut s = String::new();
    s.push_str("FORGEYARD ENVELOPE\n");
    s.push_str(&format!("project: {project}\n"));
    s.push_str(&format!("agent: {}\n", agent.as_str()));
    s.push_str(&format!("step: {step}\n"));
    if needs_cluster(agent, step) {
        let host = tokens.get("cluster.host").unwrap_or("");
        let user = tokens.get("cluster.user").unwrap_or("");
        let port = tokens.get("cluster.port").unwrap_or("22");
        s.push_str(&format!("ssh: {user}@{host}:{port}\n"));
        s.push_str(&format!("remote: {}\n", remote_path(project)));
    } else {
        s.push_str("ssh: laptop\n");
    }
    if let Some(dir) = pack_dir {
        let role = dir.join("roles").join(format!("{}.md", agent.as_str()));
        if let Ok(text) = fs::read_to_string(&role) {
            s.push_str("\n# role\n");
            s.push_str(&text);
            if !text.ends_with('\n') { s.push('\n'); }
        }
        if let Ok(text) = fs::read_to_string(dir.join("pack.toml")) {
            s.push_str("\n# pack.toml\n");
            s.push_str(&text);
        }
    }
    if !task.is_empty() {
        s.push_str("\n# task\n");
        s.push_str(task);
        s.push('\n');
    }

    s.push_str("\n# outcome\n");
    s.push_str("End with one JSON object. Do not wrap it in markdown.\n");
    match agent {
        Agent::TechPm => s.push_str("{\"kind\":\"spec_review\",\"spec_sha256\":\"<sha256 of spec.md>\",\"verdict\":\"approve|needs_changes\",\"summary\":\"...\"}\n"),
        Agent::Developer => s.push_str("{\"kind\":\"implementation\",\"branch\":\"<branch>\",\"summary\":\"...\"}\n"),
        Agent::Qa => s.push_str("{\"kind\":\"qa\",\"pr\":<number>,\"verdict\":\"merge|no_merge\",\"summary\":\"...\"}\n"),
        _ => s.push_str("{\"kind\":\"implementation\",\"branch\":\"<branch>\",\"summary\":\"...\"}\n"),
    }

    s
}

pub fn needs_cluster(agent: Agent, step: &str) -> bool {
    matches!(agent, Agent::Developer | Agent::Qa) || step.contains("implement") || step.contains("qa")
}

pub fn cluster_configured(tokens: &Tokens) -> bool {
    tokens.get("cluster.host").is_some() && tokens.get("cluster.user").is_some()
}

pub fn llm_endpoint(tokens: &Tokens) -> Option<String> {
    tokens.get("llm.endpoint").map(|s| s.to_string())
}

pub fn llm_key(tokens: &Tokens) -> String {
    tokens.get("llm.default").unwrap_or("dummy").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_always_has_project() {
        let e = build_envelope("my-app", Agent::TechPm, "review", "look", &Tokens::default(), None);
        assert!(e.contains("project: my-app"));
        assert!(e.contains("look"));
    }
    #[test]
    fn developer_gets_remote_path() {
        let mut t = Tokens::default();
        t.set("cluster.host", "10.0.0.1").unwrap();
        t.set("cluster.user", "deploy").unwrap();
        let e = build_envelope("toy", Agent::Developer, "implement", "", &t, None);
        assert!(e.contains("remote: /srv/forgeyard/toy/repo"));
        assert!(e.contains("deploy@10.0.0.1"));
        assert!(!e.contains("password"));
    }
}
