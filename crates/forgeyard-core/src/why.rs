use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::daemon::{pid_alive, read_pid, watch_pid_path};
use crate::error::Result;
use crate::events::{append_event, decode_event_line, Event};
use crate::lock::{why_lock_path, FileLock};
use crate::outcome::{latest_outcomes, Outcome};
use crate::paths::project_dir;
use crate::spec_cache::{load_spec_source, spec_cache_present, spec_cache_stale};
use crate::state::load_state;
use crate::types::{Agent, Hook};
use crate::watch::{Action, Plan, SpecVerdict, WatchIo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Why {
    pub schema: u32,
    pub ts: String,
    pub project: String,
    pub action: String,
    pub reason: String,
    pub busy: bool,
    pub spec_present: bool,
    pub spec_stale: bool,
    pub spec_sha: String,
    pub spec_verdict: String,
    pub plan_item: String,
    pub plan_status: String,
    pub cycles: u32,
    pub crashes: u32,
    pub pr_open: bool,
    pub last_outcome: String,
    pub run_id: String,
    pub watch_pid: u32,
}

impl Why {
    pub fn empty(project: &str) -> Self {
        Why {
            schema: 1,
            ts: String::new(),
            project: project.to_string(),
            action: String::new(),
            reason: String::new(),
            busy: false,
            spec_present: false,
            spec_stale: false,
            spec_sha: String::new(),
            spec_verdict: String::new(),
            plan_item: String::new(),
            plan_status: String::new(),
            cycles: 0,
            crashes: 0,
            pr_open: false,
            last_outcome: String::new(),
            run_id: String::new(),
            watch_pid: 0,
        }
    }
}

pub fn why_path(root: &Path, project: &str) -> std::path::PathBuf {
    project_dir(root, project).join("why.json")
}

pub fn action_name(action: &Action) -> &'static str {
    match action {
        Action::SleepBusy => "SleepBusy",
        Action::BlockedOnSpec => "BlockedOnSpec",
        Action::BlockedOnSpecRefresh => "BlockedOnSpecRefresh",
        Action::RunTechPm => "RunTechPm",
        Action::SeedPlan => "SeedPlan",
        Action::RunImplement { .. } => "RunImplement",
        Action::RunQa { .. } => "RunQa",
        Action::MergePr { .. } => "MergePr",
        Action::PlanDone => "PlanDone",
        Action::RetryBlocked { .. } => "RetryBlocked",
    }
}

pub fn reason_for(action: &Action, io: &dyn WatchIo, plan: &Plan) -> &'static str {
    match action {
        Action::SleepBusy => "busy",
        Action::BlockedOnSpec => "spec_missing",
        Action::BlockedOnSpecRefresh => "spec_stale",
        Action::RunTechPm => match io.spec_verdict() {
            Some(SpecVerdict::Reject) => "spec_rejected",
            _ => "spec_unreviewed",
        },
        Action::SeedPlan => "plan_seeded",
        Action::PlanDone => "plan_done",
        Action::RunImplement { item } => {
            if let Some(it) = plan.items.iter().find(|i| i.id == *item) {
                if !io.pr_open(&it.branch) && it.crashes > 0 {
                    return "no_pr";
                }
            }
            "run_implement"
        }
        Action::RunQa { .. } => "run_qa",
        Action::MergePr { .. } => "merge_pr",
        Action::RetryBlocked { .. } => "retry_blocked",
    }
}

pub fn format_last_outcome(root: &Path, project: &str) -> String {
    match latest_outcomes(root, project).into_iter().last() {
        Some((_, Outcome::SpecReview { spec_sha256, verdict, .. })) => {
            let v = match verdict {
                SpecVerdict::Approve => "approve",
                crate::watch::SpecVerdict::Reject => "reject",
            };
            format!("spec_review/{v}@{spec_sha256}")
        }
        Some((_, Outcome::Implementation { branch, .. })) => format!("implementation/@{branch}"),
        Some((_, Outcome::Qa { pr, verdict, .. })) => {
            let v = match verdict {
                crate::watch::PrVerdict::Merge => "merge",
                crate::watch::PrVerdict::NoMerge => "no_merge",
            };
            format!("qa/{v}@{pr}")
        }
        None => String::new(),
    }
}

pub fn load_why(root: &Path, project: &str) -> Option<Why> {
    let text = fs::read_to_string(why_path(root, project)).ok()?;
    decode_why(&text)
}

