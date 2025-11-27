// UI module for rendering the TUI.
// Contains widgets for tabs, breadcrumbs, lists, and log viewer.

mod tabs;

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Tab};

/// Main draw function that renders the entire UI.
pub fn draw(frame: &mut Frame, app: &App) {
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

    // Breadcrumb (placeholder for now)
    let breadcrumb = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));
    let breadcrumb_text = Paragraph::new(" > ").block(breadcrumb);
    frame.render_widget(breadcrumb_text, chunks[1]);

    // Main content area
    draw_content(frame, app, chunks[2]);

    // Status bar
    draw_status_bar(frame, chunks[3]);
}

/// Draw the main content area based on active tab.
fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.active_tab {
        Tab::Runners => Paragraph::new("Runners tab\n\nRepos with runners will appear here.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        Tab::Workflows => Paragraph::new("Workflows tab\n\nOwners (users/orgs) will appear here.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        Tab::Console => Paragraph::new("Console\n\nErrors and messages will appear here.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", app.active_tab.title()));

    frame.render_widget(content.block(block), area);
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
        Span::raw("  / "),
        Span::styled("Search", Style::default().fg(Color::DarkGray)),
        Span::raw("  q "),
        Span::styled("Quit", Style::default().fg(Color::DarkGray)),
    ];

    let status = Paragraph::new(Line::from(hints));
    frame.render_widget(status, area);
}
