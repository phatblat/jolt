// State management module.
// Handles navigation, data loading, and UI state for tabs.

#![allow(dead_code)]

pub mod navigation;
pub mod runners;
pub mod workflows;

pub use navigation::ViewLevel;
pub use runners::{RunnersTabState, RunnersViewLevel};
pub use workflows::{LoadingState, SelectableList, WorkflowsTabState};
