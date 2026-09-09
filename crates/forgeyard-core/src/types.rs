use std::fmt;
use std::str::FromStr;

/// Who wrote a hook / who holds the busy lock. SPEC.md §6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    TechPm,
    Developer,
    Qa,
    Forge,
    Watch,
    Yard,
    Pi,
    Human,
}

impl Agent {
    pub fn as_str(self) -> &'static str {
        match self {
            Agent::TechPm => "tech-pm",
            Agent::Developer => "developer",
            Agent::Qa => "qa",
            Agent::Forge => "forge",
            Agent::Watch => "watch",
            Agent::Yard => "yard",
            Agent::Pi => "pi",
            Agent::Human => "human",
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Agent {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "tech-pm" => Ok(Agent::TechPm),
            "developer" => Ok(Agent::Developer),
            "qa" => Ok(Agent::Qa),
            "forge" => Ok(Agent::Forge),
            "watch" => Ok(Agent::Watch),
            "yard" => Ok(Agent::Yard),
            "pi" => Ok(Agent::Pi),
            "human" => Ok(Agent::Human),
            other => Err(format!("unknown agent: {other}")),
        }
    }
}

/// Event hook names. SPEC.md §7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hook {
    Bind,
    Start,
    Stop,
    Status,
    TokenSet,
    TokenClear,
    Panel,
    Run,
    Intake,
}

impl Hook {
    pub fn as_str(self) -> &'static str {
        match self {
            Hook::Bind => "bind",
            Hook::Start => "start",
            Hook::Stop => "stop",
            Hook::Status => "status",
            Hook::TokenSet => "token_set",
            Hook::TokenClear => "token_clear",
            Hook::Panel => "panel",
            Hook::Run => "run",
            Hook::Intake => "intake",
        }
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Hook {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "bind" => Ok(Hook::Bind),
            "start" => Ok(Hook::Start),
            "stop" => Ok(Hook::Stop),
            "status" => Ok(Hook::Status),
            "token_set" => Ok(Hook::TokenSet),
            "token_clear" => Ok(Hook::TokenClear),
            "panel" => Ok(Hook::Panel),
            "run" => Ok(Hook::Run),
            "intake" => Ok(Hook::Intake),
            other => Err(format!("unknown hook: {other}")),
        }
    }
}

/// Busy lock value in state.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Busy,
}

impl AgentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentStatus::Idle => "idle",
            AgentStatus::Busy => "busy",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_roundtrip() {
        for a in [
            Agent::TechPm,
            Agent::Developer,
            Agent::Qa,
            Agent::Forge,
            Agent::Watch,
            Agent::Yard,
            Agent::Pi,
            Agent::Human,
        ] {
            assert_eq!(a.as_str().parse::<Agent>().unwrap(), a);
        }
    }

    #[test]
    fn hook_roundtrip() {
        for h in [
            Hook::Bind,
            Hook::Start,
            Hook::Stop,
            Hook::Status,
            Hook::TokenSet,
            Hook::TokenClear,
            Hook::Panel,
            Hook::Run,
            Hook::Intake,
        ] {
            assert_eq!(h.as_str().parse::<Hook>().unwrap(), h);
        }
    }
}
