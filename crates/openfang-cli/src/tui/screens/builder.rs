//! Capability builder screen: analyze goals, submit proposals, track jobs.

use crate::tui::theme;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Clone, Default)]
pub struct BuilderJobInfo {
    pub job_id: String,
    pub approval_id: String,
    pub proposal_name: String,
    pub proposal_kind: String,
    pub proposal_description: String,
    pub status: String,
    pub outcome_summary: String,
    pub error: String,
}

#[derive(Clone, Default)]
pub struct BuilderAnalysis {
    pub gap_detected: bool,
    pub reason: String,
    pub proposal_name: String,
    pub proposal_kind: String,
    pub proposal_description: String,
    pub rationale: String,
    pub suggested_tools: Vec<String>,
    pub workflow_steps: Vec<String>,
    pub artifact: String,
    pub proposal_json: Option<serde_json::Value>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum BuilderSubScreen {
    List,
    AnalyzeInput,
    Proposal,
    JobDetail,
}

pub struct BuilderState {
    pub sub: BuilderSubScreen,
    pub jobs: Vec<BuilderJobInfo>,
    pub list_state: ListState,
    pub current_job: Option<BuilderJobInfo>,
    pub goal_input: String,
    pub analysis: Option<BuilderAnalysis>,
    pub loading: bool,
    pub tick: usize,
    pub status_msg: String,
}

pub enum BuilderAction {
    Continue,
    Refresh,
    AnalyzeGoal(String),
    SubmitProposal {
        proposal: serde_json::Value,
        activate_after_create: bool,
    },
    LoadJob(String),
    Approve(String),
    Reject(String),
}

impl BuilderState {
    pub fn new() -> Self {
        Self {
            sub: BuilderSubScreen::List,
            jobs: Vec::new(),
            list_state: ListState::default(),
            current_job: None,
            goal_input: String::new(),
            analysis: None,
            loading: false,
            tick: 0,
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> BuilderAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return BuilderAction::Continue;
        }
        match self.sub {
            BuilderSubScreen::List => self.handle_list(key),
            BuilderSubScreen::AnalyzeInput => self.handle_input(key),
            BuilderSubScreen::Proposal => self.handle_proposal(key),
            BuilderSubScreen::JobDetail => self.handle_job_detail(key),
        }
    }

    fn handle_list(&mut self, key: KeyEvent) -> BuilderAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let total = self.jobs.len().max(1);
                let i = self.list_state.selected().unwrap_or(0);
                let next = if i == 0 { total - 1 } else { i - 1 };
                self.list_state.select(Some(next));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total = self.jobs.len().max(1);
                let i = self.list_state.selected().unwrap_or(0);
                let next = (i + 1) % total;
                self.list_state.select(Some(next));
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if idx < self.jobs.len() {
                        self.sub = BuilderSubScreen::JobDetail;
                        return BuilderAction::LoadJob(self.jobs[idx].job_id.clone());
                    }
                }
            }
            KeyCode::Char('n') => {
                self.goal_input.clear();
                self.analysis = None;
                self.sub = BuilderSubScreen::AnalyzeInput;
            }
            KeyCode::Char('r') => return BuilderAction::Refresh,
            _ => {}
        }
        BuilderAction::Continue
    }

    fn handle_input(&mut self, key: KeyEvent) -> BuilderAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = BuilderSubScreen::List;
            }
            KeyCode::Enter => {
                let goal = self.goal_input.trim().to_string();
                if !goal.is_empty() {
                    self.loading = true;
                    return BuilderAction::AnalyzeGoal(goal);
                }
            }
            KeyCode::Backspace => {
                self.goal_input.pop();
            }
            KeyCode::Char(c) => self.goal_input.push(c),
            _ => {}
        }
        BuilderAction::Continue
    }

    fn handle_proposal(&mut self, key: KeyEvent) -> BuilderAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = BuilderSubScreen::List;
            }
            KeyCode::Char('s') | KeyCode::Enter => {
                if let Some(ref analysis) = self.analysis {
                    if let Some(ref proposal) = analysis.proposal_json {
                        self.loading = true;
                        return BuilderAction::SubmitProposal {
                            proposal: proposal.clone(),
                            activate_after_create: false,
                        };
                    }
                }
            }
            KeyCode::Char('A') => {
                if let Some(ref analysis) = self.analysis {
                    if analysis.proposal_kind != "workflow" {
                        if let Some(ref proposal) = analysis.proposal_json {
                            self.loading = true;
                            return BuilderAction::SubmitProposal {
                                proposal: proposal.clone(),
                                activate_after_create: true,
                            };
                        }
                    }
                }
            }
            _ => {}
        }
        BuilderAction::Continue
    }

    fn handle_job_detail(&mut self, key: KeyEvent) -> BuilderAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = BuilderSubScreen::List;
            }
            KeyCode::Char('r') => {
                if let Some(ref job) = self.current_job {
                    self.loading = true;
                    return BuilderAction::LoadJob(job.job_id.clone());
                }
            }
            KeyCode::Char('a') => {
                if let Some(ref job) = self.current_job {
                    if job.status == "pending_approval" {
                        self.loading = true;
                        return BuilderAction::Approve(job.approval_id.clone());
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(ref job) = self.current_job {
                    if job.status == "pending_approval" {
                        self.loading = true;
                        return BuilderAction::Reject(job.approval_id.clone());
                    }
                }
            }
            _ => {}
        }
        BuilderAction::Continue
    }
}

