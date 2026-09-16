use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::time::SystemTime;

use crate::bind::bind_project;
use crate::daemon::{pid_alive, read_pid, runner_missing, watch_pid_path, yard_pid_path};
use crate::error::{ForgeError, Result};
use crate::events::Event;
use crate::intake::fy_do;
use crate::outcome::{encode_outcome, latest_outcomes};
use crate::paths::project_dir;
use crate::project::list_bound_projects;
use crate::spec_cache::{refresh_spec_cache, LiveSpecFetcher};
use crate::state::load_state;
use crate::status::render_status;
use crate::tokens::{load_tokens, save_tokens};
use crate::types::{Agent, Hook};

pub const PROTOCOL: &str = "forgeyard-mcp/1";
pub const DEFAULT_PORT: u16 = 18789;
pub const TOOLS: &[&str] = &[
    "factory_status", "project_status", "list_projects", "queue_work", "bind_repo",
    "tail_events", "last_outcome", "spec_refresh", "factory_health",
];

pub fn mcp_pid_path(root: &Path) -> std::path::PathBuf { root.join("mcp.pid") }
pub fn ngrok_pid_path(root: &Path) -> std::path::PathBuf { root.join("ngrok.pid") }
pub fn factory_events_path(root: &Path) -> std::path::PathBuf { root.join("events.jsonl") }

pub fn resolve_mcp_port(root: &Path) -> u16 {
    if let Ok(v) = std::env::var("FORGEYARD_MCP_PORT") {
        if let Ok(p) = v.parse::<u16>() { if p > 0 { return p; } }
    }
    if let Ok(t) = load_tokens(root) {
        if let Some(p) = t.get("mcp.port") {
            if let Ok(n) = p.parse::<u16>() { if n > 0 { return n; } }
        }
    }
    DEFAULT_PORT
}

pub fn generate_bind_token() -> String {
    let mut buf = [0u8; 32];
    if let Ok(mut f) = fs::File::open("/dev/urandom") { let _ = f.read_exact(&mut buf); }
    else {
        let n = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(1);
        for (i, b) in buf.iter_mut().enumerate() { *b = ((n >> ((i % 8) * 8)) as u8).wrapping_add(i as u8); }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn ensure_mcp_bind_token(root: &Path) -> Result<bool> {
    let mut t = load_tokens(root).unwrap_or_default();
    if t.get("mcp.bind_token").is_some() { return Ok(false); }
    t.set("mcp.bind_token", &generate_bind_token())?;
    save_tokens(root, &t, "fy-start")?;
    Ok(true)
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let ab = a.as_bytes(); let bb = b.as_bytes();
    let n = ab.len().max(bb.len());
    let mut acc = ab.len() ^ bb.len();
    for i in 0..n {
        let x = if i < ab.len() { ab[i] } else { 0 };
        let y = if i < bb.len() { bb[i] } else { 0 };
        acc |= (x ^ y) as usize;
    }
    acc == 0
}

pub fn extract_bearer(auth: &str) -> Option<&str> {
    let t = auth.trim();
    let rest = t.strip_prefix("Bearer ").or_else(|| t.strip_prefix("bearer "))?;
    let tok = rest.trim();
    if tok.is_empty() { None } else { Some(tok) }
}

pub fn append_factory_event(root: &Path, step: &str, status: &str, project: &str) -> Result<()> {
    let mut ev = Event::new(project, Hook::Mcp);
    ev.agent = Agent::Forge; ev.step = step.into(); ev.status = status.into();
    let _lock = crate::lock::FileLock::acquire(&crate::lock::events_lock_path(root))?;
    let mut line = crate::events::encode_event(&ev);
    if !line.ends_with('\n') { line.push('\n'); }
    let mut f = fs::OpenOptions::new().create(true).append(true).open(factory_events_path(root))?;
    f.write_all(line.as_bytes())?; f.sync_all()?; Ok(())
}

fn up_down(pid_path: &Path) -> &'static str {
    match read_pid(pid_path) { Some(p) if pid_alive(p) => "up", _ => "down" }
}

pub fn render_factory_status(root: &Path) -> String {
    let t = load_tokens(root).unwrap_or_default();
    let yard = if t.get("telegram.bot_token").is_some() || std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|s| !s.is_empty()).is_some() {
        up_down(&yard_pid_path(root))
    } else { "off" };
    let tunnel = if t.get("ngrok.url").is_some() && t.get("ngrok.auth_token").is_some() {
        up_down(&ngrok_pid_path(root))
    } else { "off" };
    let n = list_bound_projects(root).map(|v| v.len()).unwrap_or(0);
    let runner = if runner_missing() { "missing" } else { "present" };
    format!("forgeyard\nwatch:     {}\nmcp:       {}\nyard:      {}\ntunnel:    {}\nrunner:    {}\nprojects:  {}\n",
        up_down(&watch_pid_path(root)), up_down(&mcp_pid_path(root)), yard, tunnel, runner, n)
}

