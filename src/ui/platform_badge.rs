// Platform badge rendering utilities.
// Provides functions for rendering platform indicators in the UI.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::types::Platform;

/// Render a platform badge as a styled span.
///
/// Returns a span like "[GH]" or "[HR]" with appropriate styling.
pub fn render_badge(platform: Platform) -> Span<'static> {
    let (text, color) = match platform {
        Platform::GitHub => ("[GH]", Color::Blue),
        Platform::Harness => ("[HR]", Color::Rgb(255, 140, 0)), // Orange
    };

    Span::styled(
        text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

/// Render a platform badge as a string (for display name prefixes).
#[allow(dead_code)]
pub fn badge_text(platform: Platform) -> &'static str {
    match platform {
        Platform::GitHub => "[GH]",
        Platform::Harness => "[HR]",
    }
}

/// Get the color for a platform.
#[allow(dead_code)]
pub fn platform_color(platform: Platform) -> Color {
    match platform {
        Platform::GitHub => Color::Blue,
        Platform::Harness => Color::Rgb(255, 140, 0), // Orange
    }
}

/// Create a line with a platform badge prefix.
///
/// # Example
/// ```ignore
/// let line = badge_line(Platform::GitHub, "phatblat/jolt");
/// // Renders as: [GH] phatblat/jolt
/// ```
#[allow(dead_code)]
pub fn badge_line<'a>(platform: Platform, text: &'a str) -> Line<'a> {
    Line::from(vec![
        render_badge(platform),
        Span::raw(" "),
        Span::raw(text),
    ])
}

/// Create a styled line with platform badge and custom styling for the text.
#[allow(dead_code)]
pub fn badge_line_styled<'a>(platform: Platform, text: &'a str, style: Style) -> Line<'a> {
    Line::from(vec![
        render_badge(platform),
        Span::raw(" "),
        Span::styled(text, style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_text() {
        assert_eq!(badge_text(Platform::GitHub), "[GH]");
        assert_eq!(badge_text(Platform::Harness), "[HR]");
    }

    #[test]
    fn test_platform_color() {
        assert_eq!(platform_color(Platform::GitHub), Color::Blue);
        assert_eq!(platform_color(Platform::Harness), Color::Rgb(255, 140, 0));
    }

    #[test]
    fn test_render_badge() {
        let gh_badge = render_badge(Platform::GitHub);
        assert_eq!(gh_badge.content, "[GH]");

        let hr_badge = render_badge(Platform::Harness);
        assert_eq!(hr_badge.content, "[HR]");
    }
}