pub fn draw(f: &mut Frame<'_>, area: Rect, state: &mut BuilderState) {
    let block = Block::default()
        .title(" Capability Builder ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        BuilderSubScreen::List => draw_list(f, inner, state),
        BuilderSubScreen::AnalyzeInput => draw_input(f, inner, state),
        BuilderSubScreen::Proposal => draw_proposal(f, inner, state),
        BuilderSubScreen::JobDetail => draw_job_detail(f, inner, state),
    }
}

fn draw_list(f: &mut Frame<'_>, area: Rect, state: &mut BuilderState) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)]).split(area);

    let left_items: Vec<ListItem> = if state.loading && state.jobs.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "Loading builder jobs…",
            theme::dim_style(),
        )))]
    } else if state.jobs.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No builder jobs. Press [n] to analyze a goal.",
            theme::dim_style(),
        )))]
    } else {
        state
            .jobs
            .iter()
            .map(|job| {
                ListItem::new(vec![
                    Line::from(Span::styled(
                        format!("{} [{}]", job.proposal_name, job.status),
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        format!("{} · {}", job.proposal_kind, job.job_id),
                        theme::dim_style(),
                    )),
                ])
            })
            .collect()
    };
    let list = List::new(left_items)
        .block(
            Block::default()
                .title(" Jobs ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER)),
        )
        .highlight_style(theme::selected_style())
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, chunks[0], &mut state.list_state);

    let detail = if let Some(ref job) = state.current_job {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("{} ({})", job.proposal_name, job.proposal_kind),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                format!("Status: {}", job.status),
                theme::dim_style(),
            )),
            Line::from(Span::styled(
                format!("Approval: {}", job.approval_id),
                theme::dim_style(),
            )),
            Line::from(Span::styled(
                job.proposal_description.clone(),
                theme::dim_style(),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::raw(job.outcome_summary.clone())),
            Line::from(Span::styled(
                job.error.clone(),
                Style::default().fg(theme::RED),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "[Enter] open  [n] analyze  [r] refresh",
                theme::hint_style(),
            )),
        ];
        append_builder_footer(&mut lines, state);
        lines
    } else {
        let mut lines = vec![
            Line::from(Span::styled(
                "Capability creation is approval-driven.",
                theme::dim_style(),
            )),
            Line::from(Span::styled(
                "Analyze a durable goal, then submit the draft for approval.",
                theme::dim_style(),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "[n] analyze new goal  [r] refresh",
                theme::hint_style(),
            )),
        ];
        append_builder_footer(&mut lines, state);
        lines
    };
    let paragraph = Paragraph::new(detail).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" Summary ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .padding(Padding::horizontal(1)),
    );
    f.render_widget(paragraph, chunks[1]);
}

