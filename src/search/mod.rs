//! Search module for web search integration.
//!
//! This module provides functionality to:
//! - Detect search triggers (e.g., !g, !wiki, !d, !yt)
//! - Parse search queries
//! - Generate search URLs for various providers

mod detection;
mod providers;

pub use detection::{SearchDetection, detect_search};
pub use providers::{SearchProvider, find_provider_by_trigger, get_providers};
