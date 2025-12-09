//! Clipboard history list delegate.

use crate::clipboard::{ClipboardItem, data};
use crate::ui::clipboard::render_clipboard_item;
use crate::ui::theme::theme;
use gpui::{App, Context, SharedString, Task, Window, div, prelude::*};
use gpui_component::IndexPath;
use gpui_component::list::{ListDelegate, ListItem as GpuiListItem, ListState};
use std::sync::Arc;

/// Delegate for displaying clipboard history.
pub struct ClipboardListDelegate {
    items: Vec<ClipboardItem>,
    selected_index: Option<usize>,
    query: String,
    on_select: Option<Arc<dyn Fn(&ClipboardItem) + Send + Sync>>,
    on_back: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Default for ClipboardListDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardListDelegate {
    pub fn new() -> Self {
        let items = data::search_items("");

        Self {
            selected_index: if items.is_empty() { None } else { Some(0) },
            items,
            query: String::new(),
            on_select: None,
            on_back: None,
        }
    }

    /// Set callback for when a clipboard item is selected.
    pub fn set_on_select(&mut self, callback: impl Fn(&ClipboardItem) + Send + Sync + 'static) {
        self.on_select = Some(Arc::new(callback));
    }

    /// Set callback for going back to main view.
    pub fn set_on_back(&mut self, callback: impl Fn() + Send + Sync + 'static) {
        self.on_back = Some(Arc::new(callback));
    }

    /// Set the search query and filter items.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.filter();
    }

    /// Filter clipboard items based on query.
    pub fn filter(&mut self) {
        self.items = data::search_items(&self.query);
        self.selected_index = if self.items.is_empty() { None } else { Some(0) };
    }

    /// Get currently selected clipboard item.
    pub fn selected_item(&self) -> Option<&ClipboardItem> {
        self.selected_index.and_then(|idx| self.items.get(idx))
    }

    /// Get total count of filtered items.
    pub fn filtered_count(&self) -> usize {
        self.items.len()
    }

    /// Get selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set selected index.
    pub fn set_selected(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = Some(index);
        }
    }

    /// Move selection up (previous item).
    pub fn select_up(&mut self) {
        if let Some(idx) = self.selected_index
            && idx > 0
        {
            self.selected_index = Some(idx - 1);
        }
    }

    /// Move selection down (next item).
    pub fn select_down(&mut self) {
        if let Some(idx) = self.selected_index {
            let max = self.items.len().saturating_sub(1);
            if idx < max {
                self.selected_index = Some(idx + 1);
            }
        }
    }

    /// Confirm selection (copy to clipboard).
    pub fn do_confirm(&self) {
        if let Some(item) = self.selected_item()
            && let Some(ref on_select) = self.on_select
        {
            on_select(item);
        }
    }

    /// Cancel (go back).
    pub fn do_back(&self) {
        if let Some(ref on_back) = self.on_back {
            on_back();
        }
    }
}

impl ListDelegate for ClipboardListDelegate {
    type Item = GpuiListItem;

    fn sections_count(&self, _cx: &App) -> usize {
        1
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.items.len()
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Self::Item> {
        let item = self.items.get(ix.row)?;
        let is_selected = self.selected_index == Some(ix.row);

        let element = render_clipboard_item(item, is_selected, ix.row);

        Some(GpuiListItem::new(("clipboard-item", ix.row)).child(element))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix.map(|i| i.row);
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.set_query(query.to_string());
        Task::ready(())
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.do_confirm();
    }

    fn cancel(&mut self, _window: &mut Window, _cx: &mut Context<ListState<Self>>) {
        self.do_back();
    }

    fn render_empty(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = theme();
        div()
            .w_full()
            .h(t.empty_state_height)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(t.empty_state_color)
                    .child(SharedString::from("No clipboard history")),
            )
    }
}
