use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::error::{ForgeError, Result};

pub const CANONICAL: &[&str] = &[
    "telegram.bot_token", "llm.endpoint", "llm.default", "llm.tech-pm",
    "llm.developer", "llm.qa", "github.pat", "cluster.host", "cluster.user",
    "cluster.port", "cluster.password",
];

fn is_secret(name: &str) -> bool {
    matches!(name,
        "telegram.bot_token" | "llm.default" | "llm.tech-pm" | "llm.developer"
        | "llm.qa" | "github.pat" | "cluster.password")
}

fn is_token_like(name: &str) -> bool {
    is_secret(name) && name != "cluster.password"
}

#[derive(Debug, Clone, Default)]
pub struct Tokens {
    pub values: BTreeMap<String, String>,
    pub allow_user_ids: Vec<i64>,
}

impl Tokens {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(|s| s.as_str()).filter(|s| !s.is_empty())
    }
    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        if !CANONICAL.contains(&name) {
            return Err(ForgeError::Usage(format!("unknown token name: {name}")));
        }
        self.values.insert(name.to_string(), value.to_string());
        Ok(())
    }
    pub fn is_set(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
}

pub fn tokens_path(root: &Path) -> PathBuf {
    root.join("tokens").join("tokens.toml")
}
pub fn tokens_meta_path(root: &Path) -> PathBuf {
    root.join("tokens").join("tokens.meta.toml")
}

pub fn load_tokens(root: &Path) -> Result<Tokens> {
    let path = tokens_path(root);
    if !path.exists() {
        return Ok(Tokens::default());
    }
    Ok(parse_tokens(&fs::read_to_string(&path)?))
}

pub fn save_tokens(root: &Path, t: &Tokens, updated_by: &str) -> Result<()> {
    fs::create_dir_all(root.join("tokens"))?;
    let path = tokens_path(root);
    {
        let mut f = OpenOptions::new().create(true).write(true).truncate(true).open(&path)?;
        f.write_all(encode_tokens(t).as_bytes())?;
        f.sync_all()?;
    }
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms)?;
    write_meta(root, t, updated_by)
}

pub fn list_rows(t: &Tokens, meta: &Meta) -> String {
    let mut out = String::from("name                 set  tail  updated\n");
    for name in CANONICAL {
        let set = if t.is_set(name) { "yes" } else { "no " };
        let row = meta.rows.get(*name);
        let tail = row.map(|r| r.tail.as_str()).filter(|s| !s.is_empty()).unwrap_or("-");
        let updated = row.map(|r| r.updated_at.as_str()).filter(|s| !s.is_empty()).unwrap_or("-");
        out.push_str(&format!("{name:<20} {set}  {tail:<4}  {updated}\n"));
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct MetaRow {
    pub tail: String,
    pub set: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub rows: BTreeMap<String, MetaRow>,
    pub cluster_host: String,
    pub cluster_user: String,
}

pub fn load_meta(root: &Path) -> Result<Meta> {
    let path = tokens_meta_path(root);
    if !path.exists() {
        return Ok(Meta::default());
    }
    Ok(parse_meta(&fs::read_to_string(path)?))
}

fn write_meta(root: &Path, t: &Tokens, updated_by: &str) -> Result<()> {
    let now = format!("{}", {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    });
    let mut meta = load_meta(root).unwrap_or_default();
    for name in CANONICAL {
        let val = t.values.get(*name).cloned().unwrap_or_default();
        let set = if val.is_empty() { "no" } else { "yes" };
        let tail = if is_token_like(name) && !val.is_empty() { tail4(&val) } else { String::new() };
        let tail = if *name == "cluster.password" { String::new() } else { tail };
        meta.rows.insert((*name).into(), MetaRow {
            tail, set: set.into(), updated_at: now.clone(), updated_by: updated_by.into(),
        });
    }
    meta.cluster_host = t.values.get("cluster.host").cloned().unwrap_or_default();
    meta.cluster_user = t.values.get("cluster.user").cloned().unwrap_or_default();
    fs::write(tokens_meta_path(root), encode_meta(&meta))?;
    Ok(())
}

fn tail4(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let n = c.len().min(4);
    format!("..{}", c[c.len()-n..].iter().collect::<String>())
}

fn encode_tokens(t: &Tokens) -> String {
    let g = |k: &str| t.values.get(k).cloned().unwrap_or_default();
    let ids = t.allow_user_ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
    format!(
        "[telegram]\nbot_token = \"{}\"\nallow_user_ids = [{}]\n\n[llm]\nendpoint = \"{}\"\ndefault = \"{}\"\ntech-pm = \"{}\"\ndeveloper = \"{}\"\nqa = \"{}\"\n\n[github]\npat = \"{}\"\n\n[cluster]\nhost = \"{}\"\nuser = \"{}\"\nport = {}\npassword = \"{}\"\n",
        esc(&g("telegram.bot_token")), ids, esc(&g("llm.endpoint")), esc(&g("llm.default")),
        esc(&g("llm.tech-pm")), esc(&g("llm.developer")), esc(&g("llm.qa")),
        esc(&g("github.pat")), esc(&g("cluster.host")), esc(&g("cluster.user")),
        g("cluster.port").parse::<u16>().unwrap_or(22), esc(&g("cluster.password")),
    )
}

fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"") }