pub fn render_factory_health(root: &Path) -> String {
    format!("protocol={PROTOCOL}\nversion=0.1.0\n{}", render_factory_status(root))
}

pub struct HttpOutcome { pub status: u16, pub body: String }

pub fn handle_http(root: &Path, expected: &str, auth_header: Option<&str>, body: &str) -> HttpOutcome {
    let presented = auth_header.and_then(extract_bearer).unwrap_or("");
    if expected.is_empty() || !constant_time_eq(presented, expected) {
        let _ = append_factory_event(root, "auth", "denied", "");
        return HttpOutcome { status: 401, body: "{\"error\":\"unauthorized\"}\n".into() };
    }
    dispatch_rpc(root, body)
}

fn dispatch_rpc(root: &Path, body: &str) -> HttpOutcome {
    let method = json_str(body, "method").unwrap_or_default();
    let id = json_raw_id(body);
    match method.as_str() {
        "initialize" => ok_rpc(&id, r#"{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"forgeyard-mcp","version":"0.1.0"}}"#.into()),
        "notifications/initialized" | "initialized" => HttpOutcome { status: 202, body: String::new() },
        "tools/list" => ok_rpc(&id, tools_list_json()),
        "tools/call" => {
            let name = json_str(body, "name").unwrap_or_default();
            let args = json_obj_after(body, "arguments").unwrap_or_else(|| "{}".into());
            match call_tool(root, &name, &args) {
                Ok(text) => ok_rpc(&id, tool_result_json(&text, false)),
                Err(e) => ok_rpc(&id, tool_result_json(&e.to_string(), true)),
            }
        }
        "" => err_rpc(&id, -32700, "parse error"),
        other => err_rpc(&id, -32601, &format!("unknown method: {other}")),
    }
}

fn call_tool(root: &Path, name: &str, args: &str) -> Result<String> {
    if !TOOLS.contains(&name) { return Err(ForgeError::Usage(format!("unknown tool: {name}"))); }
    if args.contains("gh pr merge") || name.contains("merge") {
        return Err(ForgeError::Usage("merge is not an MCP tool".into()));
    }
    let project = json_str(args, "project").unwrap_or_default();
    let _ = append_factory_event(root, name, "ok", &project);
    match name {
        "factory_status" => Ok(render_factory_status(root)),
        "factory_health" => Ok(render_factory_health(root)),
        "list_projects" => {
            let mut out = String::new();
            for n in list_bound_projects(root)? {
                let spec = match fs::read_to_string(project_dir(root, &n).join("spec.md")) {
                    Ok(s) if !s.trim().is_empty() => "present", _ => "missing",
                };
                let step = load_state(root, &n).map(|s| s.step).unwrap_or_else(|_| "-".into());
                out.push_str(&format!("{n} spec={spec} step={step}\n"));
            }
            Ok(out)
        }
        "project_status" => {
            if project.is_empty() { return Err(ForgeError::Usage("project required".into())); }
            render_status(root, &project)
        }
        "queue_work" => {
            let url = json_str(args, "url").unwrap_or_default();
            let text = json_str(args, "text").unwrap_or_default();
            if url.is_empty() || text.is_empty() { return Err(ForgeError::Usage("url and text required".into())); }
            fy_do(root, &url, &text)
        }
        "bind_repo" => {
            let url = json_str(args, "url").unwrap_or_default();
            if url.is_empty() { return Err(ForgeError::Usage("url required".into())); }
            let p = bind_project(root, &url)?;
            Ok(format!("{}/{}", p.owner, p.repo))
        }
        "tail_events" => {
            let n = json_u64(args, "n").unwrap_or(50).clamp(1, 200) as usize;
            let path = if project.is_empty() { factory_events_path(root) } else { project_dir(root, &project).join("events.jsonl") };
            let Ok(text) = fs::read_to_string(path) else { return Ok(String::new()) };
            let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
            let start = lines.len().saturating_sub(n);
            Ok(lines[start..].join("\n") + if lines.len() > start { "\n" } else { "" })
        }
        "last_outcome" => {
            if project.is_empty() { return Err(ForgeError::Usage("project required".into())); }
            let dir = project_dir(root, &project);
            if !dir.join("PROJECT.toml").exists() { return Err(ForgeError::Precondition(format!("unknown project: {project}"))); }
            match latest_outcomes(root, &project).into_iter().last() {
                Some((id, o)) => Ok(format!("run_id={id}\n{}", encode_outcome(&o))),
                None => Ok("none\n".into()),
            }
        }
        "spec_refresh" => {
            if project.is_empty() { return Err(ForgeError::Usage("project required".into())); }
            let src = refresh_spec_cache(root, &project, &LiveSpecFetcher)?;
            Ok(format!("sha={}", src.content_sha256))
        }
        _ => Err(ForgeError::Usage(format!("unknown tool: {name}"))),
    }
}

fn tools_list_json() -> String {
    let mut tools = String::from("[");
    for (i, name) in TOOLS.iter().enumerate() {
        if i > 0 { tools.push(','); }
        tools.push_str(&format!(r#"{{"name":"{name}","description":"{name}","inputSchema":{{"type":"object","properties":{{}}}}}}"#));
    }
    tools.push(']');
    format!(r#"{{"tools":{tools}}}"#)
}
fn tool_result_json(text: &str, is_error: bool) -> String {
    let esc = text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    format!(r#"{{"content":[{{"type":"text","text":"{esc}"}}],"isError":{}}}"#, if is_error { "true" } else { "false" })
}
fn ok_rpc(id: &str, result: String) -> HttpOutcome {
    HttpOutcome { status: 200, body: format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{result}}}"#) + "\n" }
}
fn err_rpc(id: &str, code: i32, msg: &str) -> HttpOutcome {
    let id = if id.is_empty() { "null" } else { id };
    HttpOutcome { status: 200, body: format!(r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":{code},"message":"{msg}"}}}}"#) + "\n" }
}
fn json_str(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if !rest.starts_with('"') { return None; }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(c) = chars.next() {
        match c { '"' => break, '\\' => { if let Some(n) = chars.next() { out.push(n); } } _ => out.push(c) }
    }
    Some(out)
}
fn json_u64(text: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    rest.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()
}
fn json_raw_id(text: &str) -> String {
    let Some(i) = text.find("\"id\"") else { return "null".into() };
    let rest = text[i + 4..].trim_start();
    let rest = rest.strip_prefix(':').unwrap_or(rest).trim_start();
    if rest.starts_with('"') { return format!("\"{}\"", json_str(text, "id").unwrap_or_default()); }
    rest.chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect()
}
fn json_obj_after(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if !rest.starts_with('{') { return Some(rest.to_string()); }
    let mut depth = 0i32;
    for (idx, c) in rest.char_indices() {
        match c { '{' => depth += 1, '}' => { depth -= 1; if depth == 0 { return Some(rest[..=idx].to_string()); } } _ => {} }
    }
    Some(rest.to_string())
}

pub fn serve_mcp(root: &Path, port: u16, token: &str) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| ForgeError::Precondition(format!("bind 127.0.0.1:{port}: {e}")))?;
    for stream in listener.incoming() {
        if let Ok(mut s) = stream {
            let mut buf = vec![0u8; 65536];
            if let Ok(n) = s.read(&mut buf) {
                let raw = String::from_utf8_lossy(&buf[..n]);
                let (headers, body) = if let Some(i) = raw.find("\r\n\r\n") { (&raw[..i], &raw[i+4..]) } else { (raw.as_ref(), "") };
                let first = headers.lines().next().unwrap_or("");
                if !first.contains("/mcp") {
                    let _ = write_http(&mut s, 404, "not found\n");
                } else {
                    let auth = headers.lines().find_map(|l| l.split_once(':').and_then(|(k,v)| k.trim().eq_ignore_ascii_case("authorization").then(|| v.trim().to_string())));
                    let out = handle_http(root, token, auth.as_deref(), body);
                    let _ = write_http(&mut s, out.status, &out.body);
                }
            }
        }
    }
    Ok(())
}
fn write_http(s: &mut TcpStream, status: u16, body: &str) -> Result<()> {
    let reason = if status == 401 { "Unauthorized" } else if status == 202 { "Accepted" } else { "OK" };
    let head = format!("HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    s.write_all(head.as_bytes())?; s.write_all(body.as_bytes())?; Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intake::fy_do;
    use crate::outcome::validate_and_store;
    use crate::types::Agent;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("fy-mcp-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p); fs::create_dir_all(&p).unwrap(); p
    }
    #[test]
    fn bearer_missing_or_wrong_is_401_and_denied() {
        let root = tmp();
        let out = handle_http(&root, "secret-token-16xx", None, "{}");
        assert_eq!(out.status, 401);
        let log = fs::read_to_string(factory_events_path(&root)).unwrap();
        assert!(log.contains("\"hook\":\"mcp\"") && log.contains("\"status\":\"denied\""));
    }
    #[test]
    fn queue_work_matches_fy_do() {
        let a = tmp(); let b = tmp();
        fy_do(&a, "https://github.com/acme/toy/issues/4", "add login").unwrap();
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"queue_work","arguments":{"url":"https://github.com/acme/toy/issues/4","text":"add login"}}}"#;
        let out = handle_http(&b, "tokentokentoken16", Some("Bearer tokentokentoken16"), body);
        assert_eq!(out.status, 200, "{}", out.body);
        assert_eq!(fs::read_to_string(project_dir(&a,"toy").join("inbox.md")).unwrap(), fs::read_to_string(project_dir(&b,"toy").join("inbox.md")).unwrap());
    }
    #[test]
    fn project_status_bytes_equal_fy_status() {
        let root = tmp();
        fy_do(&root, "https://github.com/acme/toy", "x").unwrap();
        let want = render_status(&root, "toy").unwrap();
        let body = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_status","arguments":{"project":"toy"}}}"#;
        let out = handle_http(&root, "tokentokentoken16", Some("Bearer tokentokentoken16"), body);
        assert_eq!(out.status, 200);
        assert!(out.body.contains(&want.replace('\n', "\\n")), "{}", out.body);
    }
    #[test]
    fn last_outcome_reads_stored_file() {
        let root = tmp();
        fy_do(&root, "https://github.com/acme/toy", "x").unwrap();
        let stdout = r#"{"kind":"qa","pr":9,"verdict":"no_merge","summary":"fail"}"#;
        validate_and_store(&root, "toy", "20260101T000000Z-aaaa", Agent::Qa, "qa-on-cluster", stdout).unwrap();
        let body = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"last_outcome","arguments":{"project":"toy"}}}"#;
        let out = handle_http(&root, "tokentokentoken16", Some("Bearer tokentokentoken16"), body);
        assert_eq!(out.status, 200);
        assert!(out.body.contains("run_id=20260101T000000Z-aaaa"), "{}", out.body);
        assert!(out.body.contains("no_merge"), "{}", out.body);
    }
    #[test]
    fn tools_cannot_merge() { assert!(!TOOLS.iter().any(|t| t.contains("merge"))); }
}
