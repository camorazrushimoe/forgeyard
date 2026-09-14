use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::paths::project_dir;
use crate::state::load_state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    Ready, Implementing, PrOpen, QaRunning, NoMerge, Merging, Merged, Done, Blocked,
}
impl ItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemStatus::Ready => "ready",
            ItemStatus::Implementing => "implementing",
            ItemStatus::PrOpen => "pr_open",
            ItemStatus::QaRunning => "qa_running",
            ItemStatus::NoMerge => "no_merge",
            ItemStatus::Merging => "merging",
            ItemStatus::Merged => "merged",
            ItemStatus::Done => "done",
            ItemStatus::Blocked => "blocked",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ready" => ItemStatus::Ready,
            "implementing" => ItemStatus::Implementing,
            "pr_open" => ItemStatus::PrOpen,
            "qa_running" => ItemStatus::QaRunning,
            "no_merge" => ItemStatus::NoMerge,
            "merging" => ItemStatus::Merging,
            "merged" => ItemStatus::Merged,
            "done" => ItemStatus::Done,
            "blocked" => ItemStatus::Blocked,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanItem {
    pub id: String,
    pub title: String,
    pub branch: String,
    pub status: ItemStatus,
    pub crashes: u32,
    pub cycles: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub default_branch: String,
    pub items: Vec<PlanItem>,
}

impl Plan {
    pub fn in_flight(&self) -> Option<usize> {
        self.items.iter().position(|i| !matches!(i.status, ItemStatus::Done | ItemStatus::Merged | ItemStatus::Blocked))
    }
    pub fn complete(&self) -> bool {
        !self.items.is_empty() && self.items.iter().all(|i| matches!(i.status, ItemStatus::Done | ItemStatus::Merged))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVerdict { Approve, Reject }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrVerdict { Merge, NoMerge }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    SleepBusy,
    BlockedOnSpec,
    BlockedOnSpecRefresh,
    RunTechPm,
    SeedPlan,
    RunImplement { item: String },
    RunQa { item: String },
    MergePr { item: String },
    PlanDone,
    RetryBlocked { item: String },
}

pub trait WatchIo {
    fn busy(&self) -> bool;
    fn spec_present(&self) -> bool;
    fn spec_stale(&self) -> bool;
    fn spec_verdict(&self) -> Option<SpecVerdict>;
    fn pr_open(&self, branch: &str) -> bool;
    fn pr_verdict(&self, branch: &str) -> Option<PrVerdict>;
    fn default_branch(&self) -> String;
}

pub fn tick(plan: &mut Plan, io: &dyn WatchIo) -> Action {
    if io.busy() { return Action::SleepBusy; }
    if !io.spec_present() { return Action::BlockedOnSpec; }
    if io.spec_stale() { return Action::BlockedOnSpecRefresh; }
    match io.spec_verdict() {
        None | Some(SpecVerdict::Reject) => return Action::RunTechPm,
        Some(SpecVerdict::Approve) => {}
    }
    if plan.items.is_empty() {
        let b = io.default_branch();
        plan.default_branch = b.clone();
        plan.items.push(PlanItem {
            id: "1".into(), title: "implement spec".into(),
            branch: format!("fy/{b}-1"), status: ItemStatus::Ready, crashes: 0, cycles: 0,
        });
        return Action::SeedPlan;
    }
    if plan.complete() { return Action::PlanDone; }
    let idx = match plan.in_flight() { Some(i) => i, None => return Action::PlanDone };
    let id = plan.items[idx].id.clone();
    let branch = plan.items[idx].branch.clone();
    match plan.items[idx].status {
        ItemStatus::Ready | ItemStatus::NoMerge => {
            if plan.items[idx].cycles >= 5 {
                plan.items[idx].status = ItemStatus::Blocked;
                return Action::RetryBlocked { item: id };
            }
            plan.items[idx].cycles += 1;
            plan.items[idx].status = ItemStatus::Implementing;
            Action::RunImplement { item: id }
        }
        ItemStatus::Implementing => {
            if io.pr_open(&branch) {
                plan.items[idx].status = ItemStatus::PrOpen;
                Action::RunQa { item: id }
            } else if plan.items[idx].crashes >= 3 {
                plan.items[idx].status = ItemStatus::Blocked;
                Action::RetryBlocked { item: id }
            } else {
                plan.items[idx].crashes += 1;
                plan.items[idx].status = ItemStatus::Ready;
                Action::RunImplement { item: id }
            }
        }
        ItemStatus::PrOpen => {
            plan.items[idx].status = ItemStatus::QaRunning;
            Action::RunQa { item: id }
        }
        ItemStatus::QaRunning => match io.pr_verdict(&branch) {
            Some(PrVerdict::Merge) => {
                plan.items[idx].status = ItemStatus::Merging;
                Action::MergePr { item: id }
            }
            Some(PrVerdict::NoMerge) => {
                plan.items[idx].status = ItemStatus::NoMerge;
                Action::RunImplement { item: id }
            }
            None => {
                if plan.items[idx].crashes >= 3 {
                    plan.items[idx].status = ItemStatus::Blocked;
                    Action::RetryBlocked { item: id }
                } else {
                    plan.items[idx].crashes += 1;
                    Action::RunQa { item: id }
                }
            }
        },
        ItemStatus::Merging => {
            plan.items[idx].status = ItemStatus::Done;
            Action::MergePr { item: id }
        }
        ItemStatus::Merged => {
            plan.items[idx].status = ItemStatus::Done;
            Action::PlanDone
        }
        ItemStatus::Done | ItemStatus::Blocked => Action::PlanDone,
    }
}

pub fn encode_plan(p: &Plan) -> String {
    let mut s = format!("default_branch = \"{}\"\n", p.default_branch);
    for it in &p.items {
        s.push_str(&format!(
            "\n[[item]]\nid = \"{}\"\ntitle = \"{}\"\nbranch = \"{}\"\nstatus = \"{}\"\ncrashes = {}\ncycles = {}\n",
            it.id, it.title, it.branch, it.status.as_str(), it.crashes, it.cycles
        ));
    }
    s
}

pub fn load_plan(root: &Path, project: &str) -> Result<Plan> {
    let path = project_dir(root, project).join("plan.json");
    if !path.exists() {
        return Ok(Plan { default_branch: "main".into(), items: vec![] });
    }
    Ok(decode_plan(&fs::read_to_string(path)?))
}

pub fn save_plan(root: &Path, project: &str, plan: &Plan) -> Result<()> {
    let dir = project_dir(root, project);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("plan.json"), encode_plan(plan))?;
    Ok(())
}