pub fn save_why(root: &Path, why: &Why) -> Result<()> {
    let dir = project_dir(root, &why.project);
    fs::create_dir_all(&dir)?;
    let _lock = FileLock::acquire(&why_lock_path(&dir))?;
    let path = why_path(root, &why.project);
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = OpenOptions::new().create(true).write(true).truncate(true).open(&tmp)?;
        f.write_all(encode_why(why).as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn encode_why(w: &Why) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ts\": \"{}\",\n  \"project\": \"{}\",\n  \"action\": \"{}\",\n  \"reason\": \"{}\",\n  \"busy\": {},\n  \"spec_present\": {},\n  \"spec_stale\": {},\n  \"spec_sha\": \"{}\",\n  \"spec_verdict\": \"{}\",\n  \"plan_item\": \"{}\",\n  \"plan_status\": \"{}\",\n  \"cycles\": {},\n  \"crashes\": {},\n  \"pr_open\": {},\n  \"last_outcome\": \"{}\",\n  \"run_id\": \"{}\",\n  \"watch_pid\": {}\n}}\n",
        w.schema,
        esc(&w.ts),
        esc(&w.project),
        esc(&w.action),
        esc(&w.reason),
        if w.busy { "true" } else { "false" },
        if w.spec_present { "true" } else { "false" },
        if w.spec_stale { "true" } else { "false" },
        esc(&w.spec_sha),
        esc(&w.spec_verdict),
        esc(&w.plan_item),
        esc(&w.plan_status),
        w.cycles,
        w.crashes,
        if w.pr_open { "true" } else { "false" },
        esc(&w.last_outcome),
        esc(&w.run_id),
        w.watch_pid
    )
}

pub fn decode_why(text: &str) -> Option<Why> {
    Some(Why {
        schema: json_u32(text, "schema").unwrap_or(1),
        ts: json_str(text, "ts").unwrap_or_default(),
        project: json_str(text, "project")?,
        action: json_str(text, "action").unwrap_or_default(),
        reason: json_str(text, "reason").unwrap_or_default(),
        busy: json_bool(text, "busy"),
        spec_present: json_bool(text, "spec_present"),
        spec_stale: json_bool(text, "spec_stale"),
        spec_sha: json_str(text, "spec_sha").unwrap_or_default(),
        spec_verdict: json_str(text, "spec_verdict").unwrap_or_default(),
        plan_item: json_str(text, "plan_item").unwrap_or_default(),
        plan_status: json_str(text, "plan_status").unwrap_or_default(),
        cycles: json_u32(text, "cycles").unwrap_or(0),
        crashes: json_u32(text, "crashes").unwrap_or(0),
        pr_open: json_bool(text, "pr_open"),
        last_outcome: json_str(text, "last_outcome").unwrap_or_default(),
        run_id: json_str(text, "run_id").unwrap_or_default(),
        watch_pid: json_u32(text, "watch_pid").unwrap_or(0),
    })
}

/// Write why.json after a tick. Append tick event only when action changed.
/// Returns true when the action changed (including first tick).
pub fn record_tick(
    root: &Path,
    project: &str,
    plan: &Plan,
    action: &Action,
    io: &dyn WatchIo,
) -> Result<bool> {
    let prev = load_why(root, project);
    let name = action_name(action).to_string();
    let reason = reason_for(action, io, plan).to_string();
    let changed = prev.as_ref().map(|p| p.action != name).unwrap_or(true);

    let item = plan
        .in_flight()
        .and_then(|i| plan.items.get(i))
        .or_else(|| plan.items.last());
    let branch = item.map(|i| i.branch.as_str()).unwrap_or("");
    let verdict = match io.spec_verdict() {
        Some(SpecVerdict::Approve) => "approve",
        Some(SpecVerdict::Reject) => "reject",
        None => "",
    };
    let sha = load_spec_source(root, project)
        .map(|s| s.content_sha256)
        .unwrap_or_default();
    let run_id = load_state(root, project)
        .ok()
        .map(|s| s.run_id)
        .unwrap_or_default();

    let snap = Why {
        schema: 1,
        ts: now_rfc3339(),
        project: project.to_string(),
        action: name.clone(),
        reason: reason.clone(),
        busy: io.busy(),
        spec_present: io.spec_present(),
        spec_stale: io.spec_stale(),
        spec_sha: sha,
        spec_verdict: verdict.into(),
        plan_item: item.map(|i| i.id.clone()).unwrap_or_default(),
        plan_status: item.map(|i| i.status.as_str().to_string()).unwrap_or_default(),
        cycles: item.map(|i| i.cycles).unwrap_or(0),
        crashes: item.map(|i| i.crashes).unwrap_or(0),
        pr_open: if branch.is_empty() { false } else { io.pr_open(branch) },
        last_outcome: format_last_outcome(root, project),
        run_id,
        watch_pid: std::process::id(),
    };
    save_why(root, &snap)?;

    if changed {
        let mut ev = Event::new(project, Hook::Run);
        ev.agent = Agent::Watch;
        ev.step = "tick".into();
        ev.status = name;
        ev.summary = reason.clone();
        append_event(root, &ev)?;
        if std::env::var("FORGEYARD_WATCH_TRACE").ok().as_deref() == Some("1") {
            eprintln!("{} {} {}", project, snap.action, reason);
        }
    }
    Ok(changed)
}

