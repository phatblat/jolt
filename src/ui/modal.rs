// Modal UI components.
// Reusable modal dialogs for user input and confirmation.

use ratatui::{prelude::*, widgets::*};

/// Draw a branch selection modal on top of the current view.
pub fn draw_branch_modal(
    frame: &mut Frame,
    input: &str,
    branch_history: &[String],
    history_selection: usize,
) {
    let area = frame.area();

    // Create centered modal
    let modal_width = 60;
    let modal_height = 15;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

    // Clear the area behind the modal
    frame.render_widget(Clear, modal_area);

    // Split modal into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title and input
            Constraint::Min(1),    // Branch history list
            Constraint::Length(2), // Instructions
        ])
        .split(modal_area);

    // Input section
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Switch Branch ");

    let input_line = Line::from(vec![
        Span::styled("Branch: ", Style::default().fg(Color::DarkGray)),
        Span::raw(input),
        Span::styled("█", Style::default().fg(Color::Yellow)),
    ]);

    let input_widget = Paragraph::new(input_line)
        .block(input_block)
        .style(Style::default());
    frame.render_widget(input_widget, chunks[0]);

    // Branch history section
    let history_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Recent Branches ");

    if branch_history.is_empty() {
        let empty_text = Paragraph::new("No recent branches")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(history_block);
        frame.render_widget(empty_text, chunks[1]);
    } else {
        let items: Vec<ListItem> = branch_history
            .iter()
            .map(|branch| {
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(branch, Style::default().fg(Color::White)),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(history_selection));

        let list_widget = List::new(items)
            .block(history_block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list_widget, chunks[1], &mut list_state);
    }

    // Instructions
    let instructions = Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::styled(" = Switch  ", Style::default().fg(Color::DarkGray)),
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::styled(" = Navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::styled(" = Cancel ", Style::default().fg(Color::DarkGray)),
    ]);

    let instructions_widget = Paragraph::new(instructions).alignment(Alignment::Center);
    frame.render_widget(instructions_widget, chunks[2]);
}