fn decode_plan(text: &str) -> Plan {
    let mut plan = Plan { default_branch: "main".into(), items: vec![] };
    let mut cur: Option<PlanItem> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("[[item]]") {
            if let Some(it) = cur.take() { plan.items.push(it); }
            cur = Some(PlanItem { id: String::new(), title: String::new(), branch: String::new(), status: ItemStatus::Ready, crashes: 0, cycles: 0 });
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim();
        let v = v.trim().trim_matches('"').to_string();
        if cur.is_none() && k == "default_branch" { plan.default_branch = v; continue; }
        if let Some(it) = cur.as_mut() {
            match k {
                "id" => it.id = v,
                "title" => it.title = v,
                "branch" => it.branch = v,
                "status" => it.status = ItemStatus::parse(&v).unwrap_or(ItemStatus::Ready),
                "crashes" => it.crashes = v.parse().unwrap_or(0),
                "cycles" => it.cycles = v.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    if let Some(it) = cur { plan.items.push(it); }
    plan
}

pub fn project_busy(root: &Path, project: &str) -> bool {
    load_state(root, project).map(|s| s.is_busy()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake { busy: bool, spec: bool, stale: bool, verdict: Option<SpecVerdict>, pr: bool, pr_v: Option<PrVerdict>, def: String }
    impl WatchIo for Fake {
        fn busy(&self) -> bool { self.busy }
        fn spec_present(&self) -> bool { self.spec }
        fn spec_stale(&self) -> bool { self.stale }
        fn spec_verdict(&self) -> Option<SpecVerdict> { self.verdict }
        fn pr_open(&self, _b: &str) -> bool { self.pr }
        fn pr_verdict(&self, _b: &str) -> Option<PrVerdict> { self.pr_v }
        fn default_branch(&self) -> String { self.def.clone() }
    }
    fn base() -> Fake { Fake { busy: false, spec: true, stale: false, verdict: Some(SpecVerdict::Approve), pr: false, pr_v: None, def: "main".into() } }
    fn item(st: ItemStatus, cycles: u32) -> Plan {
        Plan { default_branch: "main".into(), items: vec![PlanItem { id: "1".into(), title: "t".into(), branch: "fy/main-1".into(), status: st, crashes: 0, cycles }] }
    }
    #[test]
    fn busy_sleeps() { let mut p = Plan { default_branch: "main".into(), items: vec![] }; let mut io = base(); io.busy = true; assert_eq!(tick(&mut p, &io), Action::SleepBusy); }
    #[test]
    fn no_spec_blocks() { let mut p = Plan { default_branch: "main".into(), items: vec![] }; let mut io = base(); io.spec = false; assert_eq!(tick(&mut p, &io), Action::BlockedOnSpec); }
    #[test]
    fn unreviewed_spec_is_a() { let mut p = Plan { default_branch: "main".into(), items: vec![] }; let mut io = base(); io.verdict = None; assert_eq!(tick(&mut p, &io), Action::RunTechPm); }
    #[test]
    fn approved_no_plan_seeds_b() { let mut p = Plan { default_branch: String::new(), items: vec![] }; assert_eq!(tick(&mut p, &base()), Action::SeedPlan); assert_eq!(p.items[0].status, ItemStatus::Ready); }
    #[test]
    fn ready_goes_implement() { let mut p = item(ItemStatus::Ready, 0); assert_eq!(tick(&mut p, &base()), Action::RunImplement { item: "1".into() }); }
    #[test]
    fn implementing_with_pr_starts_qa() { let mut p = item(ItemStatus::Implementing, 1); let mut io = base(); io.pr = true; assert_eq!(tick(&mut p, &io), Action::RunQa { item: "1".into() }); }
    #[test]
    fn qa_merge_verdict_merges() { let mut p = item(ItemStatus::QaRunning, 1); let mut io = base(); io.pr_v = Some(PrVerdict::Merge); assert_eq!(tick(&mut p, &io), Action::MergePr { item: "1".into() }); }
    #[test]
    fn qa_no_merge_returns_to_implement() { let mut p = item(ItemStatus::QaRunning, 1); let mut io = base(); io.pr_v = Some(PrVerdict::NoMerge); assert_eq!(tick(&mut p, &io), Action::RunImplement { item: "1".into() }); }
    #[test]
    fn implement_cap_blocks() { let mut p = item(ItemStatus::Ready, 5); assert_eq!(tick(&mut p, &base()), Action::RetryBlocked { item: "1".into() }); }
    #[test]
    fn complete_plan_is_d() { let mut p = item(ItemStatus::Done, 1); assert_eq!(tick(&mut p, &base()), Action::PlanDone); }
    #[test]
    fn stale_cache_blocks() { let mut p = Plan { default_branch: "main".into(), items: vec![] }; let mut io = base(); io.stale = true; assert_eq!(tick(&mut p, &io), Action::BlockedOnSpecRefresh); }
}
