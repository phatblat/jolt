// Generic list rendering for selectable items.
// Provides styled list views with loading and empty states.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use ratatui::{prelude::*, widgets::*};

use crate::github::{
    EnrichedRunner, Job, Owner, OwnerType, Repository, RunConclusion, RunStatus, RunnerStatus,
    Workflow, WorkflowRun,
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
        RunStatus::Unknown => Color::Gray,
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
        Some(RunConclusion::Unknown) => Color::Gray,
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
pub fn render_owners_list(
    frame: &mut Frame,
    list: &mut SelectableList<Owner>,
    favorites: &HashSet<String>,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading owners"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No accessible owners found");
            } else {
                // Sort: favorites first, then alphabetically
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_fav = favorites.contains(&a.login);
                    let b_fav = favorites.contains(&b.login);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.login.cmp(&b.login),
                    }
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|owner| {
                        let is_fav = favorites.contains(&owner.login);
                        let star = if is_fav { "⭐ " } else { "" };
                        let type_indicator = match owner.owner_type {
                            OwnerType::User => "👤",
                            OwnerType::Organization => "🏢",
                            OwnerType::Bot => "🤖",
                            OwnerType::Unknown => "❓",
                        };
                        ListItem::new(format!("{}{} {}", star, type_indicator, owner.login))
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

/// Render repositories list (for Workflows tab with owner context).
pub fn render_repositories_list(
    frame: &mut Frame,
    list: &mut SelectableList<Repository>,
    favorites: &HashSet<String>,
    owner: &str,
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
                // Sort: favorites first, then by name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", owner, a.name);
                    let b_key = format!("{}/{}", owner, b.name);
                    let a_fav = favorites.contains(&a_key);
                    let b_fav = favorites.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|repo| {
                        let key = format!("{}/{}", owner, repo.name);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };
                        let visibility = if repo.private { "🔒" } else { "🌐" };
                        let updated = format_relative_time(&repo.updated_at);
                        ListItem::new(Line::from(vec![
                            Span::raw(format!("{}{} ", star, visibility)),
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

/// Render repositories list for Runners tab (shows owner/repo).
pub fn render_runner_repositories_list(
    frame: &mut Frame,
    list: &mut SelectableList<Repository>,
    favorites: &HashSet<String>,
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
                // Sort: favorites first, then by name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", a.owner.login, a.name);
                    let b_key = format!("{}/{}", b.owner.login, b.name);
                    let a_fav = favorites.contains(&a_key);
                    let b_fav = favorites.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a_key.cmp(&b_key),
                    }
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|repo| {
                        let key = format!("{}/{}", repo.owner.login, repo.name);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };
                        let visibility = if repo.private { "🔒" } else { "🌐" };
                        let updated = format_relative_time(&repo.updated_at);
                        ListItem::new(Line::from(vec![
                            Span::raw(format!("{}{} ", star, visibility)),
                            Span::styled(
                                format!("{}/{}", repo.owner.login, repo.name),
                                Style::default().fg(Color::Cyan),
                            ),
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
pub fn render_workflows_list(
    frame: &mut Frame,
    list: &mut SelectableList<Workflow>,
    favorites: &HashSet<String>,
    owner: &str,
    repo: &str,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading workflows"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No workflows in this repository");
            } else {
                // Sort: favorites first, then by name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", owner, repo, a.id);
                    let b_key = format!("{}/{}/{}", owner, repo, b.id);
                    let a_fav = favorites.contains(&a_key);
                    let b_fav = favorites.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|workflow| {
                        let key = format!("{}/{}/{}", owner, repo, workflow.id);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };
                        // Extract just the filename from path (e.g., "ci.yml" from ".github/workflows/ci.yml")
                        let filename = workflow.path.rsplit('/').next().unwrap_or(&workflow.path);
                        ListItem::new(Line::from(vec![
                            Span::raw(star),
                            Span::styled(&workflow.name, Style::default().fg(Color::Cyan)),
                            Span::styled(
                                format!("  {}", filename),
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
                        let is_in_progress = job.status == RunStatus::InProgress;

                        // Calculate duration - live for in-progress, final for completed
                        let duration = if is_in_progress {
                            if let Some(start) = job.started_at {
                                let secs = chrono::Utc::now()
                                    .signed_duration_since(start)
                                    .num_seconds();
                                format!("{}m {}s", secs / 60, secs % 60)
                            } else {
                                "-".to_string()
                            }
                        } else {
                            match (job.started_at, job.completed_at) {
                                (Some(start), Some(end)) => {
                                    let secs = end.signed_duration_since(start).num_seconds();
                                    format!("{}m {}s", secs / 60, secs % 60)
                                }
                                _ => "-".to_string(),
                            }
                        };

                        let mut first_line = vec![
                            Span::raw(format!("{} ", status_icon)),
                            Span::styled(&job.name, Style::default().fg(color)),
                            Span::styled(
                                format!("  {}", duration),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ];

                        // For in-progress jobs, show additional info on separate lines
                        if is_in_progress {
                            let mut lines = vec![Line::from(first_line)];

                            // Show runner name on its own line
                            if let Some(runner) = &job.runner_name {
                                lines.push(Line::from(vec![
                                    Span::raw("     "),
                                    Span::styled("@ ", Style::default().fg(Color::Cyan)),
                                    Span::styled(runner, Style::default().fg(Color::Cyan)),
                                ]));
                            }

                            // Find current step (in_progress status)
                            let current_step = job
                                .steps
                                .iter()
                                .find(|s| s.status == RunStatus::InProgress)
                                .map(|s| s.name.as_str());

                            if let Some(step_name) = current_step {
                                lines.push(Line::from(vec![
                                    Span::raw("     "),
                                    Span::styled("→ ", Style::default().fg(Color::Yellow)),
                                    Span::styled(step_name, Style::default().fg(Color::Yellow)),
                                ]));
                            }

                            ListItem::new(lines)
                        } else {
                            // For completed jobs, show runner on same line
                            if let Some(runner) = &job.runner_name {
                                first_line.push(Span::styled(
                                    format!("  @ {}", runner),
                                    Style::default().fg(Color::Cyan),
                                ));
                            }
                            ListItem::new(Line::from(first_line))
                        }
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

/// Render runners list.
pub fn render_runners_list(
    frame: &mut Frame,
    list: &mut SelectableList<EnrichedRunner>,
    favorites: &HashSet<String>,
    owner: &str,
    repo: &str,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading runners"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No runners found");
            } else {
                // Sort: favorites first, then by name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", owner, repo, a.runner.name);
                    let b_key = format!("{}/{}/{}", owner, repo, b.runner.name);
                    let a_fav = favorites.contains(&a_key);
                    let b_fav = favorites.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.runner.name.cmp(&b.runner.name),
                    }
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|enriched| {
                        let runner = &enriched.runner;
                        let key = format!("{}/{}/{}", owner, repo, runner.name);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };

                        let (status_icon, status_color) = match runner.status {
                            RunnerStatus::Online => ("🟢", Color::Green),
                            RunnerStatus::Offline => ("⚫", Color::DarkGray),
                            RunnerStatus::Unknown => ("❓", Color::Gray),
                        };

                        let labels: Vec<&str> = runner
                            .labels
                            .iter()
                            .take(3)
                            .map(|l| l.name.as_str())
                            .collect();
                        let labels_str = if labels.is_empty() {
                            String::new()
                        } else {
                            format!("  [{}]", labels.join(", "))
                        };

                        // Build busy indicator with job details if available
                        let busy_info = if runner.busy {
                            if let Some(job_info) = &enriched.current_job {
                                let mut parts = Vec::new();

                                // PR number
                                if let Some(pr) = job_info.pr_number {
                                    parts.push(format!("PR #{}", pr));
                                }

                                // Branch name (truncate if too long)
                                if let Some(branch) = &job_info.branch {
                                    let branch_display = if branch.len() > 30 {
                                        format!("{}...", &branch[..27])
                                    } else {
                                        branch.clone()
                                    };
                                    parts.push(branch_display);
                                }

                                // Time since trigger
                                if let Some(started_at) = job_info.started_at {
                                    let time_str = format_relative_time(&started_at);
                                    parts.push(time_str);
                                }

                                if parts.is_empty() {
                                    "  active".to_string()
                                } else {
                                    format!("  {}", parts.join(" • "))
                                }
                            } else {
                                "  active".to_string()
                            }
                        } else {
                            String::new()
                        };

                        ListItem::new(Line::from(vec![
                            Span::raw(format!("{}{} ", star, status_icon)),
                            Span::styled(&runner.name, Style::default().fg(status_color)),
                            Span::styled(
                                format!("  {}", runner.os),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::styled(labels_str, Style::default().fg(Color::DarkGray)),
                            Span::styled(busy_info, Style::default().fg(Color::Yellow)),
                        ]))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Self-Hosted Runners "),
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