pub fn render_why(root: &Path, project: &str) -> Result<String> {
    let dir = project_dir(root, project);
    if !dir.join("PROJECT.toml").exists() {
        return Err(crate::error::ForgeError::Precondition(format!(
            "unknown project: {project}"
        )));
    }
    let state = load_state(root, project).unwrap_or_else(|_| crate::state::State::idle(project));
    let why = load_why(root, project);
    let watch = match read_pid(&watch_pid_path(root)) {
        Some(pid) if pid_alive(pid) => "up",
        _ => "down",
    };
    let spec = spec_line(root, project);
    let plan = crate::watch::load_plan(root, project).unwrap_or(Plan {
        default_branch: "main".into(),
        items: vec![],
    });
    let plan_line = match plan.in_flight().and_then(|i| plan.items.get(i)).or_else(|| {
        plan.items.iter().find(|i| {
            !matches!(
                i.status,
                crate::watch::ItemStatus::Done | crate::watch::ItemStatus::Merged
            )
        })
    }) {
        Some(it) => format!(
            "item={} status={} cycles={} crashes={}",
            it.id,
            it.status.as_str(),
            it.cycles,
            it.crashes
        ),
        None => "-".into(),
    };
    let (action, reason, updated, pr, outcome) = match &why {
        Some(w) => (
            dash(&w.action),
            dash(&w.reason),
            dash(&w.ts),
            if w.pr_open { "yes" } else { "no" }.to_string(),
            dash(&w.last_outcome),
        ),
        None => ("-".into(), "-".into(), "-".into(), "-".into(), "-".into()),
    };
    let stop = last_stop_line(&dir);
    let error = if !state.run_id.is_empty() {
        let p = dir.join("runs").join(&state.run_id).join("outcome.error");
        if p.exists() {
            format!("runs/{}/outcome.error", state.run_id)
        } else {
            "-".into()
        }
    } else {
        "-".into()
    };
    let mut out = String::from("forgeyard why\n");
    out.push_str(&row("project", project));
    out.push_str(&row("watch", watch));
    out.push_str(&row("busy", if state.is_busy() { "yes" } else { "no" }));
    out.push_str(&row("workflow", dash(&state.workflow).as_str()));
    out.push_str(&row("step", dash(&state.step).as_str()));
    out.push_str(&row("action", &action));
    out.push_str(&row("reason", &reason));
    out.push_str(&row("spec", &spec));
    out.push_str(&row("plan", &plan_line));
    out.push_str(&row("pr", &pr));
    out.push_str(&row("outcome", &outcome));
    out.push_str(&row("run", dash(&state.run_id).as_str()));
    out.push_str(&row("stop", &stop));
    out.push_str(&row("error", &error));
    out.push_str(&row("updated", &updated));
    Ok(out)
}

fn spec_line(root: &Path, project: &str) -> String {
    let present = spec_cache_present(root, project);
    let stale = spec_cache_stale(root, project);
    let sha = load_spec_source(root, project)
        .map(|s| s.content_sha256)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "-".into());
    let state = if present { "present" } else { "missing" };
    let fresh = if stale { "stale" } else { "fresh" };
    format!("{state} {fresh} sha={sha}")
}

fn last_stop_line(dir: &Path) -> String {
    let path = dir.join("events.jsonl");
    let Ok(text) = fs::read_to_string(path) else {
        return "-".into();
    };
    for line in text.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(ev) = decode_event_line(line) {
            if ev.hook == Hook::Stop {
                let mut s = ev.status;
                if !ev.summary.is_empty() {
                    s.push(' ');
                    s.push_str(&ev.summary);
                }
                return if s.is_empty() { "-".into() } else { s };
            }
        }
    }
    "-".into()
}

