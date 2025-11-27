// App state and main event loop.
// Manages tabs, navigation state, and keyboard input handling.

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;

use crate::ui;

/// Active tab in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    Runners,
    #[default]
    Workflows,
    Console,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Runners => "Runners",
            Tab::Workflows => "Workflows",
            Tab::Console => "Console",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Tab::Runners => Tab::Workflows,
            Tab::Workflows => Tab::Console,
            Tab::Console => Tab::Runners,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Runners => Tab::Console,
            Tab::Workflows => Tab::Runners,
            Tab::Console => Tab::Workflows,
        }
    }
}

/// Main application state.
pub struct App {
    /// Currently active tab.
    pub active_tab: Tab,
    /// Number of unread console errors (for badge).
    pub console_unread: usize,
    /// Whether the app should exit.
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::default(),
            console_unread: 0,
            should_quit: false,
        }
    }

    /// Main event loop.
    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::draw(frame, self))?;
            self.handle_events()?;
        }
        Ok(())
    }

    /// Handle keyboard and other events.
    #[allow(clippy::collapsible_if)]
    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Tab => {
                            self.active_tab = self.active_tab.next();
                            self.clear_console_badge_if_viewing();
                        }
                        KeyCode::BackTab => {
                            self.active_tab = self.active_tab.prev();
                            self.clear_console_badge_if_viewing();
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Clear console badge when viewing console tab.
    fn clear_console_badge_if_viewing(&mut self) {
        if self.active_tab == Tab::Console {
            self.console_unread = 0;
        }
    }
}
