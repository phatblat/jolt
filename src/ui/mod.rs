// UI module for rendering the TUI.
// Contains widgets for tabs, breadcrumbs, lists, and log viewer.

mod breadcrumb;
mod list;
mod tabs;

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Tab};
use crate::github::{RunConclusion, RunStatus};
use crate::state::{AnalyzeViewLevel, ConsoleLevel, LoadingState, RunnersViewLevel, ViewLevel};

/// Main draw function that renders the entire UI.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Length(3), // Breadcrumb
            Constraint::Min(1),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    // Tab bar
    tabs::draw_tabs(frame, app, chunks[0]);

    // Breadcrumb (for Workflows and Runners tabs)
    match app.active_tab {
        Tab::Workflows => {
            let breadcrumbs = app.workflows.nav.breadcrumbs();
            // Get timestamp from the current view's data
            let timestamp = match app.workflows.nav.current() {
                ViewLevel::Owners => app.workflows.owners.last_updated,
                ViewLevel::Repositories { .. } => app.workflows.repositories.last_updated,
                ViewLevel::Workflows { .. } => app.workflows.workflows.last_updated,
                ViewLevel::Runs { .. } => app.workflows.runs.last_updated,
                ViewLevel::Jobs { .. } => app.workflows.jobs.last_updated,
                ViewLevel::Logs { .. } => None, // Logs don't use SelectableList
            };
            breadcrumb::draw_breadcrumb(frame, &breadcrumbs, chunks[1], timestamp);
        }
        Tab::Runners => {
            let breadcrumbs = app.runners.nav.breadcrumbs();
            // Get timestamp from the current view's data
            let timestamp = match app.runners.nav.current() {
                RunnersViewLevel::Repositories => app.runners.repositories.last_updated,
                RunnersViewLevel::Runners { .. } => app.runners.runners.last_updated,
                RunnersViewLevel::Runs { .. } => app.runners.runs.last_updated,
                RunnersViewLevel::Jobs { .. } => app.runners.jobs.last_updated,
                RunnersViewLevel::Logs { .. } => None, // Logs don't use SelectableList
            };
            breadcrumb::draw_runners_breadcrumb(frame, &breadcrumbs, chunks[1], timestamp);
        }
        Tab::Analyze | Tab::Sync => {
            let block = Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray));
            frame.render_widget(block, chunks[1]);
        }
    }

    // Main content area
    draw_content(frame, app, chunks[2]);

    // Status bar
    draw_status_bar(frame, app, chunks[3]);

    // Help overlay (rendered last, on top of everything)
    if app.show_help {
        draw_help_overlay(frame);
    }
}

/// Draw the main content area based on active tab.
fn draw_content(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        Tab::Runners => draw_runners_tab(frame, app, area),
        Tab::Workflows => draw_workflows_tab(frame, app, area),
        Tab::Analyze => draw_analyze_tab(frame, app, area),
        Tab::Sync => draw_sync_tab(frame, app, area),
    }
}

/// Draw the Runners tab with navigation hierarchy.
fn draw_runners_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.runners.nav.current().clone() {
        RunnersViewLevel::Repositories => {
            list::render_runner_repositories_list(
                frame,
                &mut app.runners.repositories,
                &app.favorite_repos,
                area,
            );
        }
        RunnersViewLevel::Runners {
            ref owner,
            ref repo,
        } => {
            list::render_runners_list(
                frame,
                &mut app.runners.runners,
                &app.favorite_runners,
                owner,
                repo,
                area,
            );
        }
        RunnersViewLevel::Runs { .. } => {
            list::render_runs_list(frame, &mut app.runners.runs, area);
        }
        RunnersViewLevel::Jobs { .. } => {
            list::render_jobs_list(
                frame,
                &mut app.runners.jobs,
                &app.runners.job_groups,
                &app.runners.job_list_items,
                area,
            );
        }
        RunnersViewLevel::Logs { .. } => {
            draw_runners_log_viewer(frame, app, area);
        }
    }
}