fn row(key: &str, val: &str) -> String {
    format!("{:<12}{}\n", format!("{key}:"), val)
}

fn dash(s: &str) -> String {
    if s.is_empty() {
        "-".into()
    } else {
        s.to_string()
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}

fn json_str(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let bytes = rest[1..].as_bytes();
    let mut j = 0;
    while j < bytes.len() {
        match bytes[j] {
            b'"' => return Some(out),
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
    None
}

fn json_u32(text: &str, key: &str) -> Option<u32> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = text[i + pat.len()..].trim_start().strip_prefix(':')?.trim_start();
    let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    n.parse().ok()
}

fn json_bool(text: &str, key: &str) -> bool {
    let pat = format!("\"{key}\"");
    if let Some(i) = text.find(&pat) {
        let rest = text[i + pat.len()..].trim_start();
        if let Some(rest) = rest.strip_prefix(':') {
            return rest.trim_start().starts_with("true");
        }
    }
    false
}

fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut s = secs;
    let rem = s % 86400;
    s /= 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let mut year: i32 = 1970;
    loop {
        let ydays: u64 = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if s >= ydays {
            s -= ydays;
            year += 1;
        } else {
            break;
        }
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let mdays = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for d in mdays {
        if s >= d {
            s -= d;
            month += 1;
        } else {
            break;
        }
    }
    let day = s + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::bind_project;
    use crate::events::events_path;
    use crate::watch::{ItemStatus, PlanItem, SpecVerdict};
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-why-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    struct Fake {
        busy: bool,
        spec: bool,
        stale: bool,
        verdict: Option<SpecVerdict>,
        pr: bool,
    }
    impl WatchIo for Fake {
        fn busy(&self) -> bool {
            self.busy
        }
        fn spec_present(&self) -> bool {
            self.spec
        }
        fn spec_stale(&self) -> bool {
            self.stale
        }
        fn spec_verdict(&self) -> Option<SpecVerdict> {
            self.verdict
        }
        fn pr_open(&self, _b: &str) -> bool {
            self.pr
        }
        fn pr_verdict(&self, _b: &str) -> Option<crate::watch::PrVerdict> {
            None
        }
        fn default_branch(&self) -> String {
            "main".into()
        }
    }

    fn plan() -> Plan {
        Plan {
            default_branch: "main".into(),
            items: vec![],
        }
    }

    #[test]
    fn one_tick_writes_why() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let io = Fake {
            busy: false,
            spec: false,
            stale: false,
            verdict: None,
            pr: false,
        };
        let mut p = plan();
        let act = crate::watch::tick(&mut p, &io);
        assert_eq!(act, Action::BlockedOnSpec);
        record_tick(&root, "toy", &p, &act, &io).unwrap();
        let w = load_why(&root, "toy").unwrap();
        assert_eq!(w.action, "BlockedOnSpec");
        assert_eq!(w.reason, "spec_missing");
    }

    #[test]
    fn same_action_does_not_append_tick_event() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let io = Fake {
            busy: false,
            spec: false,
            stale: false,
            verdict: None,
            pr: false,
        };
        let p = plan();
        let act = Action::BlockedOnSpec;
        record_tick(&root, "toy", &p, &act, &io).unwrap();
        record_tick(&root, "toy", &p, &act, &io).unwrap();
        let log = fs::read_to_string(events_path(&project_dir(&root, "toy"))).unwrap();
        let ticks = log.lines().filter(|l| l.contains("\"step\":\"tick\"")).count();
        assert_eq!(ticks, 1);
    }

    #[test]
    fn action_change_appends_one_tick_line() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let io = Fake {
            busy: false,
            spec: true,
            stale: true,
            verdict: None,
            pr: false,
        };
        let p = plan();
        record_tick(&root, "toy", &p, &Action::BlockedOnSpec, &io).unwrap();
        record_tick(&root, "toy", &p, &Action::BlockedOnSpecRefresh, &io).unwrap();
        let log = fs::read_to_string(events_path(&project_dir(&root, "toy"))).unwrap();
        let ticks: Vec<_> = log
            .lines()
            .filter(|l| l.contains("\"step\":\"tick\""))
            .collect();
        assert_eq!(ticks.len(), 2);
        assert!(ticks[1].contains("BlockedOnSpecRefresh"));
    }

    #[test]
    fn render_without_why_is_dashes() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let block = render_why(&root, "toy").unwrap();
        assert!(block.starts_with("forgeyard why\n"));
        assert!(block.contains("action:     -"));
        assert!(block.contains("watch:      down"));
    }

    #[test]
    fn render_golden_with_fixture() {
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let dir = project_dir(&root, "toy");
        fs::write(
            dir.join("spec.md"),
            "# spec\n",
        )
        .unwrap();
        fs::write(
            dir.join("spec-source.json"),
            "{\"owner\":\"acme\",\"repo\":\"toy\",\"branch\":\"main\",\"commit_sha\":\"c\",\"fetched_at\":\"1\",\"content_sha256\":\"abc\",\"stale\":true,\"stale_reason\":\"x\"}\n",
        )
        .unwrap();
        let mut w = Why::empty("toy");
        w.ts = "2026-09-15T05:00:00Z".into();
        w.action = "BlockedOnSpecRefresh".into();
        w.reason = "spec_stale".into();
        w.spec_present = true;
        w.spec_stale = true;
        w.spec_sha = "abc".into();
        w.last_outcome = "spec_review/approve@old".into();
        w.pr_open = false;
        save_why(&root, &w).unwrap();
        fs::write(
            dir.join("plan.json"),
            "default_branch = \"main\"\n\n[[item]]\nid = \"1\"\ntitle = \"t\"\nbranch = \"fy/main-1\"\nstatus = \"ready\"\ncrashes = 0\ncycles = 1\n",
        )
        .unwrap();
        fs::write(
            dir.join("state.json"),
            "{\n  \"schema\": 1,\n  \"project\": \"toy\",\n  \"workflow\": \"spec-review\",\n  \"step\": \"review\",\n  \"agent\": \"watch\",\n  \"agent_status\": \"idle\",\n  \"run_id\": \"20260915T050000Z-ab12\",\n  \"input\": \"\",\n  \"spec_path\": \"spec.md\",\n  \"updated_at\": \"2026-09-15T05:00:00Z\"\n}\n",
        )
        .unwrap();
        fs::write(
            dir.join("events.jsonl"),
            "{\"ts\":\"2026-09-15T05:00:00Z\",\"project\":\"toy\",\"hook\":\"stop\",\"status\":\"fail\",\"summary\":\"outcome_invalid\"}\n",
        )
        .unwrap();
        let run = dir.join("runs").join("20260915T050000Z-ab12");
        fs::create_dir_all(&run).unwrap();
        fs::write(run.join("outcome.error"), "reason: missing kind\nexcerpt: nope\n").unwrap();
        let block = render_why(&root, "toy").unwrap();
        let expect = "\
forgeyard why
project:    toy
watch:      down
busy:       no
workflow:   spec-review
step:       review
action:     BlockedOnSpecRefresh
reason:     spec_stale
spec:       present stale sha=abc
plan:       item=1 status=ready cycles=1 crashes=0
pr:         no
outcome:    spec_review/approve@old
run:        20260915T050000Z-ab12
stop:       fail outcome_invalid
error:      runs/20260915T050000Z-ab12/outcome.error
updated:    2026-09-15T05:00:00Z
";
        assert_eq!(block, expect);
    }

    #[test]
    fn deleting_why_does_not_change_tick() {
        let io = Fake {
            busy: false,
            spec: true,
            stale: false,
            verdict: Some(SpecVerdict::Approve),
            pr: false,
        };
        let mut p = Plan {
            default_branch: String::new(),
            items: vec![],
        };
        assert_eq!(crate::watch::tick(&mut p, &io), Action::SeedPlan);
        let root = tmp();
        bind_project(&root, "https://github.com/acme/toy").unwrap();
        let _ = fs::remove_file(why_path(&root, "toy"));
        let mut p2 = Plan {
            default_branch: String::new(),
            items: vec![],
        };
        assert_eq!(crate::watch::tick(&mut p2, &io), Action::SeedPlan);
    }

    #[test]
    fn implement_without_pr_after_crash_is_no_pr() {
        let io = Fake {
            busy: false,
            spec: true,
            stale: false,
            verdict: Some(SpecVerdict::Approve),
            pr: false,
        };
        let p = Plan {
            default_branch: "main".into(),
            items: vec![PlanItem {
                id: "1".into(),
                title: "t".into(),
                branch: "fy/main-1".into(),
                status: ItemStatus::Implementing,
                crashes: 1,
                cycles: 1,
            }],
        };
        assert_eq!(
            reason_for(&Action::RunImplement { item: "1".into() }, &io, &p),
            "no_pr"
        );
    }
}
