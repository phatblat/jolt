// Breadcrumb rendering for navigation trail.
// Shows the current navigation path with clickable segments.

use ratatui::{prelude::*, widgets::*};

use crate::state::navigation::BreadcrumbNode;

/// Render the breadcrumb trail.
pub fn draw_breadcrumb(frame: &mut Frame, breadcrumbs: &[BreadcrumbNode], area: Rect) {
    let mut spans = Vec::new();

    for (i, node) in breadcrumbs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" > ", Style::default().fg(Color::DarkGray)));
        }

        let style = if i == breadcrumbs.len() - 1 {
            // Current level is highlighted
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        spans.push(Span::styled(node.label.clone(), style));
    }

    let breadcrumb_line = Line::from(spans);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(breadcrumb_line)
        .block(block)
        .style(Style::default());

    frame.render_widget(paragraph, area);
}
