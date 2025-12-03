// Tab bar rendering for the four main tabs.
// Handles visual indication of active tab and sync status.

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Tab};
use crate::state::ViewLevel;

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

    // Show branch selector on the right side when on Workflows tab at Workflows view level
    if app.active_tab == Tab::Workflows {
        if let ViewLevel::Workflows { .. } = app.workflows.nav.current() {
            if let Some(branch) = &app.workflows.current_branch {
                let branch_line = Line::from(vec![
                    Span::styled("branch (b): ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        branch,
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
                let branch_para = Paragraph::new(branch_line).alignment(Alignment::Right);
                // Render on the second line of the tab area (below the tabs)
                frame.render_widget(
                    branch_para,
                    Rect {
                        x: area.x,
                        y: area.y + 1,
                        width: area.width,
                        height: 1,
                    },
                );
            }
        }
    }
}
