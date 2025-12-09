//! Search provider definitions.
//!
//! This module defines the available search providers (Google, DuckDuckGo, Wikipedia, YouTube)
//! with their triggers, URL templates, and icons.

use crate::assets::PhosphorIcon;

/// A search provider configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchProvider {
    /// The provider name (e.g., "Google", "DuckDuckGo")
    pub name: &'static str,
    /// The trigger string (e.g., "!g", "!d", "!wiki")
    pub trigger: &'static str,
    /// The URL template with {query} placeholder
    pub url_template: &'static str,
    /// The Phosphor icon to use
    pub icon: PhosphorIcon,
}

impl SearchProvider {
    /// Build a search URL with the given query.
    pub fn build_url(&self, query: &str) -> String {
        let encoded_query = urlencoding::encode(query);
        self.url_template.replace("{query}", &encoded_query)
    }
}

/// Get all available search providers.
pub fn get_providers() -> Vec<SearchProvider> {
    vec![
        SearchProvider {
            name: "Google",
            trigger: "!g",
            url_template: "https://www.google.com/search?q={query}",
            icon: PhosphorIcon::MagnifyingGlass,
        },
        SearchProvider {
            name: "DuckDuckGo",
            trigger: "!d",
            url_template: "https://duckduckgo.com/?q={query}",
            icon: PhosphorIcon::Globe,
        },
        SearchProvider {
            name: "Wikipedia",
            trigger: "!wiki",
            url_template: "https://en.wikipedia.org/wiki/Special:Search?search={query}",
            icon: PhosphorIcon::BookOpen,
        },
        SearchProvider {
            name: "YouTube",
            trigger: "!yt",
            url_template: "https://www.youtube.com/results?search_query={query}",
            icon: PhosphorIcon::YoutubeLogo,
        },
    ]
}

/// Find a provider by its trigger.
pub fn find_provider_by_trigger(trigger: &str) -> Option<SearchProvider> {
    get_providers().into_iter().find(|p| p.trigger == trigger)
}
