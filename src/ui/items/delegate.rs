use crate::calculator::{evaluate_expression, looks_like_expression};
use crate::items::{ActionItem, CalculatorItem, ListItem, SearchItem, SubmenuItem};
use crate::search::{SearchDetection, detect_search, get_providers};
use crate::ui::items::render_item;
use crate::ui::theme::theme;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use gpui::{App, Context, SharedString, Task, Window, div, prelude::*};
use gpui_component::IndexPath;
use gpui_component::list::{ListDelegate, ListItem as GpuiListItem, ListState};
use std::sync::Arc;

/// Section information for the list.
#[derive(Clone, Debug, Default)]
pub struct SectionInfo {
    /// Number of search items in filtered results
    pub search_count: usize,
    /// Number of windows in filtered results
    pub window_count: usize,
    /// Number of commands (submenus and actions) in filtered results
    pub command_count: usize,
    /// Number of applications in filtered results
    pub app_count: usize,
}

/// Types of sections in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SectionType {
    Calculator,
    Search,
    Windows,
    Commands,
    Applications,
}

/// A generic delegate for displaying and filtering list items.
pub struct ItemListDelegate {
    items: Arc<Vec<ListItem>>,
    filtered_indices: Vec<usize>,
    section_info: SectionInfo,
    selected_index: Option<usize>,
    query: String,
    /// Calculator result shown at the top when the query is a math expression.
    calculator_item: Option<CalculatorItem>,
    /// Search items shown when query triggers search or when no matches found.
    search_items: Vec<SearchItem>,
    on_confirm: Option<Arc<dyn Fn(&ListItem) + Send + Sync>>,
    on_cancel: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl ItemListDelegate {
    pub fn new(mut items: Vec<ListItem>) -> Self {
        // Add built-in submenu items
        items.push(ListItem::Submenu(
            SubmenuItem::grid("submenu-emojis", "Emojis", 8)
                .with_description("Search and copy emojis")
                .with_icon("smiley"),
        ));
        items.push(ListItem::Submenu(
            SubmenuItem::list("submenu-clipboard", "Clipboard History")
                .with_description("View and paste clipboard history")
                .with_icon("clipboard"),
        ));

        // Add built-in action items
        for action in ActionItem::builtins() {
            items.push(ListItem::Action(action));
        }

        let len = items.len();
        let filtered_indices: Vec<usize> = (0..len).collect();
        let section_info = Self::compute_section_info(&items, &filtered_indices);

        Self {
            items: Arc::new(items),
            filtered_indices,
            section_info,
            selected_index: if len > 0 { Some(0) } else { None },
            query: String::new(),
            calculator_item: None,
            search_items: Vec::new(),
            on_confirm: None,
            on_cancel: None,
        }
    }

    /// Compute section counts from filtered indices.
    fn compute_section_info(items: &[ListItem], filtered_indices: &[usize]) -> SectionInfo {
        let mut info = SectionInfo::default();

        for &item_idx in filtered_indices {
            if let Some(item) = items.get(item_idx) {
                if item.is_window() {
                    info.window_count += 1;
                } else if item.is_submenu() || item.is_action() {
                    // Actions are grouped with commands (submenus)
                    info.command_count += 1;
                } else if item.is_application() {
                    info.app_count += 1;
                }
            }
        }

        info
    }

    /// Set the callback for when an item is confirmed (Enter pressed).
    pub fn set_on_confirm(&mut self, callback: impl Fn(&ListItem) + Send + Sync + 'static) {
        self.on_confirm = Some(Arc::new(callback));
    }

    /// Set the callback for when the list is cancelled (Escape pressed).
    pub fn set_on_cancel(&mut self, callback: impl Fn() + Send + Sync + 'static) {
        self.on_cancel = Some(Arc::new(callback));
    }

    /// Returns the items Arc for use in background filtering.
    pub fn items(&self) -> Arc<Vec<ListItem>> {
        Arc::clone(&self.items)
    }

    /// Filter items on a background thread - returns filtered indices.
    /// Results are sorted by type (windows first) then by score.
    pub fn filter_items_sync(items: &[ListItem], query: &str) -> Vec<usize> {
        if query.is_empty() {
            // Sort by type priority (windows first, then applications)
            let mut indices: Vec<usize> = (0..items.len()).collect();
            indices.sort_by_key(|&idx| items[idx].sort_priority());
            indices
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(usize, i64)> = items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    matcher
                        .fuzzy_match(item.name(), query)
                        .map(|score| (idx, score))
                })
                .collect();

            // Sort by type priority first, then by score within each type
            scored.sort_by(|a, b| {
                let priority_a = items[a.0].sort_priority();
                let priority_b = items[b.0].sort_priority();
                priority_a.cmp(&priority_b).then_with(|| b.1.cmp(&a.1))
            });
            scored.into_iter().map(|(idx, _)| idx).collect()
        }
    }

    /// Apply pre-computed filter results.
    pub fn apply_filter_results(&mut self, query: String, indices: Vec<usize>) {
        // Only apply if query still matches (user might have typed more)
        if self.query == query {
            // Evaluate calculator expression
            self.calculator_item = self.try_evaluate_calculator(&query);

            // Generate search items
            let has_matches = !indices.is_empty();
            self.search_items = self.try_generate_search_items(&query, has_matches);

            self.section_info = Self::compute_section_info(&self.items, &indices);
            self.section_info.search_count = self.search_items.len();
            self.filtered_indices = indices;

            let has_items = self.calculator_item.is_some()
                || !self.search_items.is_empty()
                || !self.filtered_indices.is_empty();
            self.selected_index = if has_items { Some(0) } else { None };
        }
    }

    fn filter_items(&mut self) {
        // Try to evaluate as calculator expression
        self.calculator_item = self.try_evaluate_calculator(&self.query.clone());

        self.filtered_indices = Self::filter_items_sync(&self.items, &self.query);

        // Generate search items
        let has_matches = !self.filtered_indices.is_empty();
        self.search_items = self.try_generate_search_items(&self.query, has_matches);

        self.section_info = Self::compute_section_info(&self.items, &self.filtered_indices);
        self.section_info.search_count = self.search_items.len();

        // Set selection: calculator at 0 if present, otherwise first filtered item
        let has_items = self.calculator_item.is_some()
            || !self.search_items.is_empty()
            || !self.filtered_indices.is_empty();
        self.selected_index = if has_items { Some(0) } else { None };
    }

    /// Try to evaluate the query as a calculator expression.
    fn try_evaluate_calculator(&self, query: &str) -> Option<CalculatorItem> {
        if !looks_like_expression(query) {
            return None;
        }

        evaluate_expression(query).map(CalculatorItem::from_calc_result)
    }

    /// Generate search items based on the query and current filter state.
    fn try_generate_search_items(&self, query: &str, has_matches: bool) -> Vec<SearchItem> {
        // Don't show search items if calculator is active
        if self.calculator_item.is_some() {
            return Vec::new();
        }

        let detection = detect_search(query);

        match detection {
            SearchDetection::Triggered { provider, query } => {
                // User explicitly triggered a provider (e.g., "!g rust")
                vec![SearchItem::new(provider, query)]
            }
            SearchDetection::Fallback { query } => {
                // Show all providers if we have no other matches
                if !has_matches && !query.is_empty() {
                    get_providers()
                        .into_iter()
                        .map(|provider| SearchItem::new(provider, query.clone()))
                        .collect()
                } else {
                    Vec::new()
                }
            }
            SearchDetection::None => Vec::new(),
        }
    }

    /// Check if a calculator item is currently shown.
    pub fn has_calculator(&self) -> bool {
        self.calculator_item.is_some()
    }

    /// Get the item at a global row index, accounting for calculator and search items.
    fn get_item_at(&self, row: usize) -> Option<ListItem> {
        let mut offset = 0;

        // Calculator is at position 0
        if self.calculator_item.is_some() {
            if row == 0 {
                return self.calculator_item.clone().map(ListItem::Calculator);
            }
            offset += 1;
        }

        // Search items come after calculator
        let search_count = self.search_items.len();
        if row < offset + search_count {
            let search_idx = row - offset;
            return self
                .search_items
                .get(search_idx)
                .cloned()
                .map(ListItem::Search);
        }
        offset += search_count;

        // Regular filtered items come after search
        self.filtered_indices
            .get(row - offset)
            .and_then(|&idx| self.items.get(idx))
            .cloned()
    }

    /// Convert section + row to global selected index.
    fn section_row_to_global(&self, section: usize, row: usize) -> usize {
        let section_type = self.section_type_at(section);
        self.section_start_index(section_type) + row
    }

    /// Convert global index to section + row.
    pub fn global_to_section_row(&self, global: usize) -> (usize, usize) {
        let has_calc = self.calculator_item.is_some();
        let has_search = self.section_info.search_count > 0;
        let has_windows = self.section_info.window_count > 0;
        let has_commands = self.section_info.command_count > 0;

        let calc_offset = if has_calc { 1 } else { 0 };
        let search_end = calc_offset + self.section_info.search_count;
        let window_end = search_end + self.section_info.window_count;
        let command_end = window_end + self.section_info.command_count;

        // Determine which section and compute the row within it
        let mut section_idx = 0;

        if has_calc {
            if global == 0 {
                return (0, 0);
            }
            section_idx += 1;
        }

        if has_search {
            if global < search_end {
                return (section_idx, global - calc_offset);
            }
            section_idx += 1;
        }

        if has_windows {
            if global < window_end {
                return (section_idx, global - search_end);
            }
            section_idx += 1;
        }

        if has_commands {
            if global < command_end {
                return (section_idx, global - window_end);
            }
            section_idx += 1;
        }

        // Must be in Applications section
        (section_idx, global - command_end)
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
        self.calculator_item = None;
        self.search_items.clear();
        self.filter_items();
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.filter_items();
    }

    /// Set query without filtering (for async filtering).
    pub fn set_query_only(&mut self, query: String) {
        self.query = query;
    }

    /// Get current query.
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn filtered_count(&self) -> usize {
        let calc_count = if self.calculator_item.is_some() { 1 } else { 0 };
        let search_count = self.search_items.len();
        self.filtered_indices.len() + calc_count + search_count
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn set_selected(&mut self, index: usize) {
        self.selected_index = Some(index);
    }

    pub fn do_confirm(&self) {
        if let Some(idx) = self.selected_index
            && let Some(item) = self.get_item_at(idx)
            && let Some(ref on_confirm) = self.on_confirm
        {
            on_confirm(&item);
        }
    }

    pub fn do_cancel(&self) {
        if let Some(ref on_cancel) = self.on_cancel {
            on_cancel();
        }
    }

    /// Get the currently selected item, if any.
    pub fn selected_item(&self) -> Option<ListItem> {
        self.selected_index.and_then(|idx| self.get_item_at(idx))
    }

    /// Determine what type of section is at the given section index.
    fn section_type_at(&self, section: usize) -> SectionType {
        let has_calc = self.calculator_item.is_some();
        let has_search = self.section_info.search_count > 0;
        let has_windows = self.section_info.window_count > 0;
        let has_commands = self.section_info.command_count > 0;

        let mut current_section = 0;

        if has_calc {
            if section == current_section {
                return SectionType::Calculator;
            }
            current_section += 1;
        }

        if has_search {
            if section == current_section {
                return SectionType::Search;
            }
            current_section += 1;
        }

        if has_windows {
            if section == current_section {
                return SectionType::Windows;
            }
            current_section += 1;
        }

        if has_commands && section == current_section {
            return SectionType::Commands;
        }
        // current_section += 1; // Not needed, Applications is the default

        // Default to Applications
        SectionType::Applications
    }

    /// Get the starting filtered index for a given section type.
    fn section_start_index(&self, section_type: SectionType) -> usize {
        let has_calc = self.calculator_item.is_some();
        let calc_offset = if has_calc { 1 } else { 0 };

        match section_type {
            SectionType::Calculator => 0,
            SectionType::Search => calc_offset,
            SectionType::Windows => calc_offset + self.section_info.search_count,
            SectionType::Commands => {
                calc_offset + self.section_info.search_count + self.section_info.window_count
            }
            SectionType::Applications => {
                calc_offset
                    + self.section_info.search_count
                    + self.section_info.window_count
                    + self.section_info.command_count
            }
        }
    }
}

