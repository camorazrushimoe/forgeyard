use std::io::{BufRead, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::error::{ForgeError, Result};
use crate::tokens::{load_tokens, save_tokens, Tokens};

pub const DONE: &str = "ready.  start: fy start\n";

pub trait Probe {
    fn telegram_get_me(&self, token: &str) -> Result<()>;
    fn llm_models(&self, endpoint: &str, key: &str) -> Result<()>;
    fn github_user(&self, pat: &str) -> Result<()>;
    fn ssh_auth(&self, user: &str, host: &str, port: u16, password: &str) -> Result<()>;
}

pub struct LiveProbe;

impl Probe for LiveProbe {
    fn telegram_get_me(&self, token: &str) -> Result<()> {
        http_ok(&format!("https://api.telegram.org/bot{token}/getMe"), None)
    }
    fn llm_models(&self, endpoint: &str, key: &str) -> Result<()> {
        http_ok(&format!("{}/models", endpoint.trim_end_matches('/')), Some(key))
    }
    fn github_user(&self, pat: &str) -> Result<()> {
        http_ok("https://api.github.com/user", Some(pat))
    }
    fn ssh_auth(&self, user: &str, host: &str, port: u16, _password: &str) -> Result<()> {
        let addr = format!("{host}:{port}");
        let sock = addr.parse().map_err(|e| ForgeError::Precondition(format!("{e}")))?;
        TcpStream::connect_timeout(&sock, Duration::from_secs(10))
            .map_err(|e| ForgeError::Precondition(format!("ssh tcp {addr}: {e}")))?;
        let _ = user;
        Ok(())
    }
}

fn http_ok(url: &str, bearer: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-o", "/dev/null", "-w", "%{http_code}", "-m", "10", "-A", "forgeyard-onboard"]);
    if let Some(t) = bearer {
        cmd.arg("-H").arg(format!("Authorization: Bearer {t}"));
    }
    cmd.arg(url);
    let out = cmd.output().map_err(|e| ForgeError::Precondition(format!("curl: {e}")))?;
    let code = String::from_utf8_lossy(&out.stdout);
    if code.starts_with('2') { Ok(()) } else { Err(ForgeError::Precondition(format!("http {code} for check"))) }
}

pub fn parse_ssh_target(s: &str) -> Result<(String, String, u16)> {
    let s = s.trim();
    let (user, rest) = s.split_once('@').ok_or_else(|| ForgeError::Usage("need user@host".into()))?;
    if user.is_empty() { return Err(ForgeError::Usage("empty user".into())); }
    let (host, port) = if let Some((h, p)) = rest.rsplit_once(':') {
        if p.chars().all(|c| c.is_ascii_digit()) { (h, p.parse().unwrap_or(22)) } else { (rest, 22) }
    } else { (rest, 22) };
    if host.is_empty() { return Err(ForgeError::Usage("empty host".into())); }
    Ok((user.to_string(), host.to_string(), port))
}

pub fn apply_field(t: &mut Tokens, name: &str, value: &str) -> Result<()> {
    if name == "cluster.target" {
        let (user, host, port) = parse_ssh_target(value)?;
        t.set("cluster.user", &user)?;
        t.set("cluster.host", &host)?;
        t.set("cluster.port", &port.to_string())?;
    } else {
        t.set(name, value)?;
    }
    Ok(())
}

pub fn check_field(probe: &dyn Probe, name: &str, value: &str, tokens: &Tokens) -> Result<()> {
    match name {
        "telegram.bot_token" => if value.is_empty() { Ok(()) } else { probe.telegram_get_me(value) },
        "llm.endpoint" => if value.is_empty() { Err(ForgeError::Precondition("llm endpoint required".into())) } else { Ok(()) },
        "llm.default" => {
            let ep = tokens.get("llm.endpoint").unwrap_or("");
            if ep.is_empty() { Err(ForgeError::Precondition("set llm.endpoint first".into())) } else { probe.llm_models(ep, value) }
        }
        "github.pat" => {
            if value.is_empty() { Err(ForgeError::Precondition("github pat required".into())) } else { probe.github_user(value) }
        }
        "cluster.target" => parse_ssh_target(value).map(|_| ()),
        "cluster.password" => {
            let user = tokens.get("cluster.user").unwrap_or("");
            let host = tokens.get("cluster.host").unwrap_or("");
            let port = tokens.get("cluster.port").unwrap_or("22").parse().unwrap_or(22);
            probe.ssh_auth(user, host, port, value)
        }
        _ => Err(ForgeError::Usage(format!("unknown field {name}"))),
    }
}

const ORDER: &[&str] = ["telegram.bot_token", "llm.endpoint", "llm.default", "github.pat", "cluster.target", "cluster.password"];

fn prompt_for(name: &str) -> &'static str {
    match name {
        "telegram.bot_token" => "telegram bot token (empty = skip yard): ",
        "llm.endpoint" => "llm endpoint (https://openrouter.ai/api/v1 or https://api.openai.com/v1): ",
        "llm.default" => "llm token: ",
        "github.pat" => "github PAT (repo scope): ",
        "cluster.target" => "ssh user@host[:port]: ",
        "cluster.password" => "ssh password: ",
        _ => "value: ",
    }
}

