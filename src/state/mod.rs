// State management module.
// Handles navigation, data loading, and UI state for tabs.

#![allow(dead_code)]

pub mod navigation;
pub mod runners;
pub mod workflows;

pub use navigation::{NavigationStack, ViewLevel};
pub use runners::{RunnersNavStack, RunnersTabState, RunnersViewLevel};
pub use workflows::{LoadingState, SelectableList, WorkflowsTabState};
