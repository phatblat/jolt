// Breadcrumb rendering for navigation trail.
// Shows the current navigation path with clickable segments.

use chrono::{DateTime, Utc};
use ratatui::{prelude::*, widgets::*};

use crate::state::navigation::BreadcrumbNode;
use crate::state::runners::RunnersBreadcrumb;

/// Format timestamp for display in ISO 8601 format with local timezone.
fn format_timestamp(dt: &DateTime<Utc>) -> String {
    let local: DateTime<chrono::Local> = dt.with_timezone(&chrono::Local);
    local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

/// Render the breadcrumb trail.
pub fn draw_breadcrumb(
    frame: &mut Frame,
    breadcrumbs: &[BreadcrumbNode],
    area: Rect,
    timestamp: Option<DateTime<Utc>>,
    current_branch: Option<&str>,
) {
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

    // Split area based on whether we have a branch
    if let Some(branch) = current_branch {
        // With branch: need 3 lines - breadcrumb, branch, border
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Breadcrumb + timestamp
                Constraint::Length(1), // Branch line
                Constraint::Length(1), // Border line
            ])
            .split(area);

        // Render breadcrumb text with timestamp on same line
        let breadcrumb_para = Paragraph::new(breadcrumb_line).style(Style::default());
        frame.render_widget(breadcrumb_para, chunks[0]);

        if let Some(ts) = timestamp {
            let timestamp_text = format_timestamp(&ts);
            let timestamp_line = Line::from(vec![Span::styled(
                timestamp_text,
                Style::default().fg(Color::DarkGray),
            )]);
            let timestamp_para = Paragraph::new(timestamp_line).alignment(Alignment::Right);
            frame.render_widget(timestamp_para, chunks[0]);
        }

        // Render branch line
        let branch_line = Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                branch,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  (press 'b' to switch)",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        let branch_para = Paragraph::new(branch_line).style(Style::default());
        frame.render_widget(branch_para, chunks[1]);

        // Render bottom border
        let border = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(border, chunks[2]);
    } else {
        // Without branch: breadcrumb + border
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));

        let paragraph = Paragraph::new(breadcrumb_line)
            .block(block)
            .style(Style::default());

        // Add timestamp on the right if provided
        if let Some(ts) = timestamp {
            let timestamp_text = format_timestamp(&ts);
            frame.render_widget(paragraph, area);

            let timestamp_line = Line::from(vec![Span::styled(
                timestamp_text,
                Style::default().fg(Color::DarkGray),
            )]);
            let timestamp_para = Paragraph::new(timestamp_line).alignment(Alignment::Right);
            frame.render_widget(
                timestamp_para,
                Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: 1,
                },
            );
        } else {
            frame.render_widget(paragraph, area);
        }
    }
}

/// Render the breadcrumb trail for Runners tab.
pub fn draw_runners_breadcrumb(
    frame: &mut Frame,
    breadcrumbs: &[RunnersBreadcrumb],
    area: Rect,
    timestamp: Option<DateTime<Utc>>,
) {
    let mut spans = Vec::new();

    for (i, node) in breadcrumbs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" > ", Style::default().fg(Color::DarkGray)));
        }

        let style = if i == breadcrumbs.len() - 1 {
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

    let mut paragraph = Paragraph::new(breadcrumb_line)
        .block(block)
        .style(Style::default());

    // Add timestamp on the right if provided
    if let Some(ts) = timestamp {
        let timestamp_text = format_timestamp(&ts);
        let timestamp_span = Span::styled(timestamp_text, Style::default().fg(Color::DarkGray));
        paragraph = paragraph.alignment(Alignment::Left);

        // Render breadcrumb first
        frame.render_widget(paragraph, area);

        // Then render timestamp on the right
        let timestamp_line = Line::from(vec![timestamp_span]);
        let timestamp_para = Paragraph::new(timestamp_line)
            .alignment(Alignment::Right)
            .style(Style::default());
        frame.render_widget(
            timestamp_para,
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );
    } else {
        frame.render_widget(paragraph, area);
    }
}
