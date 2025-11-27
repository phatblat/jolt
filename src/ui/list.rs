// Generic list rendering for selectable items.
// Provides styled list views with loading and empty states.

use chrono::{DateTime, Utc};
use ratatui::{prelude::*, widgets::*};

use crate::github::{
    Job, Owner, OwnerType, Repository, RunConclusion, RunStatus, Workflow, WorkflowRun,
};
use crate::state::{LoadingState, SelectableList};

/// Format a timestamp as relative time (e.g., "2h ago").
pub fn format_relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_days() > 0 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

/// Get color for run status.
#[allow(dead_code)]
fn status_color(status: &RunStatus) -> Color {
    match status {
        RunStatus::Completed => Color::Green,
        RunStatus::InProgress => Color::Yellow,
        RunStatus::Queued | RunStatus::Waiting | RunStatus::Pending => Color::Blue,
        RunStatus::Requested => Color::Cyan,
    }
}

/// Get color for run conclusion.
fn conclusion_color(conclusion: &Option<RunConclusion>) -> Color {
    match conclusion {
        Some(RunConclusion::Success) => Color::Green,
        Some(RunConclusion::Failure) => Color::Red,
        Some(RunConclusion::Cancelled) => Color::Gray,
        Some(RunConclusion::Skipped) => Color::Gray,
        Some(RunConclusion::TimedOut) => Color::Red,
        Some(RunConclusion::ActionRequired) => Color::Yellow,
        Some(RunConclusion::Neutral) => Color::White,
        Some(RunConclusion::Stale) => Color::Gray,
        Some(RunConclusion::StartupFailure) => Color::Red,
        None => Color::Yellow, // In progress
    }
}

/// Render a loading indicator.
pub fn render_loading(frame: &mut Frame, area: Rect, message: &str) {
    let text = Paragraph::new(format!("⏳ {}...", message))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(text, area);
}

/// Render an error message.
pub fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    let text = Paragraph::new(format!("❌ {}", error))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    frame.render_widget(text, area);
}

/// Render an empty state message.
pub fn render_empty(frame: &mut Frame, area: Rect, message: &str) {
    let text = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(text, area);
}

/// Render owners list.
pub fn render_owners_list(frame: &mut Frame, list: &mut SelectableList<Owner>, area: Rect) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading owners"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No accessible owners found");
            } else {
                let items: Vec<ListItem> = data
                    .items
                    .iter()
                    .map(|owner| {
                        let type_indicator = match owner.owner_type {
                            OwnerType::User => "👤",
                            OwnerType::Organization => "🏢",
                        };
                        ListItem::new(format!("{} {}", type_indicator, owner.login))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(" Owners "))
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                frame.render_stateful_widget(list_widget, area, &mut list.list_state);
            }
        }
    }
}

/// Render repositories list.
pub fn render_repositories_list(
    frame: &mut Frame,
    list: &mut SelectableList<Repository>,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading repositories"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No repositories found");
            } else {
                let items: Vec<ListItem> = data
                    .items
                    .iter()
                    .map(|repo| {
                        let visibility = if repo.private { "🔒" } else { "🌐" };
                        let updated = format_relative_time(&repo.updated_at);
                        ListItem::new(Line::from(vec![
                            Span::raw(format!("{} ", visibility)),
                            Span::styled(&repo.name, Style::default().fg(Color::Cyan)),
                            Span::styled(
                                format!("  {}", updated),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Repositories "),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                frame.render_stateful_widget(list_widget, area, &mut list.list_state);
            }
        }
    }
}

/// Render workflows list.
pub fn render_workflows_list(frame: &mut Frame, list: &mut SelectableList<Workflow>, area: Rect) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading workflows"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No workflows in this repository");
            } else {
                let items: Vec<ListItem> = data
                    .items
                    .iter()
                    .map(|workflow| {
                        ListItem::new(Line::from(vec![
                            Span::styled(&workflow.name, Style::default().fg(Color::Cyan)),
                            Span::styled(
                                format!("  {}", workflow.path),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(" Workflows "))
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                frame.render_stateful_widget(list_widget, area, &mut list.list_state);
            }
        }
    }
}

/// Render workflow runs list.
pub fn render_runs_list(frame: &mut Frame, list: &mut SelectableList<WorkflowRun>, area: Rect) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading workflow runs"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No workflow runs found");
            } else {
                let items: Vec<ListItem> = data
                    .items
                    .iter()
                    .map(|run| {
                        let status_icon = match run.conclusion {
                            Some(RunConclusion::Success) => "✅",
                            Some(RunConclusion::Failure) => "❌",
                            Some(RunConclusion::Cancelled) => "⚪",
                            Some(RunConclusion::Skipped) => "⏭️",
                            _ => match run.status {
                                RunStatus::InProgress => "🔄",
                                RunStatus::Queued | RunStatus::Waiting => "⏳",
                                _ => "❓",
                            },
                        };

                        let color = conclusion_color(&run.conclusion);
                        let time = format_relative_time(&run.created_at);

                        let mut spans = vec![
                            Span::raw(format!("{} ", status_icon)),
                            Span::styled(
                                format!("#{}", run.run_number),
                                Style::default().fg(color),
                            ),
                            Span::styled(
                                format!("  {}", time),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ];

                        if let Some(branch) = &run.head_branch {
                            spans.push(Span::styled(
                                format!("  {}", branch),
                                Style::default().fg(Color::Magenta),
                            ));
                        }

                        if !run.pull_requests.is_empty() {
                            let pr_nums: Vec<String> = run
                                .pull_requests
                                .iter()
                                .map(|pr| format!("#{}", pr.number))
                                .collect();
                            spans.push(Span::styled(
                                format!("  PR {}", pr_nums.join(", ")),
                                Style::default().fg(Color::Blue),
                            ));
                        }

                        ListItem::new(Line::from(spans))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Workflow Runs "),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                frame.render_stateful_widget(list_widget, area, &mut list.list_state);
            }
        }
    }
}

/// Render jobs list.
pub fn render_jobs_list(frame: &mut Frame, list: &mut SelectableList<Job>, area: Rect) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading jobs"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No jobs in this run");
            } else {
                let items: Vec<ListItem> = data
                    .items
                    .iter()
                    .map(|job| {
                        let status_icon = match job.conclusion {
                            Some(RunConclusion::Success) => "✅",
                            Some(RunConclusion::Failure) => "❌",
                            Some(RunConclusion::Cancelled) => "⚪",
                            Some(RunConclusion::Skipped) => "⏭️",
                            _ => match job.status {
                                RunStatus::InProgress => "🔄",
                                RunStatus::Queued | RunStatus::Waiting => "⏳",
                                _ => "❓",
                            },
                        };

                        let color = conclusion_color(&job.conclusion);

                        let duration = match (job.started_at, job.completed_at) {
                            (Some(start), Some(end)) => {
                                let secs = end.signed_duration_since(start).num_seconds();
                                format!("{}m {}s", secs / 60, secs % 60)
                            }
                            _ => "-".to_string(),
                        };

                        let mut spans = vec![
                            Span::raw(format!("{} ", status_icon)),
                            Span::styled(&job.name, Style::default().fg(color)),
                            Span::styled(
                                format!("  {}", duration),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ];

                        if let Some(runner) = &job.runner_name {
                            spans.push(Span::styled(
                                format!("  @ {}", runner),
                                Style::default().fg(Color::Cyan),
                            ));
                        }

                        ListItem::new(Line::from(spans))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(" Jobs "))
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                frame.render_stateful_widget(list_widget, area, &mut list.list_state);
            }
        }
    }
}
