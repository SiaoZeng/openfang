//! Capability registry surface for deterministic routing.
//!
//! This is distinct from the security `Capability` enum. These records describe
//! what the system can do so the builtin router can make deterministic choices.

use crate::workflow::WorkflowId;
use openfang_types::agent::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingCapabilityKind {
    Hand,
    Workflow,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingCapabilityTarget {
    Hand { hand_id: String },
    Workflow { workflow_id: WorkflowId },
    Agent { agent_id: AgentId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingCapability {
    pub kind: RoutingCapabilityKind,
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub target: RoutingCapabilityTarget,
}

impl RoutingCapability {
    pub fn matches_name(&self, query: &str) -> bool {
        normalize_route_text(&self.id) == query || normalize_route_text(&self.name) == query
    }
}

pub fn normalize_route_text(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn tokenize_route_text(input: &str) -> Vec<String> {
    normalize_route_text(input)
        .split_whitespace()
        .filter(|token| token.len() >= 3)
        .map(str::to_string)
        .collect()
}
