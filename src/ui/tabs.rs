// Tab bar rendering with badge support for Console tab.
// Handles visual indication of active tab and unread error count.

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Tab};

/// Draw the tab bar at the top of the screen.
pub fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = [Tab::Runners, Tab::Workflows, Tab::Console];

    let tab_titles: Vec<Line> = tabs
        .iter()
        .map(|tab| {
            let title = if *tab == Tab::Console && app.console_unread > 0 {
                format!("{} ({})", tab.title(), app.console_unread)
            } else {
                tab.title().to_string()
            };

            let style = if *tab == app.active_tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if *tab == Tab::Console && app.console_unread > 0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(Span::styled(title, style))
        })
        .collect();

    let selected_index = tabs.iter().position(|t| *t == app.active_tab).unwrap_or(0);

    let tabs_widget = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" jolt ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .select(selected_index)
        .highlight_style(Style::default().fg(Color::Yellow))
        .divider(Span::raw(" │ "));

    frame.render_widget(tabs_widget, area);
}
