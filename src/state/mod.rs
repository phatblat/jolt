// State management module.
// Handles navigation, data loading, and UI state for tabs.

#![allow(dead_code)]

pub mod analyze;
pub mod navigation;
pub mod runners;
pub mod sync;
pub mod workflows;

#[allow(unused_imports)]
pub use analyze::{
    AnalysisSession, AnalyzeTabState, AnalyzeViewLevel, NavigationContext, RunMetadata, SourceTab,
};
pub use navigation::{NavigationStack, ViewLevel};
pub use runners::{RunnersNavStack, RunnersTabState, RunnersViewLevel};
#[allow(unused_imports)]
pub use sync::{ConsoleLevel, ConsoleMessage, SyncMetrics, SyncTabState};
pub use workflows::{LoadingState, SelectableList, WorkflowsTabState};