/// Draw the log viewer for the Runners tab.
fn draw_runners_log_viewer(frame: &mut Frame, app: &App, area: Rect) {
    // Split area for search input if active
    let (log_area, search_area) = if app.search_active {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    match &app.runners.log_content {
        LoadingState::Idle => {
            let block = Block::default().borders(Borders::ALL).title(" Logs ");
            let text = Paragraph::new("Press Enter to load logs")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Loading => {
            let block = Block::default().borders(Borders::ALL).title(" Logs ");
            let text = Paragraph::new("⏳ Loading logs...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow))
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Error(e) => {
            let block = Block::default().borders(Borders::ALL).title(" Logs ");
            // Check job status/conclusion for special states
            let (is_skipped, is_waiting, is_in_progress, job_id) = match app.runners.nav.current() {
                RunnersViewLevel::Logs {
                    job_status,
                    job_conclusion,
                    job_id,
                    ..
                } => (
                    matches!(job_conclusion, Some(RunConclusion::Skipped)),
                    matches!(
                        job_status,
                        RunStatus::Queued | RunStatus::Waiting | RunStatus::Pending
                    ),
                    matches!(job_status, RunStatus::InProgress),
                    Some(*job_id),
                ),
                _ => (false, false, false, None),
            };
            let lines = if is_skipped {
                vec![
                    Line::from(Span::styled(
                        "⏭️  This job was skipped",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press 'o' to view in browser",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else if is_waiting {
                vec![
                    Line::from(Span::styled(
                        "⏳ This job is queued and waiting to run",
                        Style::default().fg(Color::Blue),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press 'o' to view in browser",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else if is_in_progress {
                // Look up job to get steps
                let job = job_id.and_then(|id| {
                    app.runners
                        .jobs
                        .data
                        .data()
                        .and_then(|data| data.items.iter().find(|j| j.id == id))
                });
                let mut lines = vec![
                    Line::from(Span::styled(
                        "🔄 This job is in progress",
                        Style::default().fg(Color::Yellow),
                    )),
                    Line::from(""),
                ];
                if let Some(job) = job {
                    lines.push(Line::from(Span::styled(
                        "Steps:",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )));
                    for step in &job.steps {
                        let (icon, color) = match (&step.status, &step.conclusion) {
                            (_, Some(RunConclusion::Success)) => ("✅", Color::Green),
                            (_, Some(RunConclusion::Failure)) => ("❌", Color::Red),
                            (_, Some(RunConclusion::Skipped)) => ("⏭️", Color::Gray),
                            (RunStatus::InProgress, _) => ("🔄", Color::Yellow),
                            (RunStatus::Queued | RunStatus::Waiting | RunStatus::Pending, _) => {
                                ("⏳", Color::Blue)
                            }
                            _ => ("⚪", Color::DarkGray),
                        };
                        lines.push(Line::from(vec![
                            Span::raw(format!("  {} ", icon)),
                            Span::styled(&step.name, Style::default().fg(color)),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Press 'o' to view in browser",
                    Style::default().fg(Color::DarkGray),
                )));
                // Render left-aligned for steps list
                let text = Paragraph::new(lines).block(block);
                frame.render_widget(text, log_area);
                // Render search input if active and return early
                if let Some(search_area) = search_area {
                    let search_line = Line::from(vec![
                        Span::styled("/", Style::default().fg(Color::Yellow)),
                        Span::raw(&app.search_query),
                        Span::styled("█", Style::default().fg(Color::Yellow)),
                    ]);
                    let search_widget =
                        Paragraph::new(search_line).style(Style::default().bg(Color::DarkGray));
                    frame.render_widget(search_widget, search_area);
                }
                return;
            } else {
                vec![
                    Line::from(Span::styled(
                        format!("❌ {}", e),
                        Style::default().fg(Color::Red),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press 'o' to view in browser",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            };
            let text = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Loaded(logs) => {
            let line_count = logs.lines().count();
            let scroll_y = app.runners.log_scroll_y as usize;
            let (sel_start, sel_end) = app.runners.log_selection_range();
            let cursor_line = app.runners.log_selection_cursor;

            // Get session lines for decoration
            let session_lines = match app.runners.nav.current() {
                RunnersViewLevel::Logs { job_id, run_id, .. } => {
                    app.analyze.get_session_lines(*job_id, *run_id)
                }
                _ => Vec::new(),
            };

            // Build title with line info and search match count
            let title = if !app.search_matches.is_empty() {
                format!(
                    " Logs [{}-{}/{}] Match {}/{} ",
                    scroll_y + 1,
                    (scroll_y + log_area.height.saturating_sub(2) as usize).min(line_count),
                    line_count,
                    app.search_match_index + 1,
                    app.search_matches.len()
                )
            } else {
                format!(
                    " Logs [{}-{}/{}] ",
                    scroll_y + 1,
                    (scroll_y + log_area.height.saturating_sub(2) as usize).min(line_count),
                    line_count
                )
            };

            // Build selection status for bottom bar
            let selection_count = sel_end - sel_start + 1;
            let clipboard_icon = if app
                .clipboard_flash_until
                .map(|t| t > std::time::Instant::now())
                .unwrap_or(false)
            {
                " 📋"
            } else {
                ""
            };
            let selection_status = if selection_count > 1 {
                format!(
                    " Sel: {}-{} ({} lines){} ",
                    sel_start + 1,
                    sel_end + 1,
                    selection_count,
                    clipboard_icon
                )
            } else {
                format!(" Line {}{} ", cursor_line + 1, clipboard_icon)
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(Line::from(selection_status).centered());

            // Add line numbers and highlight matching/selected lines
            let query_lower = app.search_query.to_lowercase();
            let numbered_lines: Vec<Line> = logs
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    let line_num = i + 1;
                    let is_match =
                        !query_lower.is_empty() && line.to_lowercase().contains(&query_lower);
                    let is_current_match =
                        app.search_matches.get(app.search_match_index) == Some(&i);
                    let is_selected = i >= sel_start && i <= sel_end;
                    let is_cursor = i == cursor_line;
                    // Check if line is part of an existing analysis session
                    let is_in_session = session_lines
                        .iter()
                        .any(|(start, end, _)| i >= *start && i <= *end);

                    // Determine line style: cursor > selection > search match > session > normal
                    let (line_style, line_num_style) = if is_cursor {
                        (
                            Style::default().bg(Color::Blue).fg(Color::Black),
                            Style::default().bg(Color::Blue).fg(Color::Black),
                        )
                    } else if is_selected {
                        (
                            Style::default().bg(Color::DarkGray).fg(Color::White),
                            Style::default().bg(Color::DarkGray).fg(Color::White),
                        )
                    } else if is_current_match {
                        (
                            Style::default().bg(Color::Yellow).fg(Color::Black),
                            Style::default().fg(Color::DarkGray),
                        )
                    } else if is_match {
                        (
                            Style::default().bg(Color::DarkGray),
                            Style::default().fg(Color::DarkGray),
                        )
                    } else if is_in_session {
                        (Style::default(), Style::default().fg(Color::Magenta))
                    } else {
                        (Style::default(), Style::default().fg(Color::DarkGray))
                    };

                    // Use bookmark emoji for session lines, line number for others
                    let line_num_display = if is_in_session {
                        format!("   🔖 │ ")
                    } else {
                        format!("{:>5} │ ", line_num)
                    };

                    Line::from(vec![
                        Span::styled(line_num_display, line_num_style),
                        Span::styled(line, line_style),
                    ])
                })
                .collect();

            let text = Paragraph::new(numbered_lines)
                .block(block)
                .scroll((app.runners.log_scroll_y, app.runners.log_scroll_x));
            frame.render_widget(text, log_area);
        }
    }

    // Render search input if active
    if let Some(search_area) = search_area {
        let search_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(&app.search_query),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ]);
        let search_widget = Paragraph::new(search_line).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(search_widget, search_area);
    }
}

/// Draw the Workflows tab with navigation hierarchy.
fn draw_workflows_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.workflows.nav.current().clone() {
        ViewLevel::Owners => {
            list::render_owners_list(frame, &mut app.workflows.owners, &app.favorite_owners, area);
        }
        ViewLevel::Repositories { ref owner } => {
            list::render_repositories_list(
                frame,
                &mut app.workflows.repositories,
                &app.favorite_repos,
                owner,
                area,
            );
        }
        ViewLevel::Workflows {
            ref owner,
            ref repo,
        } => {
            list::render_workflows_list(
                frame,
                &mut app.workflows.workflows,
                &app.favorite_workflows,
                owner,
                repo,
                area,
            );
        }
        ViewLevel::Runs { .. } => {
            list::render_runs_list(frame, &mut app.workflows.runs, area);
        }
        ViewLevel::Jobs { .. } => {
            list::render_jobs_list(
                frame,
                &mut app.workflows.jobs,
                &app.workflows.job_groups,
                &app.workflows.job_list_items,
                area,
            );
        }
        ViewLevel::Logs { .. } => {
            draw_log_viewer(frame, app, area);
        }
    }
}

/// Draw the log viewer.
fn draw_log_viewer(frame: &mut Frame, app: &App, area: Rect) {
    // Split area for search input if active
    let (log_area, search_area) = if app.search_active {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    match &app.workflows.log_content {
        LoadingState::Idle => {
            let block = Block::default().borders(Borders::ALL).title(" Logs ");
            let text = Paragraph::new("Press Enter to load logs")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Loading => {
            let block = Block::default().borders(Borders::ALL).title(" Logs ");
            let text = Paragraph::new("⏳ Loading logs...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow))
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Error(e) => {
            let block = Block::default().borders(Borders::ALL).title(" Logs ");
            // Check job status/conclusion for special states
            let (is_skipped, is_waiting, is_in_progress, job_id) = match app.workflows.nav.current()
            {
                ViewLevel::Logs {
                    job_status,
                    job_conclusion,
                    job_id,
                    ..
                } => (
                    matches!(job_conclusion, Some(RunConclusion::Skipped)),
                    matches!(
                        job_status,
                        RunStatus::Queued | RunStatus::Waiting | RunStatus::Pending
                    ),
                    matches!(job_status, RunStatus::InProgress),
                    Some(*job_id),
                ),
                _ => (false, false, false, None),
            };
            let lines = if is_skipped {
                vec![
                    Line::from(Span::styled(
                        "⏭️  This job was skipped",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press 'o' to view in browser",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else if is_waiting {
                vec![
                    Line::from(Span::styled(
                        "⏳ This job is queued and waiting to run",
                        Style::default().fg(Color::Blue),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press 'o' to view in browser",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else if is_in_progress {
                // Look up job to get steps
                let job = job_id.and_then(|id| {
                    app.workflows
                        .jobs
                        .data
                        .data()
                        .and_then(|data| data.items.iter().find(|j| j.id == id))
                });
                let mut lines = vec![
                    Line::from(Span::styled(
                        "🔄 This job is in progress",
                        Style::default().fg(Color::Yellow),
                    )),
                    Line::from(""),
                ];
                if let Some(job) = job {
                    lines.push(Line::from(Span::styled(
                        "Steps:",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )));
                    for step in &job.steps {
                        let (icon, color) = match (&step.status, &step.conclusion) {
                            (_, Some(RunConclusion::Success)) => ("✅", Color::Green),
                            (_, Some(RunConclusion::Failure)) => ("❌", Color::Red),
                            (_, Some(RunConclusion::Skipped)) => ("⏭️", Color::Gray),
                            (RunStatus::InProgress, _) => ("🔄", Color::Yellow),
                            (RunStatus::Queued | RunStatus::Waiting | RunStatus::Pending, _) => {
                                ("⏳", Color::Blue)
                            }
                            _ => ("⚪", Color::DarkGray),
                        };
                        lines.push(Line::from(vec![
                            Span::raw(format!("  {} ", icon)),
                            Span::styled(&step.name, Style::default().fg(color)),
                        ]));
                    }
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Press 'o' to view in browser",
                    Style::default().fg(Color::DarkGray),
                )));
                // Render left-aligned for steps list
                let text = Paragraph::new(lines).block(block);
                frame.render_widget(text, log_area);
                // Render search input if active and return early
                if let Some(search_area) = search_area {
                    let search_line = Line::from(vec![
                        Span::styled("/", Style::default().fg(Color::Yellow)),
                        Span::raw(&app.search_query),
                        Span::styled("█", Style::default().fg(Color::Yellow)),
                    ]);
                    let search_widget =
                        Paragraph::new(search_line).style(Style::default().bg(Color::DarkGray));
                    frame.render_widget(search_widget, search_area);
                }
                return;
            } else {
                vec![
                    Line::from(Span::styled(
                        format!("❌ {}", e),
                        Style::default().fg(Color::Red),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press 'o' to view in browser",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            };
            let text = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Loaded(logs) => {
            let line_count = logs.lines().count();
            let scroll_y = app.workflows.log_scroll_y as usize;
            let (sel_start, sel_end) = app.workflows.log_selection_range();
            let cursor_line = app.workflows.log_selection_cursor;

            // Get session lines for decoration
            let session_lines = match app.workflows.nav.current() {
                ViewLevel::Logs { job_id, run_id, .. } => {
                    app.analyze.get_session_lines(*job_id, *run_id)
                }
                _ => Vec::new(),
            };

            // Build title with line info and search match count
            let title = if !app.search_matches.is_empty() {
                format!(
                    " Logs [{}-{}/{}] Match {}/{} ",
                    scroll_y + 1,
                    (scroll_y + log_area.height.saturating_sub(2) as usize).min(line_count),
                    line_count,
                    app.search_match_index + 1,
                    app.search_matches.len()
                )
            } else {
                format!(
                    " Logs [{}-{}/{}] ",
                    scroll_y + 1,
                    (scroll_y + log_area.height.saturating_sub(2) as usize).min(line_count),
                    line_count
                )
            };

            // Build selection status for bottom bar
            let selection_count = sel_end - sel_start + 1;
            let clipboard_icon = if app
                .clipboard_flash_until
                .map(|t| t > std::time::Instant::now())
                .unwrap_or(false)
            {
                " 📋"
            } else {
                ""
            };
            let selection_status = if selection_count > 1 {
                format!(
                    " Sel: {}-{} ({} lines){} ",
                    sel_start + 1,
                    sel_end + 1,
                    selection_count,
                    clipboard_icon
                )
            } else {
                format!(" Line {}{} ", cursor_line + 1, clipboard_icon)
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_bottom(Line::from(selection_status).centered());

            // Add line numbers and highlight matching/selected lines
            let query_lower = app.search_query.to_lowercase();
            let numbered_lines: Vec<Line> = logs
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    let line_num = i + 1;
                    let is_match =
                        !query_lower.is_empty() && line.to_lowercase().contains(&query_lower);
                    let is_current_match =
                        app.search_matches.get(app.search_match_index) == Some(&i);
                    let is_selected = i >= sel_start && i <= sel_end;
                    let is_cursor = i == cursor_line;
                    // Check if line is part of an existing analysis session
                    let is_in_session = session_lines
                        .iter()
                        .any(|(start, end, _)| i >= *start && i <= *end);

                    // Determine line style: cursor > selection > search match > session > normal
                    let (line_style, line_num_style) = if is_cursor {
                        (
                            Style::default().bg(Color::Blue).fg(Color::Black),
                            Style::default().bg(Color::Blue).fg(Color::Black),
                        )
                    } else if is_selected {
                        (
                            Style::default().bg(Color::DarkGray).fg(Color::White),
                            Style::default().bg(Color::DarkGray).fg(Color::White),
                        )
                    } else if is_current_match {
                        (
                            Style::default().bg(Color::Yellow).fg(Color::Black),
                            Style::default().fg(Color::DarkGray),
                        )
                    } else if is_match {
                        (
                            Style::default().bg(Color::DarkGray),
                            Style::default().fg(Color::DarkGray),
                        )
                    } else if is_in_session {
                        (Style::default(), Style::default().fg(Color::Magenta))
                    } else {
                        (Style::default(), Style::default().fg(Color::DarkGray))
                    };

                    // Use bookmark emoji for session lines, line number for others
                    let line_num_display = if is_in_session {
                        format!("   🔖 │ ")
                    } else {
                        format!("{:>5} │ ", line_num)
                    };

                    Line::from(vec![
                        Span::styled(line_num_display, line_num_style),
                        Span::styled(line, line_style),
                    ])
                })
                .collect();

            let text = Paragraph::new(numbered_lines)
                .block(block)
                .scroll((app.workflows.log_scroll_y, app.workflows.log_scroll_x));
            frame.render_widget(text, log_area);
        }
    }

    // Render search input if active
    if let Some(search_area) = search_area {
        let search_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(&app.search_query),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ]);
        let search_widget = Paragraph::new(search_line).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(search_widget, search_area);
    }
}

/// Draw the Analyze tab with saved analysis sessions.
fn draw_analyze_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    match &app.analyze.view {
        AnalyzeViewLevel::List => {
            draw_analyze_list(frame, app, area);
        }
        AnalyzeViewLevel::Detail { session_id } => {
            let session_id = session_id.clone();
            draw_analyze_detail(frame, app, &session_id, area);
        }
    }
}

/// Draw the analysis session list view.
fn draw_analyze_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Analyze ");

    if app.analyze.sessions.is_empty() {
        let text = Paragraph::new("No saved sessions\n\nPress 'a' in log viewer to save selection")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(text, area);
    } else {
        let items: Vec<ListItem> = app
            .analyze
            .sessions
            .iter()
            .map(|session| {
                let time = list::format_relative_time(&session.created_at);
                let line_info = format!(
                    "{} lines",
                    session.excerpt_end_line - session.excerpt_start_line + 1
                );

                ListItem::new(Line::from(vec![
                    Span::styled(&session.title, Style::default().fg(Color::White)),
                    Span::raw("  "),
                    Span::styled(line_info, Style::default().fg(Color::Cyan)),
                    Span::raw("  "),
                    Span::styled(time, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list_widget = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list_widget, area, &mut app.analyze.list_state);
    }
}

/// Draw the analysis session detail view.
fn draw_analyze_detail(frame: &mut Frame, app: &App, session_id: &str, area: Rect) {
    let session = match app.analyze.find_session(session_id) {
        Some(s) => s,
        None => {
            let block = Block::default().borders(Borders::ALL).title(" Analyze ");
            let text = Paragraph::new("Session not found")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red))
                .block(block);
            frame.render_widget(text, area);
            return;
        }
    };

    // Split area: header info + log excerpt
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(area);

    // Header with session metadata
    let header_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", session.title));

    let ctx = &session.nav_context;
    let meta = &session.run_metadata;
    let header_lines = vec![
        Line::from(vec![
            Span::styled("Repo: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/{}", ctx.owner, ctx.repo),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("  "),
            Span::styled("Job: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&ctx.job_name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Run #", Style::default().fg(Color::DarkGray)),
            Span::styled(
                ctx.run_number.to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Lines: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{}-{}/{}",
                    session.excerpt_start_line + 1,
                    session.excerpt_end_line + 1,
                    session.total_log_lines
                ),
                Style::default().fg(Color::Cyan),
            ),
            if let Some(branch) = &meta.branch_name {
                Span::styled(format!("  {}", branch), Style::default().fg(Color::Magenta))
            } else {
                Span::raw("")
            },
        ]),
    ];

    let header = Paragraph::new(header_lines).block(header_block);
    frame.render_widget(header, chunks[0]);

    // Log excerpt
    let log_block = Block::default()
        .borders(Borders::ALL)
        .title(" Log Excerpt ");

    let log_lines: Vec<Line> = session
        .log_excerpt
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let line_num = session.excerpt_start_line + i + 1;
            Line::from(vec![
                Span::styled(
                    format!("{:>6} │ ", line_num),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(line),
            ])
        })
        .collect();

    let log_paragraph = Paragraph::new(log_lines)
        .block(log_block)
        .scroll((app.analyze.detail_scroll_y, 0));

    frame.render_widget(log_paragraph, chunks[1]);
}

/// Draw the Sync tab with sync controls and activity log.
fn draw_sync_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    // Split area: status panel + activity log
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    // Status panel
    draw_sync_status(frame, app, chunks[0]);

    // Activity log
    draw_sync_activity_log(frame, app, chunks[1]);
}

/// Draw sync status panel with toggle, metrics, and progress.
fn draw_sync_status(frame: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = app.sync.status_display();
    let status_color = match status_color {
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "red" => Color::Red,
        _ => Color::DarkGray,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Sync Status ");

    let metrics = &app.sync.metrics;
    let progress = &app.sync.progress;

    let lines = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                status_text,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("(Shift+S to toggle)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Phase: ", Style::default().fg(Color::DarkGray)),
            Span::styled(progress.phase.display(), Style::default().fg(Color::Cyan)),
            if let Some(item) = &progress.current_item {
                Span::styled(format!(" - {}", item), Style::default().fg(Color::White))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled("Jobs synced: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} (session) / {} (total)",
                    metrics.jobs_synced_session, metrics.jobs_synced_total
                ),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Logs cached: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} (session) / {} (total)",
                    metrics.logs_cached_session, metrics.logs_cached_total
                ),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("  "),
            Span::styled("Errors: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                metrics.errors_total.to_string(),
                if metrics.errors_total > 0 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Draw sync activity log with console messages.
fn draw_sync_activity_log(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Activity Log ");

    if app.sync.messages.is_empty() {
        let text = Paragraph::new("No activity yet")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(text, area);
    } else {
        // Show newest messages first (reverse order)
        let items: Vec<ListItem> = app
            .sync
            .messages
            .iter()
            .rev()
            .map(|msg| {
                let (icon, color) = match msg.level {
                    ConsoleLevel::Error => ("❌", Color::Red),
                    ConsoleLevel::Warn => ("⚠️", Color::Yellow),
                    ConsoleLevel::Info => ("ℹ️", Color::Cyan),
                };

                let time = list::format_relative_time(&msg.timestamp);

                ListItem::new(Line::from(vec![
                    Span::raw(format!("{} ", icon)),
                    Span::styled(time, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(msg.message.clone(), Style::default().fg(color)),
                ]))
            })
            .collect();

        let list_widget = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list_widget, area, &mut app.sync.list_state);
    }
}

/// Draw the status bar with keybinding hints and rate limit.
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let in_logs = (app.active_tab == Tab::Workflows
        && matches!(app.workflows.nav.current(), ViewLevel::Logs { .. }))
        || (app.active_tab == Tab::Runners
            && matches!(app.runners.nav.current(), RunnersViewLevel::Logs { .. }));

    let mut hints = if in_logs {
        vec![
            Span::raw(" ↑↓←→ "),
            Span::styled("Scroll", Style::default().fg(Color::DarkGray)),
            Span::raw("  PgUp/Dn "),
            Span::styled("Page", Style::default().fg(Color::DarkGray)),
            Span::raw("  Home/End "),
            Span::styled("Jump", Style::default().fg(Color::DarkGray)),
            Span::raw("  Esc "),
            Span::styled("Back", Style::default().fg(Color::DarkGray)),
            Span::raw("  r "),
            Span::styled("Refresh", Style::default().fg(Color::DarkGray)),
            Span::raw("  ? "),
            Span::styled("Help", Style::default().fg(Color::DarkGray)),
            Span::raw("  q "),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
        ]
    } else {
        vec![
            Span::raw(" ↑↓ "),
            Span::styled("Navigate", Style::default().fg(Color::DarkGray)),
            Span::raw("  ↵ "),
            Span::styled("Select", Style::default().fg(Color::DarkGray)),
            Span::raw("  Esc "),
            Span::styled("Back", Style::default().fg(Color::DarkGray)),
            Span::raw("  Tab "),
            Span::styled("Switch", Style::default().fg(Color::DarkGray)),
            Span::raw("  r "),
            Span::styled("Refresh", Style::default().fg(Color::DarkGray)),
            Span::raw("  ? "),
            Span::styled("Help", Style::default().fg(Color::DarkGray)),
            Span::raw("  q "),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
        ]
    };

    // Add rate limit info on the right if available
    if let Some(client) = &app.github_client {
        let rate = client.rate_limit();
        let rate_color = if rate.remaining < 100 {
            Color::Red
        } else if rate.remaining < 500 {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        hints.push(Span::styled(
            format!("  API: {}/{}", rate.remaining, rate.limit),
            Style::default().fg(rate_color),
        ));
    }

    let status = Paragraph::new(Line::from(hints));
    frame.render_widget(status, area);
}

/// Draw the help overlay.
fn draw_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    // Create a centered popup
    let popup_width = 55;
    let popup_height = 28;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑/↓ or j/k    ", Style::default().fg(Color::Cyan)),
            Span::raw("Navigate list / scroll logs"),
        ]),
        Line::from(vec![
            Span::styled("  ←/→ or h/l    ", Style::default().fg(Color::Cyan)),
            Span::raw("Horizontal scroll (logs)"),
        ]),
        Line::from(vec![
            Span::styled("  Enter         ", Style::default().fg(Color::Cyan)),
            Span::raw("Select / drill down"),
        ]),
        Line::from(vec![
            Span::styled("  Esc           ", Style::default().fg(Color::Cyan)),
            Span::raw("Go back / close help"),
        ]),
        Line::from(vec![
            Span::styled("  Tab/1/2/3/4   ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch tabs"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp/Dn ^u/^d ", Style::default().fg(Color::Cyan)),
            Span::raw("Page scroll (logs)"),
        ]),
        Line::from(vec![
            Span::styled("  Home/End g/G  ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to start/end (logs)"),
        ]),
        Line::from(vec![
            Span::styled("  /             ", Style::default().fg(Color::Cyan)),
            Span::raw("Search in logs"),
        ]),
        Line::from(vec![
            Span::styled("  n/N           ", Style::default().fg(Color::Cyan)),
            Span::raw("Next/prev search match"),
        ]),
        Line::from(vec![
            Span::styled("  r             ", Style::default().fg(Color::Cyan)),
            Span::raw("Refresh current view"),
        ]),
        Line::from(vec![
            Span::styled("  o             ", Style::default().fg(Color::Cyan)),
            Span::raw("Open in GitHub"),
        ]),
        Line::from(vec![
            Span::styled("  f             ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle favorite"),
        ]),
        Line::from(vec![
            Span::styled("  c             ", Style::default().fg(Color::Cyan)),
            Span::raw("Copy selection to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  a             ", Style::default().fg(Color::Cyan)),
            Span::raw("Save selection to Analyze"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+S       ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle background sync"),
        ]),
        Line::from(vec![
            Span::styled("  ?             ", Style::default().fg(Color::Cyan)),
            Span::raw("Show/hide this help"),
        ]),
        Line::from(vec![
            Span::styled("  q             ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" or ", Style::default().fg(Color::DarkGray)),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::styled(" to close", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .alignment(Alignment::Left);

    frame.render_widget(help_paragraph, popup_area);
}
