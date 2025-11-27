// UI module for rendering the TUI.
// Contains widgets for tabs, breadcrumbs, lists, and log viewer.

mod breadcrumb;
mod list;
mod tabs;

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, ConsoleLevel, Tab};
use crate::state::{LoadingState, ViewLevel};

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

    // Breadcrumb (only for Workflows tab)
    if app.active_tab == Tab::Workflows {
        let breadcrumbs = app.workflows.nav.breadcrumbs();
        breadcrumb::draw_breadcrumb(frame, &breadcrumbs, chunks[1]);
    } else {
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(block, chunks[1]);
    }

    // Main content area
    draw_content(frame, app, chunks[2]);

    // Status bar
    draw_status_bar(frame, chunks[3]);
}

/// Draw the main content area based on active tab.
fn draw_content(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        Tab::Runners => draw_runners_tab(frame, area),
        Tab::Workflows => draw_workflows_tab(frame, app, area),
        Tab::Console => draw_console_tab(frame, app, area),
    }
}

/// Draw the Runners tab (placeholder for now).
fn draw_runners_tab(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new("Runners tab\n\nRepos with runners will appear here.")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    let block = Block::default().borders(Borders::ALL).title(" Runners ");

    frame.render_widget(content.block(block), area);
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
    let block = Block::default().borders(Borders::ALL).title(" Logs ");

    match &app.workflows.log_content {
        LoadingState::Idle => {
            let text = Paragraph::new("Press Enter to load logs")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            frame.render_widget(text, area);
        }
        LoadingState::Loading => {
            let text = Paragraph::new("⏳ Loading logs...")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow))
                .block(block);
            frame.render_widget(text, area);
        }
        LoadingState::Error(e) => {
            let text = Paragraph::new(format!("❌ {}", e))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red))
                .block(block);
            frame.render_widget(text, area);
        }
        LoadingState::Loaded(logs) => {
            let text = Paragraph::new(logs.as_str())
                .block(block)
                .scroll((app.workflows.log_scroll_y, app.workflows.log_scroll_x));
            frame.render_widget(text, area);
        }
    }
}

/// Draw the Console tab with error messages.
fn draw_console_tab(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Console ");

    if app.console_messages.is_empty() {
        let text = Paragraph::new("No messages")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(text, area);
    } else {
        let items: Vec<ListItem> = app
            .console_messages
            .iter()
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

        let list_widget = List::new(items).block(block);
        frame.render_widget(list_widget, area);
    }
}

/// Draw the status bar with keybinding hints.
fn draw_status_bar(frame: &mut Frame, area: Rect) {
    let hints = vec![
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
        Span::raw("  q "),
        Span::styled("Quit", Style::default().fg(Color::DarkGray)),
    ];

    let status = Paragraph::new(Line::from(hints));
    frame.render_widget(status, area);
}