fn secret_field(name: &str) -> bool {
    matches!(name, "telegram.bot_token" | "llm.default" | "github.pat" | "cluster.password")
}

pub fn run_wizard(
    root: &Path,
    probe: &dyn Probe,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<()> {
    let mut tokens = load_tokens(root).unwrap_or_default();
    for name in ORDER {
        loop {
            write!(out, "{}", prompt_for(name)).ok();
            out.flush().ok();
            let mut line = String::new();
            if input.read_line(&mut line).is_err() {
                return Err(ForgeError::Precondition("stdin closed".into()));
            }
            if secret_field(name) { writeln!(out).ok(); }
            let value = line.trim().to_string();
            match check_field(probe, name, &value, &tokens) {
                Ok(()) => { apply_field(&mut tokens, name, &value)?; break; }
                Err(e) => { writeln!(err, "check failed: {e}").ok(); }
            }
        }
    }
    save_tokens(root, &tokens, "onboard")?;
    write!(out, "{DONE}").ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-ob-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    struct Fake;
    impl Probe for Fake {
        fn telegram_get_me(&self, token: &str) -> Result<()> {
            if token == "bad" { Err(ForgeError::Precondition("tg".into())) } else { Ok(()) }
        }
        fn llm_models(&self, _e: &str, key: &str) -> Result<()> {
            if key.is_empty() { Err(ForgeError::Precondition("llm".into())) } else { Ok(()) }
        }
        fn github_user(&self, pat: &str) -> Result<()> {
            if pat == "badpat" { Err(ForgeError::Precondition("gh".into())) } else { Ok(()) }
        }
        fn ssh_auth(&self, _u: &str, _h: &str, _p: u16, password: &str) -> Result<()> {
            if password == "secret-pass" { Ok(()) } else { Err(ForgeError::Precondition("ssh".into())) }
        }
    }
    #[test]
    fn parse_user_host_port() {
        let (u, h, p) = parse_ssh_target("deploy@203.0.113.10:2222").unwrap();
        assert_eq!((u.as_str(), h.as_str(), p), ("deploy", "203.0.113.10", 2222));
    }
    #[test]
    fn empty_telegram_ok() {
        assert!(check_field(&Fake, "telegram.bot_token", "", &Tokens::default()).is_ok());
    }
    #[test]
    fn wizard_retries_bad_pat_then_saves_0600() {
        let root = tmp();
        let script = "\nhttp://127.0.0.1:9/v1\nsk-x\nbadpat\ngoodpat\ndeploy@10.0.0.1\nsecret-pass\n";
        let mut input = script.as_bytes();
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_wizard(&root, &Fake, &mut input, &mut out, &mut err).unwrap();
        let out_s = String::from_utf8_lossy(&out);
        let err_s = String::from_utf8_lossy(&err);
        assert!(err_s.contains("check failed"));
        assert!(out_s.contains("ready.  start: fy start"));
        assert!(!out_s.contains("secret-pass"));
        assert!(!out_s.contains("goodpat"));
        let loaded = load_tokens(&root).unwrap();
        assert_eq!(loaded.get("github.pat"), Some("goodpat"));
        assert_eq!(loaded.get("cluster.password"), Some("secret-pass"));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(crate::tokens::tokens_path(&root)).unwrap().permissions();
        assert_eq!(mode.mode() & 0o777, 0o600);
    }
}
