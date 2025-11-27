// UI module for rendering the TUI.
// Contains widgets for tabs, breadcrumbs, lists, and log viewer.

mod breadcrumb;
mod list;
mod tabs;

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, ConsoleLevel, Tab};
use crate::state::{LoadingState, RunnersViewLevel, ViewLevel};

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
            breadcrumb::draw_breadcrumb(frame, &breadcrumbs, chunks[1]);
        }
        Tab::Runners => {
            let breadcrumbs = app.runners.nav.breadcrumbs();
            breadcrumb::draw_runners_breadcrumb(frame, &breadcrumbs, chunks[1]);
        }
        Tab::Console => {
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
        Tab::Console => draw_console_tab(frame, app, area),
    }
}

/// Draw the Runners tab with navigation hierarchy.
fn draw_runners_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.runners.nav.current().clone() {
        RunnersViewLevel::Repositories => {
            list::render_repositories_list(frame, &mut app.runners.repositories, area);
        }
        RunnersViewLevel::Runners { .. } => {
            list::render_runners_list(frame, &mut app.runners.runners, area);
        }
        RunnersViewLevel::Runs { .. } => {
            list::render_runs_list(frame, &mut app.runners.runs, area);
        }
        RunnersViewLevel::Jobs { .. } => {
            list::render_jobs_list(frame, &mut app.runners.jobs, area);
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
            let text = Paragraph::new(format!("❌ {}", e))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red))
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Loaded(logs) => {
            let line_count = logs.lines().count();
            let scroll_y = app.runners.log_scroll_y as usize;

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

            let block = Block::default().borders(Borders::ALL).title(title);

            // Add line numbers and highlight matching lines
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

                    let line_style = if is_current_match {
                        Style::default().bg(Color::Yellow).fg(Color::Black)
                    } else if is_match {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    Line::from(vec![
                        Span::styled(
                            format!("{:>6} │ ", line_num),
                            Style::default().fg(Color::DarkGray),
                        ),
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
            list::render_owners_list(frame, &mut app.workflows.owners, area);
        }
        ViewLevel::Repositories { .. } => {
            list::render_repositories_list(frame, &mut app.workflows.repositories, area);
        }
        ViewLevel::Workflows { .. } => {
            list::render_workflows_list(frame, &mut app.workflows.workflows, area);
        }
        ViewLevel::Runs { .. } => {
            list::render_runs_list(frame, &mut app.workflows.runs, area);
        }
        ViewLevel::Jobs { .. } => {
            list::render_jobs_list(frame, &mut app.workflows.jobs, area);
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
            let text = Paragraph::new(format!("❌ {}", e))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red))
                .block(block);
            frame.render_widget(text, log_area);
        }
        LoadingState::Loaded(logs) => {
            let line_count = logs.lines().count();
            let scroll_y = app.workflows.log_scroll_y as usize;

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

            let block = Block::default().borders(Borders::ALL).title(title);

            // Add line numbers and highlight matching lines
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

                    let line_style = if is_current_match {
                        Style::default().bg(Color::Yellow).fg(Color::Black)
                    } else if is_match {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    Line::from(vec![
                        Span::styled(
                            format!("{:>6} │ ", line_num),
                            Style::default().fg(Color::DarkGray),
                        ),
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

/// Draw the Console tab with error messages.
fn draw_console_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Console ");

    if app.console_messages.is_empty() {
        let text = Paragraph::new("No messages")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(text, area);
    } else {
        // Show newest messages first (reverse order)
        let items: Vec<ListItem> = app
            .console_messages
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

        frame.render_stateful_widget(list_widget, area, &mut app.console_list_state);
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
    let popup_height = 23;
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
            Span::styled("  Tab           ", Style::default().fg(Color::Cyan)),
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