fn parse_tokens(text: &str) -> Tokens {
    let mut t = Tokens::default();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len()-1].to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue; };
        let k = k.trim();
        let v = unquote(v.trim());
        if section == "telegram" && k == "allow_user_ids" {
            t.allow_user_ids = v.trim_matches(|c| c == '[' || c == ']').split(',').filter_map(|s| s.trim().parse().ok()).collect();
            continue;
        }
        let name = match (section.as_str(), k) {
            ("telegram", "bot_token") => "telegram.bot_token",
            ("llm", "endpoint") => "llm.endpoint",
            ("llm", "default") => "llm.default",
            ("llm", "tech-pm") => "llm.tech-pm",
            ("llm", "developer") => "llm.developer",
            ("llm", "qa") => "llm.qa",
            ("github", "pat") => "github.pat",
            ("cluster", "host") => "cluster.host",
            ("cluster", "user") => "cluster.user",
            ("cluster", "port") => "cluster.port",
            ("cluster", "password") => "cluster.password",
            _ => continue,
        };
        t.values.insert(name.into(), v);
    }
    t
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len()-1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        s.to_string()
    }
}

fn encode_meta(m: &Meta) -> String {
    let mut s = String::from("# generated; no secret values\n");
    if !m.cluster_host.is_empty() {
        s.push_str(&format!("cluster_host = \"{}\"\n", esc(&m.cluster_host)));
    }
    if !m.cluster_user.is_empty() {
        s.push_str(&format!("cluster_user = \"{}\"\n", esc(&m.cluster_user)));
    }
    for (name, row) in &m.rows {
        s.push_str(&format!(
            "\n[\"{name}\"]\nset = \"{}\"\ntail = \"{}\"\nupdated_at = \"{}\"\nupdated_by = \"{}\"\n",
            row.set, esc(&row.tail), esc(&row.updated_at), esc(&row.updated_by)
        ));
    }
    s
}

fn parse_meta(text: &str) -> Meta {
    let mut m = Meta::default();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if line.starts_with('[') {
            section = line.trim_matches(|c| c == '[' || c == ']' || c == '"').to_string();
            m.rows.entry(section.clone()).or_default();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue; };
        let k = k.trim();
        let v = unquote(v.trim());
        if section.is_empty() {
            if k == "cluster_host" { m.cluster_host = v; }
            else if k == "cluster_user" { m.cluster_user = v; }
            continue;
        }
        if let Some(row) = m.rows.get_mut(&section) {
            match k {
                "set" => row.set = v,
                "tail" => row.tail = v,
                "updated_at" => row.updated_at = v,
                "updated_by" => row.updated_by = v,
                _ => {}
            }
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-tok-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn save_mode_is_0600() {
        let root = tmp();
        let mut t = Tokens::default();
        t.set("github.pat", "ghp_abcdefghijklmnop").unwrap();
        save_tokens(&root, &t, "test").unwrap();
        let mode = fs::metadata(tokens_path(&root)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    #[test]
    fn meta_has_tail_not_full_secret() {
        let root = tmp();
        let mut t = Tokens::default();
        t.set("github.pat", "ghp_abcdefghijklmnop").unwrap();
        t.set("cluster.password", "super-secret-pass").unwrap();
        t.set("cluster.host", "10.0.0.1").unwrap();
        save_tokens(&root, &t, "human").unwrap();
        let meta = fs::read_to_string(tokens_meta_path(&root)).unwrap();
        assert!(!meta.contains("ghp_abcdefghijklmnop"));
        assert!(!meta.contains("super-secret-pass"));
        let parsed = load_meta(&root).unwrap();
        let pw = parsed.rows.get("cluster.password").unwrap();
        assert_eq!(pw.set, "yes");
        assert!(pw.tail.is_empty());
        assert_eq!(parsed.cluster_host, "10.0.0.1");
    }
    #[test]
    fn empty_string_is_unset() {
        let mut t = Tokens::default();
        t.set("llm.default", "").unwrap();
        assert!(!t.is_set("llm.default"));
    }
    #[test]
    fn list_rows_has_no_raw_secret() {
        let mut t = Tokens::default();
        t.set("llm.default", "sk-SECRETVALUE9999").unwrap();
        let mut meta = Meta::default();
        meta.rows.insert("llm.default".into(), MetaRow { tail: "..9999".into(), set: "yes".into(), updated_at: "1".into(), updated_by: "t".into() });
        let table = list_rows(&t, &meta);
        assert!(!table.contains("sk-SECRETVALUE9999"));
        assert!(table.contains("llm.default"));
    }
}
