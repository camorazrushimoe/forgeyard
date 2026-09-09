use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{ForgeError, Result};
use crate::paths::project_dir;
use crate::project::list_bound_projects;
use crate::state::load_state;
use crate::status::render_status;
use crate::tokens::{list_rows, load_meta, load_tokens, save_tokens, tokens_path, CANONICAL};
use crate::types::AgentStatus;

pub const PANEL_HELP: &str = "\
forgeyard panel
/status     factory or one project
/who        who is busy right now
/projects   list bound projects
/tokens     list token names (no values)
/settoken   rotate an LLM token (interactive)
";

pub const UNKNOWN: &str = "unknown command. /status /who /projects /tokens /help";

#[derive(Debug, Clone, Default)]
pub struct PendingSet { pub user_id: i64, pub name: String, pub deadline_unix: u64 }

#[derive(Debug, Clone, Default)]
pub struct PanelState { pub pending: Option<PendingSet> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply { pub text: String, pub silent: bool, pub delete_user_msg: bool }

impl Reply {
    fn say(t: impl Into<String>) -> Self { Reply { text: t.into(), silent: false, delete_user_msg: false } }
    fn silent() -> Self { Reply { text: String::new(), silent: true, delete_user_msg: false } }
}

pub fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn handle(root: &Path, st: &mut PanelState, user_id: i64, chat_private: bool, text: &str, now: u64) -> Result<Reply> {
    if !chat_private { return Ok(Reply::silent()); }
    let tokens = load_tokens(root)?;
    if !tokens.allow_user_ids.contains(&user_id) { return Ok(Reply::silent()); }
    let msg = text.trim();
    if let Some(p) = st.pending.clone() {
        if p.user_id == user_id {
            if msg == "/cancel" { st.pending = None; return Ok(Reply::say("cancelled")); }
            if now > p.deadline_unix {
                st.pending = None;
                if !msg.starts_with('/') { return Ok(Reply::say("expired, not set")); }
            } else if !msg.starts_with('/') {
                st.pending = None;
                return finish_settoken(root, &p.name, msg);
            }
        }
    }
    if msg == "/help" || msg == "/start" { return Ok(Reply::say(PANEL_HELP)); }
    if msg == "/status" { return Ok(Reply::say(factory_status(root)?)); }
    if let Some(rest) = msg.strip_prefix("/status ") {
        let p = rest.trim();
        return match render_status(root, p) {
            Ok(b) => Ok(Reply::say(b)),
            Err(_) => Ok(Reply::say(format!("unknown project: {p}"))),
        };
    }
    if msg == "/who" { return Ok(Reply::say(who(root)?)); }
    if msg == "/projects" { return Ok(Reply::say(projects_list(root)?)); }
    if msg == "/tokens" {
        let t = load_tokens(root)?;
        let meta = load_meta(root).unwrap_or_default();
        return Ok(Reply::say(list_rows(&t, &meta)));
    }
    if let Some(rest) = msg.strip_prefix("/settoken") {
        let name = rest.trim();
        if name.is_empty() { return Ok(Reply::say("usage: /settoken <name>")); }
        if !CANONICAL.contains(&name) { return Ok(Reply::say("token not set")); }
        st.pending = Some(PendingSet { user_id, name: name.to_string(), deadline_unix: now + 120 });
        return Ok(Reply::say("send the new value as the next message, or /cancel. the value will be deleted from chat after save."));
    }
    Ok(Reply::say(UNKNOWN))
}

fn finish_settoken(root: &Path, name: &str, value: &str) -> Result<Reply> {
    let mut t = load_tokens(root)?;
    if t.set(name, value).is_err() { return Ok(Reply::say("token not set")); }
    if save_tokens(root, &t, "yard").is_err() { return Ok(Reply::say("token not set")); }
    let c: Vec<char> = value.chars().collect();
    let n = c.len().min(4);
    let tail = format!("..{}", c[c.len()-n..].iter().collect::<String>());
    Ok(Reply { text: format!("set {name} tail={tail}"), silent: false, delete_user_msg: true })
}

fn factory_status(root: &Path) -> Result<String> {
    let names = list_bound_projects(root)?;
    if names.is_empty() { return Ok("factory: no projects\n".into()); }
    let mut s = String::from("factory\n");
    for n in names {
        match render_status(root, &n) {
            Ok(b) => s.push_str(&b),
            Err(_) => s.push_str(&format!("{n}  (unreadable)\n")),
        }
    }
    Ok(s)
}

fn who(root: &Path) -> Result<String> {
    let mut lines = Vec::new();
    for n in list_bound_projects(root)? {
        if let Ok(st) = load_state(root, &n) {
            if st.agent_status == AgentStatus::Busy {
                lines.push(format!("{n}  {}  {}  {}", st.agent.as_str(), st.step, st.run_id));
            }
        }
    }
    if lines.is_empty() { Ok("idle: no agents running\n".into()) } else { Ok(lines.join("\n") + "\n") }
}

fn projects_list(root: &Path) -> Result<String> {
    let names = list_bound_projects(root)?;
    if names.is_empty() { return Ok("(none)\n".into()); }
    let mut s = String::new();
    for n in names {
        let spec_yes = std::fs::read_to_string(project_dir(root, &n).join("spec.md")).map(|t| !t.trim().is_empty()).unwrap_or(false);
        let step = load_state(root, &n).map(|st| st.step).unwrap_or_default();
        s.push_str(&format!("{n:<10} bound     spec={}  step={}\n", if spec_yes { "yes" } else { "no" }, if step.is_empty() { "-" } else { &step }));
    }
    Ok(s)
}

pub fn preflight(root: &Path) -> Result<()> {
    if !tokens_path(root).exists() { return Err(ForgeError::Tokens("tokens.toml missing".into())); }
    let t = load_tokens(root)?;
    if t.allow_user_ids.is_empty() { return Err(ForgeError::Usage("empty allowlist".into())); }
    if t.get("telegram.bot_token").is_none() && std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|s| !s.is_empty()).is_none() {
        return Err(ForgeError::Tokens("telegram.bot_token missing".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::bind_project;
    use crate::tokens::{save_tokens, Tokens};
    use std::env; use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-yard-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p); fs::create_dir_all(&p).unwrap(); p
    }
    fn seeded() -> (std::path::PathBuf, i64) {
        let root = tmp();
        let mut t = Tokens::default();
        t.set("telegram.bot_token", "111:AAA").unwrap();
        t.allow_user_ids = vec![42];
        save_tokens(&root, &t, "t").unwrap();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        (root, 42)
    }
    #[test]
    fn stranger_is_silent() {
        let (root, _) = seeded(); let mut st = PanelState::default();
        assert!(handle(&root, &mut st, 99, true, "/status", 0).unwrap().silent);
    }
    #[test]
    fn group_ignored() {
        let (root, uid) = seeded(); let mut st = PanelState::default();
        assert!(handle(&root, &mut st, uid, false, "/status", 0).unwrap().silent);
    }
    #[test]
    fn help_and_unknown() {
        let (root, uid) = seeded(); let mut st = PanelState::default();
        assert!(handle(&root, &mut st, uid, true, "/help", 0).unwrap().text.starts_with("forgeyard panel"));
        assert_eq!(handle(&root, &mut st, uid, true, "hello", 0).unwrap().text, UNKNOWN);
    }
    #[test]
    fn status_project_matches_render() {
        let (root, uid) = seeded(); let mut st = PanelState::default();
        let r = handle(&root, &mut st, uid, true, "/status toy", 0).unwrap();
        assert_eq!(r.text, render_status(&root, "toy").unwrap());
    }
    #[test]
    fn tokens_has_no_secret() {
        let (root, uid) = seeded(); let mut st = PanelState::default();
        let r = handle(&root, &mut st, uid, true, "/tokens", 0).unwrap();
        assert!(r.text.contains("telegram.bot_token"));
        assert!(!r.text.contains("111:AAA"));
    }
    #[test]
    fn settoken_two_step() {
        let (root, uid) = seeded(); let mut st = PanelState::default();
        handle(&root, &mut st, uid, true, "/settoken llm.default", 100).unwrap();
        let r = handle(&root, &mut st, uid, true, "sk-NEWVALUE99", 110).unwrap();
        assert!(r.delete_user_msg);
        assert!(!r.text.contains("sk-NEWVALUE99"));
        assert_eq!(load_tokens(&root).unwrap().get("llm.default"), Some("sk-NEWVALUE99"));
    }
    #[test]
    fn settoken_timeout() {
        let (root, uid) = seeded(); let mut st = PanelState::default();
        handle(&root, &mut st, uid, true, "/settoken llm.default", 100).unwrap();
        assert_eq!(handle(&root, &mut st, uid, true, "sk-late", 221).unwrap().text, "expired, not set");
    }
    #[test]
    fn preflight_empty_allowlist_is_usage() {
        let root = tmp(); let mut t = Tokens::default();
        t.set("telegram.bot_token", "x").unwrap(); save_tokens(&root, &t, "t").unwrap();
        assert_eq!(preflight(&root).unwrap_err().exit(), crate::Exit::Usage);
    }
}
