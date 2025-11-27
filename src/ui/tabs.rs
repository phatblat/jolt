// Tab bar rendering for the four main tabs.
// Handles visual indication of active tab and sync status.

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Tab};

/// Draw the tab bar at the top of the screen.
pub fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = [Tab::Runners, Tab::Workflows, Tab::Analyze, Tab::Sync];

    let tab_titles: Vec<Line> = tabs
        .iter()
        .map(|tab| {
            let title = tab.title().to_string();

            let style = if *tab == app.active_tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
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
