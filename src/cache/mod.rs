// Cache module for local filesystem caching.
// Stores GitHub API responses and logs for offline access and performance.

#![allow(dead_code, unused_imports)]

pub mod paths;
pub mod store;

pub use paths::*;
pub use store::{
    CachedData, DEFAULT_TTL, read_cached, read_if_valid, read_text, write_cached, write_text,
};
