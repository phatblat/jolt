// Generic list rendering for selectable items.
// Provides styled list views with loading and empty states.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use ratatui::{prelude::*, widgets::*};

use crate::github::{
    EnrichedRunner, Job, JobGroup, JobListItem, RunConclusion, RunStatus, RunnerStatus, Workflow,
    WorkflowRun,
};
use crate::state::{LoadingState, SelectableList};
use crate::types::{self, OrgType, Organization, Platform, Project};
use crate::ui::platform_badge;

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
    } else if duration.num_seconds() > 0 {
        format!("{}s ago", duration.num_seconds())
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
    let text = Paragraph::new(format!("⏳ {message}..."))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(text, area);
}

/// Render an error message.
pub fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    let text = Paragraph::new(format!("❌ {error}"))
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

/// Render organizations list (unified type).
pub fn render_organizations_list(
    frame: &mut Frame,
    list: &mut SelectableList<Organization>,
    favorites: &HashSet<String>,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading organizations"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No accessible organizations found");
            } else {
                // Sort: favorites first, then alphabetically
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_fav = favorites.contains(&a.id);
                    let b_fav = favorites.contains(&b.id);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.id.cmp(&b.id),
                    }
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|org| {
                        let is_fav = favorites.contains(&org.id);
                        let star = if is_fav { "⭐ " } else { "" };
                        let type_indicator = match org.org_type {
                            Some(OrgType::User) => "👤",
                            Some(OrgType::Organization) => "🏢",
                            None => "❓",
                        };
                        ListItem::new(Line::from(vec![
                            platform_badge::render_badge(org.platform),
                            Span::raw(format!(" {}{} {}", star, type_indicator, org.id)),
                        ]))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Organizations "),
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

/// Render projects list for Workflows tab (unified type with org context).
pub fn render_workflow_projects_list(
    frame: &mut Frame,
    list: &mut SelectableList<Project>,
    favorites: &HashSet<String>,
    org_id: &str,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading projects"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No projects found");
            } else {
                // Sort: favorites first, then by name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", org_id, a.name);
                    let b_key = format!("{}/{}", org_id, b.name);
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
                    .map(|project| {
                        let key = format!("{}/{}", org_id, project.name);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };
                        let visibility = if project.visibility.unwrap_or(false) {
                            "🔒"
                        } else {
                            "🌐"
                        };
                        let mut spans = vec![
                            platform_badge::render_badge(project.platform),
                            Span::raw(" "),
                            Span::raw(format!("{star}{visibility} ")),
                            Span::styled(&project.name, Style::default().fg(Color::Cyan)),
                        ];
                        if let Some(updated_at) = &project.updated_at {
                            let updated = format_relative_time(updated_at);
                            spans.push(Span::styled(
                                format!("  {updated}"),
                                Style::default().fg(Color::DarkGray),
                            ));
                        }
                        ListItem::new(Line::from(spans))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(" Projects "))
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
    org_id: &str,
    project_id: &str,
    platform: Platform,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading workflows"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No workflows in this project");
            } else {
                // Sort: favorites first, then by name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", org_id, project_id, a.id);
                    let b_key = format!("{}/{}/{}", org_id, project_id, b.id);
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
                        let key = format!("{}/{}/{}", org_id, project_id, workflow.id);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };
                        // Extract just the filename from path (e.g., "ci.yml" from ".github/workflows/ci.yml")
                        let filename = workflow.path.rsplit('/').next().unwrap_or(&workflow.path);
                        ListItem::new(Line::from(vec![
                            platform_badge::render_badge(platform),
                            Span::raw(" "),
                            Span::raw(star),
                            Span::styled(&workflow.name, Style::default().fg(Color::Cyan)),
                            Span::styled(
                                format!("  {filename}"),
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
pub fn render_runs_list(
    frame: &mut Frame,
    list: &mut SelectableList<WorkflowRun>,
    platform: Platform,
    area: Rect,
    title: &str,
) {
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
                            platform_badge::render_badge(platform),
                            Span::raw(" "),
                            Span::raw(format!("{status_icon} ")),
                            Span::styled(
                                format!("#{}", run.run_number),
                                Style::default().fg(color),
                            ),
                            Span::styled(format!("  {time}"), Style::default().fg(Color::DarkGray)),
                        ];

                        if let Some(branch) = &run.head_branch {
                            spans.push(Span::styled(
                                format!("  {branch}"),
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
                            .title(format!(" {title} ")),
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

/// Render jobs list with hierarchical attempts.
pub fn render_jobs_list(
    frame: &mut Frame,
    list: &mut SelectableList<Job>,
    job_groups: &[JobGroup],
    job_list_items: &[JobListItem],
    platform: Platform,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading jobs"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No jobs in this run");
            } else if job_groups.is_empty() {
                // Fallback: no grouping available yet
                render_empty(frame, area, "Loading job attempts...");
            } else {
                let items: Vec<ListItem> = job_list_items
                    .iter()
                    .map(|list_item| {
                        let job = list_item.get_job(job_groups);
                        let is_sub_item = matches!(list_item, JobListItem::SubItem { .. });
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

                        // Add indentation for sub-items
                        let indent = if is_sub_item { "    " } else { "" };

                        let mut first_line = vec![
                            platform_badge::render_badge(platform),
                            Span::raw(" "),
                            Span::raw(indent),
                            Span::raw(format!("{status_icon} ")),
                            Span::styled(&job.name, Style::default().fg(color)),
                            Span::styled(
                                format!("  {duration}"),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ];

                        // For in-progress jobs, show additional info on separate lines
                        // But not for sub-items (previous attempts)
                        if is_in_progress && !is_sub_item {
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
                                    format!("  @ {runner}"),
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
/// Kept for Workflows tab migration reference.
#[allow(dead_code)]
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
                // Sort: repo scope first, then favorites, then by name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    a.scope
                        .cmp(&b.scope)
                        .then_with(|| {
                            let a_fav = favorites.contains(&a.favorite_key(owner, repo));
                            let b_fav = favorites.contains(&b.favorite_key(owner, repo));
                            b_fav.cmp(&a_fav)
                        })
                        .then_with(|| a.runner.name.cmp(&b.runner.name))
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|enriched| {
                        let runner = &enriched.runner;
                        let key = enriched.favorite_key(owner, repo);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };

                        let (status_icon, status_color) = if runner.busy {
                            // Active runners get yellow icon
                            ("🟡", Color::Yellow)
                        } else {
                            match runner.status {
                                RunnerStatus::Online => ("🟢", Color::Green),
                                RunnerStatus::Offline => ("⚫", Color::DarkGray),
                                RunnerStatus::Unknown => ("❓", Color::Gray),
                            }
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
                                    parts.push(format!("PR #{pr}"));
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

                        // Add platform badge at the start of each runner line
                        ListItem::new(Line::from(vec![
                            platform_badge::render_badge(Platform::GitHub),
                            Span::raw(" "),
                            Span::raw(format!("{star}{status_icon} ")),
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

// ========================================
// Unified Render Functions (for Runners tab)
// ========================================

/// Render unified projects list (used by Runners tab).
pub fn render_projects_list(
    frame: &mut Frame,
    list: &mut SelectableList<types::Project>,
    favorites: &HashSet<String>,
    area: Rect,
) {
    match &list.data {
        LoadingState::Idle => render_empty(frame, area, "Press Enter to load"),
        LoadingState::Loading => render_loading(frame, area, "Loading projects"),
        LoadingState::Error(e) => render_error(frame, area, e),
        LoadingState::Loaded(data) => {
            if data.is_empty() {
                render_empty(frame, area, "No projects found");
            } else {
                // Sort: favorites first, then by display_name
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_fav = favorites.contains(&a.display_name);
                    let b_fav = favorites.contains(&b.display_name);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.display_name.cmp(&b.display_name),
                    }
                });

                let items: Vec<ListItem> = sorted
                    .iter()
                    .map(|project| {
                        let is_fav = favorites.contains(&project.display_name);
                        let star = if is_fav { "⭐ " } else { "" };
                        ListItem::new(Line::from(vec![
                            platform_badge::render_badge(project.platform),
                            Span::raw(" "),
                            Span::raw(star),
                            Span::styled(&project.display_name, Style::default().fg(Color::Cyan)),
                        ]))
                    })
                    .collect();

                let list_widget = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(" Projects "))
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

/// Render unified runners list (used by Runners tab).
pub fn render_unified_runners_list(
    frame: &mut Frame,
    list: &mut SelectableList<types::Runner>,
    favorites: &HashSet<String>,
    org_id: &str,
    project_id: &str,
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
                    let a_key = format!("{}/{}/{}", org_id, project_id, a.name);
                    let b_key = format!("{}/{}/{}", org_id, project_id, b.name);
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
                    .map(|runner| {
                        let key = format!("{}/{}/{}", org_id, project_id, runner.name);
                        let is_fav = favorites.contains(&key);
                        let star = if is_fav { "⭐ " } else { "" };

                        let (status_icon, status_color) = match runner.status {
                            types::RunnerStatus::Online => ("🟢", Color::Green),
                            types::RunnerStatus::Offline => ("⚫", Color::DarkGray),
                            types::RunnerStatus::Busy => ("🟡", Color::Yellow),
                            types::RunnerStatus::Unhealthy => ("🔴", Color::Red),
                        };

                        let labels_str = match &runner.labels {
                            Some(labels) if !labels.is_empty() => {
                                let display: Vec<&str> =
                                    labels.iter().take(3).map(|l| l.as_str()).collect();
                                format!("  [{}]", display.join(", "))
                            }
                            _ => String::new(),
                        };

                        let os_str = match &runner.os {
                            Some(os) => format!("  {}", os),
                            None => String::new(),
                        };

                        let busy_info = match &runner.current_job {
                            Some(info) => format!("  {}", info),
                            None => String::new(),
                        };

                        ListItem::new(Line::from(vec![
                            platform_badge::render_badge(runner.platform),
                            Span::raw(" "),
                            Span::raw(format!("{}{} ", star, status_icon)),
                            Span::styled(&runner.name, Style::default().fg(status_color)),
                            Span::styled(os_str, Style::default().fg(Color::Cyan)),
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
