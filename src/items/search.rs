//! Search item representing a web search query.

use crate::assets::PhosphorIcon;
use crate::search::SearchProvider;

/// A search item representing a web search query for a specific provider.
#[derive(Clone, Debug)]
pub struct SearchItem {
    /// Unique identifier for this item (e.g., "search-google-rust")
    pub id: String,
    /// Display name (e.g., "Search on Google")
    pub name: String,
    /// The search provider
    pub provider: SearchProvider,
    /// The search query
    pub query: String,
    /// The generated search URL
    pub url: String,
}

impl SearchItem {
    /// Create a new search item for a provider and query.
    pub fn new(provider: SearchProvider, query: String) -> Self {
        let url = provider.build_url(&query);
        let id = format!("search-{}-{}", provider.name.to_lowercase(), query);
        let name = format!("Search on {}", provider.name);
        Self {
            id,
            name,
            provider,
            query,
            url,
        }
    }

    /// Get the icon for this search item.
    pub fn icon(&self) -> PhosphorIcon {
        self.provider.icon
    }

    /// Get the action label.
    pub fn action_label(&self) -> &'static str {
        "Open"
    }
}