impl ListDelegate for ItemListDelegate {
    type Item = GpuiListItem;

    fn sections_count(&self, _cx: &App) -> usize {
        let has_calc = self.calculator_item.is_some();
        let has_search = self.section_info.search_count > 0;
        let has_windows = self.section_info.window_count > 0;
        let has_commands = self.section_info.command_count > 0;
        let has_apps = self.section_info.app_count > 0;

        let mut count = 0;
        if has_calc {
            count += 1;
        }
        if has_search {
            count += 1;
        }
        if has_windows {
            count += 1;
        }
        if has_commands {
            count += 1;
        }
        if has_apps {
            count += 1;
        }
        count
    }

    fn items_count(&self, section: usize, _cx: &App) -> usize {
        let section_type = self.section_type_at(section);
        match section_type {
            SectionType::Calculator => 1,
            SectionType::Search => self.section_info.search_count,
            SectionType::Windows => self.section_info.window_count,
            SectionType::Commands => self.section_info.command_count,
            SectionType::Applications => self.section_info.app_count,
        }
    }

    fn render_section_header(
        &self,
        section: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<impl IntoElement> {
        let section_type = self.section_type_at(section);

        // Calculator and Search sections have no header
        if section_type == SectionType::Calculator || section_type == SectionType::Search {
            return None;
        }

        // Count how many non-calculator, non-search sections we have
        let has_windows = self.section_info.window_count > 0;
        let has_commands = self.section_info.command_count > 0;
        let has_apps = self.section_info.app_count > 0;
        let non_special_section_count =
            has_windows as usize + has_commands as usize + has_apps as usize;

        // Only show headers if we have multiple non-special sections
        if non_special_section_count <= 1 {
            return None;
        }

        let t = theme();
        let title = match section_type {
            SectionType::Calculator => return None,
            SectionType::Search => return None,
            SectionType::Windows => "Windows",
            SectionType::Commands => "Commands",
            SectionType::Applications => "Applications",
        };

        Some(
            div()
                .w_full()
                .px(t.item_margin_x + t.item_padding_x)
                .pt(t.section_header_margin_top)
                .pb(t.section_header_margin_bottom)
                .text_xs()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(t.section_header_color)
                .child(SharedString::from(title)),
        )
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Self::Item> {
        let section_type = self.section_type_at(ix.section);
        let global_idx = self.section_row_to_global(ix.section, ix.row);
        let selected = self.selected_index == Some(global_idx);

        let item = if section_type == SectionType::Calculator {
            self.calculator_item.clone().map(ListItem::Calculator)?
        } else if section_type == SectionType::Search {
            self.search_items
                .get(ix.row)
                .cloned()
                .map(ListItem::Search)?
        } else {
            let start = self.section_start_index(section_type);
            let calc_offset = if self.calculator_item.is_some() { 1 } else { 0 };
            let search_offset = self.search_items.len();
            let filtered_idx = start - calc_offset - search_offset + ix.row;
            let item_idx = *self.filtered_indices.get(filtered_idx)?;
            self.items.get(item_idx)?.clone()
        };

        let item_content = render_item(&item, selected, global_idx);

        // Reset ListItem default padding - we handle all styling ourselves
        Some(
            GpuiListItem::new(("list-item", global_idx))
                .py_0()
                .px_0()
                .child(item_content),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix.map(|i| self.section_row_to_global(i.section, i.row));
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.filter_items();
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
        self.do_cancel();
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
                    .child(SharedString::from("No items found")),
            )
    }
}
