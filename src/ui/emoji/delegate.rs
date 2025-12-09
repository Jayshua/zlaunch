use crate::emoji::{EmojiItem, all_emojis, search_emojis};
use crate::ui::emoji::grid::render_emoji_row;
use crate::ui::theme::theme;
use gpui::{App, Context, SharedString, Task, Window, div, prelude::*};
use gpui_component::IndexPath;
use gpui_component::list::{ListDelegate, ListItem as GpuiListItem, ListState};
use std::sync::Arc;

/// Delegate for displaying emojis in a grid layout.
pub struct EmojiGridDelegate {
    emojis: &'static [EmojiItem],
    filtered_indices: Vec<usize>,
    selected_index: Option<usize>,
    query: String,
    columns: usize,
    on_select: Option<Arc<dyn Fn(&EmojiItem) + Send + Sync>>,
    on_back: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Default for EmojiGridDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl EmojiGridDelegate {
    pub fn new() -> Self {
        let emojis = all_emojis();
        let filtered_indices: Vec<usize> = (0..emojis.len()).collect();
        let columns = theme().emoji_columns;

        Self {
            emojis,
            filtered_indices,
            selected_index: if emojis.is_empty() { None } else { Some(0) },
            query: String::new(),
            columns,
            on_select: None,
            on_back: None,
        }
    }

    /// Set callback for when an emoji is selected.
    pub fn set_on_select(&mut self, callback: impl Fn(&EmojiItem) + Send + Sync + 'static) {
        self.on_select = Some(Arc::new(callback));
    }

    /// Set callback for going back to main view.
    pub fn set_on_back(&mut self, callback: impl Fn() + Send + Sync + 'static) {
        self.on_back = Some(Arc::new(callback));
    }

    /// Get the number of rows needed for the current filtered emojis.
    fn row_count(&self) -> usize {
        self.filtered_indices.len().div_ceil(self.columns)
    }

    /// Get emojis for a specific row.
    fn emojis_for_row(&self, row: usize) -> Vec<EmojiItem> {
        let start = row * self.columns;
        let end = (start + self.columns).min(self.filtered_indices.len());

        self.filtered_indices[start..end]
            .iter()
            .filter_map(|&idx| self.emojis.get(idx).cloned())
            .collect()
    }

    /// Set the search query.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    /// Filter emojis based on query.
    pub fn filter(&mut self) {
        self.filtered_indices = search_emojis(&self.query);
        self.selected_index = if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Get currently selected emoji.
    pub fn selected_emoji(&self) -> Option<&EmojiItem> {
        self.selected_index
            .and_then(|idx| self.filtered_indices.get(idx))
            .and_then(|&emoji_idx| self.emojis.get(emoji_idx))
    }

    /// Get total count of filtered emojis.
    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set selected index.
    pub fn set_selected(&mut self, index: usize) {
        if index < self.filtered_indices.len() {
            self.selected_index = Some(index);
        }
    }

    /// Move selection left (previous item linearly).
    pub fn select_left(&mut self) {
        if let Some(idx) = self.selected_index
            && idx > 0 {
                self.selected_index = Some(idx - 1);
            }
    }

    /// Move selection right (next item linearly).
    pub fn select_right(&mut self) {
        if let Some(idx) = self.selected_index {
            let max = self.filtered_indices.len().saturating_sub(1);
            if idx < max {
                self.selected_index = Some(idx + 1);
            }
        }
    }

    /// Confirm selection (copy emoji).
    pub fn do_confirm(&self) {
        if let Some(emoji) = self.selected_emoji()
            && let Some(ref on_select) = self.on_select {
                on_select(emoji);
            }
    }

    /// Cancel (go back).
    pub fn do_back(&self) {
        if let Some(ref on_back) = self.on_back {
            on_back();
        }
    }

    /// Get the row number for the currently selected emoji.
    pub fn selected_row(&self) -> Option<usize> {
        self.selected_index.map(|idx| idx / self.columns)
    }
}

impl ListDelegate for EmojiGridDelegate {
    type Item = GpuiListItem;

    fn sections_count(&self, _cx: &App) -> usize {
        1
    }

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.row_count()
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Self::Item> {
        let row = ix.row;
        let emojis = self.emojis_for_row(row);
        let start_index = row * self.columns;

        let row_element = render_emoji_row(&emojis, start_index, self.selected_index, row);

        Some(
            GpuiListItem::new(("emoji-row", row))
                .py_0()
                .px_0()
                .child(row_element),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        // Convert row to first item in that row
        self.selected_index = ix.map(|i| i.row * self.columns);
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.filter();
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
                    .child(SharedString::from("No emojis found")),
            )
    }
}