fn draw_input(f: &mut Frame<'_>, area: Rect, state: &mut BuilderState) {
    let mut lines = vec![
        Line::from(Span::styled(
            "Enter a durable goal for the capability builder.",
            theme::dim_style(),
        )),
        Line::from(Span::raw("")),
        Line::from(Span::raw(state.goal_input.clone())),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            "[Enter] analyze  [Esc] back",
            theme::hint_style(),
        )),
    ];
    append_builder_footer(&mut lines, state);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" Analyze Goal ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .padding(Padding::horizontal(1)),
    );
    f.render_widget(paragraph, area);
}

fn draw_proposal(f: &mut Frame<'_>, area: Rect, state: &mut BuilderState) {
    let analysis = state.analysis.clone().unwrap_or_default();
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} ({})", analysis.proposal_name, analysis.proposal_kind),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(analysis.reason, theme::dim_style())),
        Line::from(Span::raw("")),
        Line::from(Span::raw(analysis.proposal_description)),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            format!("Tools: {}", analysis.suggested_tools.join(", ")),
            theme::dim_style(),
        )),
    ];
    if !analysis.rationale.is_empty() {
        lines.push(Line::from(Span::styled(
            analysis.rationale,
            theme::dim_style(),
        )));
    }
    if !analysis.workflow_steps.is_empty() {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            "Workflow draft steps:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for step in &analysis.workflow_steps {
            lines.push(Line::from(Span::styled(step.clone(), theme::dim_style())));
        }
    }
    if !analysis.artifact.is_empty() {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            "Artifact preview:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for line in analysis.artifact.lines().take(14) {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
    }
    lines.push(Line::from(Span::raw("")));
    let actions_hint = if analysis.proposal_kind == "workflow" {
        "[Enter]/[s] submit  [Esc] back"
    } else if analysis.proposal_kind == "hand" {
        "[Enter]/[s] submit  [Shift+A] create+activate hand  [Esc] back"
    } else {
        "[Enter]/[s] submit  [Shift+A] create+activate  [Esc] back"
    };
    lines.push(Line::from(Span::styled(actions_hint, theme::hint_style())));
    append_builder_footer(&mut lines, state);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" Proposal ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .padding(Padding::horizontal(1)),
    );
    f.render_widget(paragraph, area);
}

fn status_style(status: &str) -> Style {
    match status {
        "applied" => Style::default().fg(theme::GREEN),
        "pending_approval" | "applying" => Style::default().fg(theme::YELLOW),
        "rejected" | "failed" | "timed_out" => Style::default().fg(theme::RED),
        _ => theme::dim_style(),
    }
}

fn draw_job_detail(f: &mut Frame<'_>, area: Rect, state: &mut BuilderState) {
    let job = state.current_job.clone().unwrap_or_default();
    let actions_hint = if job.status == "pending_approval" {
        "[a] approve  [x] reject  [r] refresh  [Esc] back"
    } else if job.status == "applying" {
        "[r] refresh  [Esc] back  (applying…)"
    } else {
        "[r] refresh  [Esc] back"
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} ({})", job.proposal_name, job.proposal_kind),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            format!("Status: {}", job.status),
            status_style(&job.status),
        )),
        Line::from(Span::styled(
            format!("Job ID: {}", job.job_id),
            theme::dim_style(),
        )),
        Line::from(Span::styled(
            format!("Approval ID: {}", job.approval_id),
            theme::dim_style(),
        )),
        Line::from(Span::raw("")),
        Line::from(Span::raw(job.proposal_description)),
        Line::from(Span::raw("")),
        Line::from(Span::raw(job.outcome_summary)),
        Line::from(Span::styled(job.error, Style::default().fg(theme::RED))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(actions_hint, theme::hint_style())),
    ];
    append_builder_footer(&mut lines, state);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .title(" Job Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .padding(Padding::horizontal(1)),
    );
    f.render_widget(paragraph, area);
}

fn append_builder_footer(lines: &mut Vec<Line<'static>>, state: &BuilderState) {
    if state.loading {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            format!("{spinner} Working…"),
            theme::dim_style(),
        )));
    }
    if !state.status_msg.is_empty() {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            state.status_msg.clone(),
            theme::dim_style(),
        )));
    }
}
