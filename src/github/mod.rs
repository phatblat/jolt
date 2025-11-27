// GitHub API module.
// Provides client and types for interacting with the GitHub REST API.

#![allow(dead_code, unused_imports)]

pub mod client;
pub mod endpoints;
pub mod types;

pub use client::GitHubClient;
pub use types::*;
