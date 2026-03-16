//! Deterministic goal router for `builtin:router` agents.
//!
//! This module is intentionally rule-based. It should make explainable routing
//! decisions without depending on an LLM chat loop.

use crate::workflow::WorkflowId;
use openfang_types::agent::AgentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterHand {
    pub hand_id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterWorkflow {
    pub workflow_id: WorkflowId,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterAgent {
    pub agent_id: AgentId,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct RouterCatalog {
    pub hands: Vec<RouterHand>,
    pub workflows: Vec<RouterWorkflow>,
    pub agents: Vec<RouterAgent>,
    pub fallback_agent: Option<RouterAgent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterTarget {
    Hand {
        hand_id: String,
    },
    Workflow {
        workflow_id: WorkflowId,
        workflow_name: String,
    },
    Agent {
        agent_id: AgentId,
        agent_name: String,
    },
    FallbackAgent {
        agent_id: AgentId,
        agent_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterDecision {
    pub target: RouterTarget,
    pub explanation: String,
    pub forward_message: String,
}

pub fn decide(message: &str, catalog: &RouterCatalog) -> Option<RouterDecision> {
    let trimmed = message.trim();
    let normalized = normalize(trimmed);

    if let Some(decision) = explicit_route("hand:", trimmed, &catalog.hands, |hand, remainder| {
        RouterDecision {
            target: RouterTarget::Hand {
                hand_id: hand.hand_id.clone(),
            },
            explanation: format!("explicit hand selector matched '{}'", hand.hand_id),
            forward_message: remainder,
        }
    }) {
        return Some(decision);
    }

    if let Some(decision) = explicit_route(
        "workflow:",
        trimmed,
        &catalog.workflows,
        |workflow, remainder| RouterDecision {
            target: RouterTarget::Workflow {
                workflow_id: workflow.workflow_id,
                workflow_name: workflow.name.clone(),
            },
            explanation: format!("explicit workflow selector matched '{}'", workflow.name),
            forward_message: remainder,
        },
    ) {
        return Some(decision);
    }

    if let Some(decision) =
        explicit_route("agent:", trimmed, &catalog.agents, |agent, remainder| {
            RouterDecision {
                target: RouterTarget::Agent {
                    agent_id: agent.agent_id,
                    agent_name: agent.name.clone(),
                },
                explanation: format!("explicit agent selector matched '{}'", agent.name),
                forward_message: remainder,
            }
        })
    {
        return Some(decision);
    }

    if let Some(workflow) = catalog
        .workflows
        .iter()
        .find(|workflow| mentions_phrase(&normalized, &normalize(&workflow.name)))
    {
        return Some(RouterDecision {
            target: RouterTarget::Workflow {
                workflow_id: workflow.workflow_id,
                workflow_name: workflow.name.clone(),
            },
            explanation: format!("workflow name '{}' was mentioned directly", workflow.name),
            forward_message: trimmed.to_string(),
        });
    }

    if let Some(hand) = catalog.hands.iter().find(|hand| {
        mentions_phrase(&normalized, &normalize(&hand.hand_id))
            || mentions_phrase(&normalized, &normalize(&hand.name))
    }) {
        return Some(RouterDecision {
            target: RouterTarget::Hand {
                hand_id: hand.hand_id.clone(),
            },
            explanation: format!("hand '{}' was mentioned directly", hand.hand_id),
            forward_message: trimmed.to_string(),
        });
    }

    if let Some(agent) = catalog.agents.iter().find(|agent| {
        mentions_phrase(&normalized, &normalize(&agent.name))
            && !normalize(&agent.name).is_empty()
            && normalize(&agent.name) != "assistant"
    }) {
        return Some(RouterDecision {
            target: RouterTarget::Agent {
                agent_id: agent.agent_id,
                agent_name: agent.name.clone(),
            },
            explanation: format!("agent '{}' was mentioned directly", agent.name),
            forward_message: trimmed.to_string(),
        });
    }

    if let Some(decision) = keyword_route(trimmed, &normalized, catalog) {
        return Some(decision);
    }

    catalog.fallback_agent.as_ref().map(|agent| RouterDecision {
        target: RouterTarget::FallbackAgent {
            agent_id: agent.agent_id,
            agent_name: agent.name.clone(),
        },
        explanation: format!(
            "no deterministic hand, workflow, or specialist match was found; using fallback agent '{}'",
            agent.name
        ),
        forward_message: trimmed.to_string(),
    })
}

fn explicit_route<'a, T, F>(
    prefix: &str,
    message: &str,
    items: &'a [T],
    build: F,
) -> Option<RouterDecision>
where
    F: Fn(&'a T, String) -> RouterDecision,
    T: RouteItem,
{
    let trimmed = message.trim();
    let lower = trimmed.to_lowercase();
    if !lower.starts_with(prefix) {
        return None;
    }
    let remainder = &trimmed[prefix.len()..];
    let mut parts = remainder.trim().splitn(2, char::is_whitespace);
    let selector = normalize(parts.next().unwrap_or(""));
    let forwarded_tail = parts.next().unwrap_or("").trim();
    let forward_message = if forwarded_tail.is_empty() {
        trimmed.to_string()
    } else {
        forwarded_tail.to_string()
    };

    items
        .iter()
        .find(|item| item.route_key() == selector || item.route_name_key() == selector)
        .map(|item| build(item, forward_message))
}

fn keyword_route(
    message: &str,
    normalized: &str,
    catalog: &RouterCatalog,
) -> Option<RouterDecision> {
    let hand_keywords: [(&str, &[&str]); 6] = [
        (
            "browser",
            &[
                "browser",
                "website",
                "web page",
                "site",
                "navigate",
                "click",
                "screenshot",
            ],
        ),
        (
            "clip",
            &[
                "video",
                "youtube",
                "clip",
                "subtitle",
                "captions",
                "transcribe",
                "audio",
            ],
        ),
        (
            "lead",
            &["lead", "prospect", "outreach", "sales lead", "prospecting"],
        ),
        (
            "collector",
            &[
                "collect",
                "monitor",
                "tracking",
                "watch sources",
                "watchlist",
            ],
        ),
        (
            "trader",
            &["trade", "trading", "portfolio", "market scan", "signals"],
        ),
        (
            "twitter",
            &["twitter", "tweet", "x post", "social posting", "engagement"],
        ),
    ];

    let agent_keywords: [(&str, &[&str]); 8] = [
        (
            "debugger",
            &["debug", "bug", "root cause", "trace", "investigate failure"],
        ),
        (
            "security-auditor",
            &["security", "vulnerability", "audit", "threat", "hardening"],
        ),
        (
            "test-engineer",
            &["test", "coverage", "regression", "qa", "harness"],
        ),
        (
            "architect",
            &[
                "architecture",
                "design",
                "system design",
                "tradeoff",
                "refactor plan",
            ],
        ),
        ("coder", &["code", "implement", "program", "patch", "fix"]),
        (
            "researcher",
            &[
                "research",
                "sources",
                "investigate",
                "findings",
                "compare options",
            ],
        ),
        ("writer", &["write", "draft", "document", "copy", "article"]),
        (
            "analyst",
            &["analyze", "analysis", "metrics", "dashboard", "data"],
        ),
    ];

    let best_hand =
        best_keyword_match(&catalog.hands, &hand_keywords, normalized).map(|hand| RouterDecision {
            target: RouterTarget::Hand {
                hand_id: hand.hand_id.clone(),
            },
            explanation: format!("matched hand-specific keywords for '{}'", hand.hand_id),
            forward_message: message.to_string(),
        });

    let best_agent =
        best_keyword_match(&catalog.agents, &agent_keywords, normalized).map(|agent| {
            RouterDecision {
                target: RouterTarget::Agent {
                    agent_id: agent.agent_id,
                    agent_name: agent.name.clone(),
                },
                explanation: format!("matched specialist-agent keywords for '{}'", agent.name),
                forward_message: message.to_string(),
            }
        });

    match (best_hand, best_agent) {
        (Some(hand), Some(agent)) => {
            // Prefer specialist agents for generic one-shot work. Hand keywords are
            // intentionally limited to operational/autonomous packages.
            let agent_priority = [
                "debugger",
                "security-auditor",
                "test-engineer",
                "architect",
                "coder",
                "researcher",
                "writer",
                "analyst",
            ];
            let prefer_agent = match &agent.target {
                RouterTarget::Agent { agent_name, .. } => agent_priority
                    .iter()
                    .any(|name| normalize(agent_name) == *name),
                _ => true,
            };
            if prefer_agent {
                Some(agent)
            } else {
                Some(hand)
            }
        }
        (Some(hand), None) => Some(hand),
        (None, Some(agent)) => Some(agent),
        (None, None) => None,
    }
}

fn best_keyword_match<'a, T>(
    items: &'a [T],
    keyword_map: &[(&str, &[&str])],
    normalized: &str,
) -> Option<&'a T>
where
    T: RouteItem,
{
    let mut best: Option<(&T, usize)> = None;
    for item in items {
        let key = item.route_key();
        let Some((_, keywords)) = keyword_map.iter().find(|(route_key, _)| *route_key == key)
        else {
            continue;
        };
        let score = keywords
            .iter()
            .filter(|keyword| mentions_phrase(normalized, &normalize(keyword)))
            .count();
        if score == 0 {
            continue;
        }
        match best {
            Some((best_item, best_score)) => {
                if score > best_score
                    || (score == best_score && item.route_name_key() < best_item.route_name_key())
                {
                    best = Some((item, score));
                }
            }
            None => best = Some((item, score)),
        }
    }
    best.map(|(item, _)| item)
}

fn normalize(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn mentions_phrase(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    haystack == needle
        || haystack.starts_with(&format!("{needle} "))
        || haystack.ends_with(&format!(" {needle}"))
        || haystack.contains(&format!(" {needle} "))
}

trait RouteItem {
    fn route_key(&self) -> String;
    fn route_name_key(&self) -> String;
}

impl RouteItem for RouterHand {
    fn route_key(&self) -> String {
        normalize(&self.hand_id)
    }

    fn route_name_key(&self) -> String {
        normalize(&self.name)
    }
}

impl RouteItem for RouterWorkflow {
    fn route_key(&self) -> String {
        self.workflow_id.to_string()
    }

    fn route_name_key(&self) -> String {
        normalize(&self.name)
    }
}

impl RouteItem for RouterAgent {
    fn route_key(&self) -> String {
        normalize(&self.name)
    }

    fn route_name_key(&self) -> String {
        normalize(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn agent(name: &str) -> RouterAgent {
        RouterAgent {
            agent_id: AgentId(Uuid::new_v4()),
            name: name.to_string(),
            description: String::new(),
        }
    }

    fn hand(id: &str, name: &str) -> RouterHand {
        RouterHand {
            hand_id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
        }
    }

    fn workflow(name: &str) -> RouterWorkflow {
        RouterWorkflow {
            workflow_id: WorkflowId(Uuid::new_v4()),
            name: name.to_string(),
            description: String::new(),
        }
    }

    #[test]
    fn explicit_agent_route_strips_selector_from_forward_message() {
        let catalog = RouterCatalog {
            agents: vec![agent("coder")],
            ..Default::default()
        };

        let decision = decide("agent:coder fix the parser", &catalog).unwrap();
        assert_eq!(decision.forward_message, "fix the parser");
        assert!(matches!(decision.target, RouterTarget::Agent { .. }));
    }

    #[test]
    fn direct_workflow_mention_beats_fallback() {
        let catalog = RouterCatalog {
            workflows: vec![workflow("daily report")],
            fallback_agent: Some(agent("assistant")),
            ..Default::default()
        };

        let decision = decide("please run the daily report workflow", &catalog).unwrap();
        assert!(matches!(decision.target, RouterTarget::Workflow { .. }));
    }

    #[test]
    fn browser_keywords_route_to_browser_hand() {
        let catalog = RouterCatalog {
            hands: vec![hand("browser", "Browser Hand")],
            fallback_agent: Some(agent("assistant")),
            ..Default::default()
        };

        let decision = decide("open a website and take a screenshot", &catalog).unwrap();
        assert_eq!(
            decision.target,
            RouterTarget::Hand {
                hand_id: "browser".to_string()
            }
        );
    }

    #[test]
    fn specialist_keywords_route_to_agent() {
        let catalog = RouterCatalog {
            agents: vec![agent("coder"), agent("assistant")],
            fallback_agent: Some(agent("assistant")),
            ..Default::default()
        };

        let decision = decide("please implement a patch for this code path", &catalog).unwrap();
        assert!(matches!(
            decision.target,
            RouterTarget::Agent { ref agent_name, .. } if agent_name == "coder"
        ));
    }

    #[test]
    fn falls_back_to_assistant_when_no_match_exists() {
        let assistant = agent("assistant");
        let catalog = RouterCatalog {
            fallback_agent: Some(assistant.clone()),
            ..Default::default()
        };

        let decision = decide("hello there", &catalog).unwrap();
        assert_eq!(
            decision.target,
            RouterTarget::FallbackAgent {
                agent_id: assistant.agent_id,
                agent_name: "assistant".to_string()
            }
        );
    }
}
