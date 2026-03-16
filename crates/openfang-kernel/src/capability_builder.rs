//! Deterministic capability-gap detection and draft proposal generation.
//!
//! Phase 9 starts from the routing capability registry introduced in Phase 8.
//! This module decides when the current system does not have a good-enough
//! capability match for a goal and produces a reviewable draft instead of
//! silently treating every unmatched goal as general assistant work.

use crate::routing_capabilities::{
    normalize_route_text, tokenize_route_text, RoutingCapability, RoutingCapabilityKind,
};
use crate::wizard::{AgentIntent, SetupWizard};
use chrono::{DateTime, Utc};
use openfang_hands::HandCategory;
use openfang_hands::{HandAgentConfig, HandDashboard, HandDefinition};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

const MIN_GOAL_TOKENS: usize = 3;
const MIN_MATCHING_KEYWORDS: usize = 2;
const SUITABLE_SCORE_THRESHOLD: f32 = 0.42;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityProposalKind {
    Agent,
    Hand,
    Workflow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityFitCandidate {
    pub kind: RoutingCapabilityKind,
    pub id: String,
    pub name: String,
    pub score: f32,
    pub matched_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDraftStep {
    pub name: String,
    pub purpose: String,
    pub agent_name_hint: String,
    pub prompt_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProposal {
    pub trigger: String,
    pub steps: Vec<WorkflowDraftStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandProposal {
    pub category: String,
    pub tools: Vec<String>,
    pub settings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProposal {
    pub kind: CapabilityProposalKind,
    pub name: String,
    pub description: String,
    pub rationale: String,
    pub approval_required: bool,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub suggested_tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_manifest_toml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowProposal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hand: Option<HandProposal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hand_toml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGapAnalysis {
    pub gap_detected: bool,
    pub reason: String,
    pub goal_tokens: Vec<String>,
    pub threshold_min_keywords: usize,
    pub threshold_min_score: f32,
    pub top_matches: Vec<CapabilityFitCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<CapabilityProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityProposalJobStatus {
    PendingApproval,
    Applying,
    Applied,
    Rejected,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityApplyOutcome {
    Agent {
        agent_id: String,
        name: String,
    },
    Workflow {
        workflow_id: String,
        name: String,
    },
    Hand {
        hand_id: String,
        activated: bool,
        instance_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityProposalJob {
    pub job_id: Uuid,
    pub approval_id: Uuid,
    pub proposal: CapabilityProposal,
    pub activate_after_create: bool,
    pub status: CapabilityProposalJobStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<CapabilityApplyOutcome>,
}

pub fn analyze_gap(message: &str, capabilities: &[RoutingCapability]) -> CapabilityGapAnalysis {
    let normalized = normalize_route_text(message);
    let goal_tokens = unique_tokens(message);
    let top_matches = score_capabilities(&normalized, &goal_tokens, capabilities);

    if !looks_like_capability_gap_candidate(&normalized, &goal_tokens) {
        return CapabilityGapAnalysis {
            gap_detected: false,
            reason: "request looks like conversational or one-off assistant work".to_string(),
            goal_tokens,
            threshold_min_keywords: MIN_MATCHING_KEYWORDS,
            threshold_min_score: SUITABLE_SCORE_THRESHOLD,
            top_matches,
            proposal: None,
        };
    }

    if let Some(best_match) = top_matches.first() {
        if best_match.score >= SUITABLE_SCORE_THRESHOLD
            && best_match.matched_keywords.len() >= MIN_MATCHING_KEYWORDS
        {
            return CapabilityGapAnalysis {
                gap_detected: false,
                reason: format!(
                    "existing capability '{}' cleared the suitability threshold",
                    best_match.name
                ),
                goal_tokens,
                threshold_min_keywords: MIN_MATCHING_KEYWORDS,
                threshold_min_score: SUITABLE_SCORE_THRESHOLD,
                top_matches,
                proposal: None,
            };
        }
    }

    let proposal = build_proposal(message, &normalized, &goal_tokens, top_matches.first());
    CapabilityGapAnalysis {
        gap_detected: true,
        reason: format!(
            "no existing capability matched at least {} shared keywords with score >= {:.2}",
            MIN_MATCHING_KEYWORDS, SUITABLE_SCORE_THRESHOLD
        ),
        goal_tokens,
        threshold_min_keywords: MIN_MATCHING_KEYWORDS,
        threshold_min_score: SUITABLE_SCORE_THRESHOLD,
        top_matches,
        proposal: Some(proposal),
    }
}

pub fn render_gap_summary(analysis: &CapabilityGapAnalysis) -> String {
    let Some(proposal) = &analysis.proposal else {
        return analysis.reason.clone();
    };

    let kind = match proposal.kind {
        CapabilityProposalKind::Agent => "agent",
        CapabilityProposalKind::Hand => "hand",
        CapabilityProposalKind::Workflow => "workflow",
    };

    let mut lines = vec![
        format!("[Router] {}", analysis.reason),
        String::new(),
        "Draft proposal:".to_string(),
        format!("- Kind: {kind}"),
        format!("- Name: {}", proposal.name),
        format!("- Description: {}", proposal.description),
        format!(
            "- Suggested tools: {}",
            if proposal.suggested_tools.is_empty() {
                "none".to_string()
            } else {
                proposal.suggested_tools.join(", ")
            }
        ),
        "- Approval required before creation or activation.".to_string(),
    ];

    if !analysis.top_matches.is_empty() {
        let matches = analysis
            .top_matches
            .iter()
            .take(3)
            .map(|candidate| {
                format!(
                    "{} ({:.2}; matched: {})",
                    candidate.name,
                    candidate.score,
                    if candidate.matched_keywords.is_empty() {
                        "none".to_string()
                    } else {
                        candidate.matched_keywords.join(", ")
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("Top existing matches: {matches}"));
    }

    lines.join("\n")
}

fn score_capabilities(
    normalized_message: &str,
    goal_tokens: &[String],
    capabilities: &[RoutingCapability],
) -> Vec<CapabilityFitCandidate> {
    let goal_set: BTreeSet<&str> = goal_tokens.iter().map(String::as_str).collect();
    let mut scored = capabilities
        .iter()
        .map(|capability| {
            let matched_keywords = capability
                .keywords
                .iter()
                .filter(|keyword| goal_set.contains(keyword.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let score = fit_score(
                capability,
                normalized_message,
                goal_tokens,
                &matched_keywords,
            );
            CapabilityFitCandidate {
                kind: capability.kind.clone(),
                id: capability.id.clone(),
                name: capability.name.clone(),
                score,
                matched_keywords,
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .matched_keywords
                    .len()
                    .cmp(&left.matched_keywords.len())
            })
            .then_with(|| left.name.cmp(&right.name))
    });
    scored.truncate(3);
    scored
}

fn fit_score(
    capability: &RoutingCapability,
    normalized_message: &str,
    goal_tokens: &[String],
    matched_keywords: &[String],
) -> f32 {
    if capability.matches_name(normalized_message) {
        return 1.0;
    }

    if goal_tokens.is_empty() {
        return 0.0;
    }

    let overlap_ratio = matched_keywords.len() as f32 / goal_tokens.len() as f32;
    let mut score = overlap_ratio;

    if matched_keywords.len() >= MIN_MATCHING_KEYWORDS {
        score += 0.15;
    } else if matched_keywords.len() == 1 {
        score += 0.05;
    }

    let name_tokens = tokenize_route_text(&capability.name);
    let id_tokens = tokenize_route_text(&capability.id);
    if matched_keywords
        .iter()
        .any(|keyword| name_tokens.contains(keyword) || id_tokens.contains(keyword))
    {
        score += 0.1;
    }

    score.min(0.95)
}

fn build_proposal(
    message: &str,
    normalized_message: &str,
    goal_tokens: &[String],
    best_match: Option<&CapabilityFitCandidate>,
) -> CapabilityProposal {
    let kind = infer_proposal_kind(normalized_message);
    let slug = proposal_slug(goal_tokens);
    let description = proposal_description(message, kind.clone());
    let tags = proposal_tags(goal_tokens, &kind);
    let suggested_tools = suggested_tools(goal_tokens);
    let rationale = best_match
        .map(|candidate| {
            format!(
                "best existing match was '{}' with score {:.2}; below the builder threshold",
                candidate.name, candidate.score
            )
        })
        .unwrap_or_else(|| "no existing routing capability matched this goal".to_string());

    match kind {
        CapabilityProposalKind::Agent => {
            let plan = SetupWizard::build_plan(agent_intent_from_goal(
                &slug,
                &description,
                message,
                goal_tokens,
            ));
            let manifest_toml = SetupWizard::manifest_to_toml(&plan.manifest).ok();
            CapabilityProposal {
                kind: CapabilityProposalKind::Agent,
                name: slug,
                description,
                rationale,
                approval_required: true,
                tags,
                keywords: goal_tokens.to_vec(),
                suggested_tools,
                agent_manifest_toml: manifest_toml,
                workflow: None,
                hand: None,
                hand_toml: None,
            }
        }
        CapabilityProposalKind::Workflow => CapabilityProposal {
            kind: CapabilityProposalKind::Workflow,
            name: slug,
            description,
            rationale,
            approval_required: true,
            tags,
            keywords: goal_tokens.to_vec(),
            suggested_tools: suggested_tools.clone(),
            agent_manifest_toml: None,
            workflow: Some(WorkflowProposal {
                trigger: "user-invoked goal".to_string(),
                steps: vec![
                    WorkflowDraftStep {
                        name: "intake".to_string(),
                        purpose: "normalize the incoming request and required constraints"
                            .to_string(),
                        agent_name_hint: "assistant".to_string(),
                        prompt_template:
                            "Clarify the request, constraints, and success criteria: {{input}}"
                                .to_string(),
                    },
                    WorkflowDraftStep {
                        name: "gather-context".to_string(),
                        purpose: "collect the files, web data, or records needed to act"
                            .to_string(),
                        agent_name_hint: "assistant".to_string(),
                        prompt_template:
                            "Gather the context needed to execute this goal: {{input}}"
                                .to_string(),
                    },
                    WorkflowDraftStep {
                        name: "execute".to_string(),
                        purpose: "perform the core goal and produce the requested artifact"
                            .to_string(),
                        agent_name_hint: "assistant".to_string(),
                        prompt_template: "{{input}}".to_string(),
                    },
                    WorkflowDraftStep {
                        name: "review-and-deliver".to_string(),
                        purpose: "summarize the result and surface anything needing approval"
                            .to_string(),
                        agent_name_hint: "assistant".to_string(),
                        prompt_template:
                            "Review the result for completeness and summarize what happened: {{input}}"
                                .to_string(),
                    },
                ],
            }),
            hand: None,
            hand_toml: None,
        },
        CapabilityProposalKind::Hand => {
            let category = infer_hand_category(goal_tokens);
            let hand_toml = build_hand_toml(&slug, &description, goal_tokens, &suggested_tools);
            CapabilityProposal {
                kind: CapabilityProposalKind::Hand,
                name: slug,
                description,
                rationale,
                approval_required: true,
                tags,
                keywords: goal_tokens.to_vec(),
                suggested_tools: suggested_tools.clone(),
                agent_manifest_toml: None,
                workflow: None,
                hand: Some(HandProposal {
                    category: category.to_string(),
                    tools: suggested_tools,
                    settings: vec![
                        "target_sources".to_string(),
                        "execution_schedule".to_string(),
                        "delivery_channel".to_string(),
                    ],
                }),
                hand_toml,
            }
        }
    }
}

fn agent_intent_from_goal(
    slug: &str,
    description: &str,
    message: &str,
    goal_tokens: &[String],
) -> AgentIntent {
    let capabilities = capability_flags(goal_tokens);
    AgentIntent {
        name: slug.to_string(),
        description: description.to_string(),
        task: message.trim().to_string(),
        skills: Vec::new(),
        model_tier: infer_model_tier(goal_tokens).to_string(),
        scheduled: false,
        schedule: None,
        capabilities,
    }
}

fn capability_flags(goal_tokens: &[String]) -> Vec<String> {
    let token_set: BTreeSet<&str> = goal_tokens.iter().map(String::as_str).collect();
    let mut flags = Vec::new();

    if token_set
        .iter()
        .any(|token| matches!(*token, "web" | "website" | "browser" | "site" | "scrape"))
    {
        flags.push("browser".to_string());
    }
    if token_set
        .iter()
        .any(|token| matches!(*token, "file" | "files" | "document" | "report"))
    {
        flags.push("files".to_string());
    }
    if token_set
        .iter()
        .any(|token| matches!(*token, "code" | "build" | "test" | "deploy" | "debug"))
    {
        flags.push("shell".to_string());
        flags.push("files".to_string());
    }
    if token_set
        .iter()
        .any(|token| matches!(*token, "monitor" | "track" | "history" | "remember"))
    {
        flags.push("memory".to_string());
    }
    if flags.is_empty() {
        flags.push("file_read".to_string());
    }

    flags.sort();
    flags.dedup();
    flags
}

fn suggested_tools(goal_tokens: &[String]) -> Vec<String> {
    let token_set: BTreeSet<&str> = goal_tokens.iter().map(String::as_str).collect();
    let mut tools = Vec::new();

    if token_set
        .iter()
        .any(|token| matches!(*token, "web" | "website" | "browser" | "site" | "scrape"))
    {
        tools.extend([
            "browser_navigate",
            "browser_read_page",
            "web_search",
            "web_fetch",
        ]);
    }
    if token_set
        .iter()
        .any(|token| matches!(*token, "notify" | "alert" | "message" | "email" | "post"))
    {
        tools.push("comms_send");
    }
    if token_set
        .iter()
        .any(|token| matches!(*token, "file" | "document" | "report" | "draft"))
    {
        tools.extend(["file_read", "file_write"]);
    }
    if token_set
        .iter()
        .any(|token| matches!(*token, "code" | "build" | "test" | "debug" | "deploy"))
    {
        tools.extend(["shell_exec", "file_read", "file_write"]);
    }

    if tools.is_empty() {
        tools.push("file_read");
    }

    let mut tools = tools.into_iter().map(str::to_string).collect::<Vec<_>>();
    tools.sort();
    tools.dedup();
    tools
}

fn infer_proposal_kind(normalized_message: &str) -> CapabilityProposalKind {
    let workflow_signals = [
        "workflow", "pipeline", "process", "then", "after", "review", "approval", "triage",
    ];
    if workflow_signals
        .iter()
        .any(|signal| normalized_message.contains(signal))
    {
        return CapabilityProposalKind::Workflow;
    }

    let hand_signals = [
        "monitor",
        "watch",
        "track",
        "continuous",
        "continuously",
        "background",
        "every day",
        "every week",
        "daily",
        "weekly",
        "autonomous",
        "alert",
        "notify",
    ];
    if hand_signals
        .iter()
        .any(|signal| normalized_message.contains(signal))
    {
        return CapabilityProposalKind::Hand;
    }

    CapabilityProposalKind::Agent
}

fn infer_hand_category(goal_tokens: &[String]) -> HandCategory {
    let token_set: BTreeSet<&str> = goal_tokens.iter().map(String::as_str).collect();
    if token_set
        .iter()
        .any(|token| matches!(*token, "code" | "build" | "test" | "debug" | "deploy"))
    {
        HandCategory::Development
    } else if token_set
        .iter()
        .any(|token| matches!(*token, "email" | "reply" | "outreach" | "message" | "post"))
    {
        HandCategory::Communication
    } else if token_set.iter().any(|token| {
        matches!(
            *token,
            "monitor" | "track" | "scrape" | "report" | "analyze"
        )
    }) {
        HandCategory::Data
    } else {
        HandCategory::Productivity
    }
}

fn infer_model_tier(goal_tokens: &[String]) -> &'static str {
    let token_set: BTreeSet<&str> = goal_tokens.iter().map(String::as_str).collect();
    if token_set.iter().any(|token| {
        matches!(
            *token,
            "code" | "debug" | "analyze" | "investigate" | "workflow" | "pipeline"
        )
    }) {
        "complex"
    } else {
        "medium"
    }
}

fn proposal_description(message: &str, kind: CapabilityProposalKind) -> String {
    let trimmed = message.trim().trim_end_matches('.');
    match kind {
        CapabilityProposalKind::Agent => format!("Specialist agent for {trimmed}"),
        CapabilityProposalKind::Hand => format!("Autonomous hand for {trimmed}"),
        CapabilityProposalKind::Workflow => format!("Workflow for {trimmed}"),
    }
}

fn proposal_tags(goal_tokens: &[String], kind: &CapabilityProposalKind) -> Vec<String> {
    let mut tags = match kind {
        CapabilityProposalKind::Agent => vec!["builder:draft".to_string(), "agent".to_string()],
        CapabilityProposalKind::Hand => vec!["builder:draft".to_string(), "hand".to_string()],
        CapabilityProposalKind::Workflow => {
            vec!["builder:draft".to_string(), "workflow".to_string()]
        }
    };
    tags.extend(
        goal_tokens
            .iter()
            .take(4)
            .map(|token| format!("goal:{token}")),
    );
    tags
}

fn proposal_slug(goal_tokens: &[String]) -> String {
    let stop_words = [
        "please", "need", "want", "help", "with", "that", "this", "from", "into", "agent",
        "workflow", "hand", "make", "create",
    ];
    let filtered = goal_tokens
        .iter()
        .filter(|token| !stop_words.contains(&token.as_str()))
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        "custom-capability-draft".to_string()
    } else {
        filtered.join("-")
    }
}

fn build_hand_toml(
    slug: &str,
    description: &str,
    goal_tokens: &[String],
    suggested_tools: &[String],
) -> Option<String> {
    let definition = HandDefinition {
        id: slug.to_string(),
        name: humanize_slug(slug),
        description: description.to_string(),
        category: infer_hand_category(goal_tokens),
        icon: String::new(),
        tools: suggested_tools.to_vec(),
        skills: Vec::new(),
        mcp_servers: Vec::new(),
        requires: Vec::new(),
        settings: Vec::new(),
        agent: HandAgentConfig {
            name: humanize_slug(slug),
            description: description.to_string(),
            module: "builtin:chat".to_string(),
            provider: "default".to_string(),
            model: "default".to_string(),
            api_key_env: None,
            base_url: None,
            max_tokens: 4096,
            temperature: 0.7,
            system_prompt: format!(
                "You are {}.\n\nYour job is: {}.\nUse the granted tools to execute this autonomous capability safely and concisely.",
                humanize_slug(slug),
                description
            ),
            max_iterations: Some(10),
        },
        dashboard: HandDashboard::default(),
        skill_content: None,
    };

    toml::to_string_pretty(&definition).ok()
}

fn humanize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_capability_gap_candidate(normalized_message: &str, goal_tokens: &[String]) -> bool {
    if goal_tokens.len() < MIN_GOAL_TOKENS {
        return false;
    }

    let conversational_prefixes = [
        "hello",
        "hi",
        "hey",
        "thanks",
        "thank you",
        "good morning",
        "good evening",
        "who are you",
        "what can you do",
    ];
    if conversational_prefixes
        .iter()
        .any(|prefix| normalized_message.starts_with(prefix))
    {
        return false;
    }

    let durable_signals = [
        "automate",
        "monitor",
        "watch",
        "track",
        "pipeline",
        "workflow",
        "process",
        "agent",
        "hand",
        "bot",
        "assistant",
        "daily",
        "weekly",
        "every day",
        "every week",
        "background",
        "notify",
        "alert",
        "triage",
        "collect",
        "sync",
        "scrape",
    ];
    durable_signals
        .iter()
        .any(|signal| normalized_message.contains(signal))
}

fn unique_tokens(input: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut tokens = Vec::new();
    for token in tokenize_route_text(input) {
        if seen.insert(token.clone()) {
            tokens.push(token);
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing_capabilities::{
        RoutingCapability, RoutingCapabilityKind, RoutingCapabilityTarget,
    };
    use crate::workflow::WorkflowId;
    use openfang_types::agent::AgentId;
    use uuid::Uuid;

    fn capability(
        kind: RoutingCapabilityKind,
        id: &str,
        name: &str,
        description: &str,
        keywords: &[&str],
    ) -> RoutingCapability {
        let target = match kind {
            RoutingCapabilityKind::Hand => RoutingCapabilityTarget::Hand {
                hand_id: id.to_string(),
            },
            RoutingCapabilityKind::Workflow => RoutingCapabilityTarget::Workflow {
                workflow_id: WorkflowId(Uuid::new_v4()),
            },
            RoutingCapabilityKind::Agent => RoutingCapabilityTarget::Agent {
                agent_id: AgentId(Uuid::new_v4()),
            },
        };

        RoutingCapability {
            kind,
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            tags: vec![],
            keywords: keywords.iter().map(|keyword| keyword.to_string()).collect(),
            target,
        }
    }

    #[test]
    fn conversational_requests_do_not_trigger_builder() {
        let analysis = analyze_gap("hello there", &[]);
        assert!(!analysis.gap_detected);
        assert!(analysis.proposal.is_none());
    }

    #[test]
    fn strong_existing_match_suppresses_builder() {
        let capabilities = vec![capability(
            RoutingCapabilityKind::Hand,
            "browser",
            "Browser Hand",
            "Navigate websites and take screenshots",
            &["browser", "website", "navigate", "screenshot"],
        )];
        let analysis = analyze_gap("navigate a website and take a screenshot", &capabilities);
        assert!(!analysis.gap_detected);
    }

    #[test]
    fn missing_durable_goal_produces_hand_draft() {
        let capabilities = vec![capability(
            RoutingCapabilityKind::Agent,
            "writer",
            "Writer",
            "Writes drafts",
            &["write", "draft", "document"],
        )];
        let analysis = analyze_gap(
            "monitor competitor pricing daily and notify me about changes",
            &capabilities,
        );
        assert!(analysis.gap_detected);
        let proposal = analysis.proposal.expect("proposal should exist");
        assert_eq!(proposal.kind, CapabilityProposalKind::Hand);
        assert!(proposal
            .suggested_tools
            .iter()
            .any(|tool| tool == "comms_send"));
    }
}
