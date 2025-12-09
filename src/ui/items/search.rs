//! Rendering for search items.

use crate::items::SearchItem;
use crate::ui::theme::theme;
use gpui::{Div, Stateful, div, prelude::*, svg};

use super::{item_container, render_action_indicator, render_text_content};

/// Render a search item with provider icon and query.
pub fn render_search(item: &SearchItem, selected: bool, row: usize) -> Stateful<Div> {
    let mut container = item_container(row, selected)
        .child(render_search_icon(item))
        .child(render_text_content(&item.name, None, selected));

    if selected {
        container = container.child(render_action_indicator("Open"));
    }

    container
}

/// Render the search provider icon.
fn render_search_icon(item: &SearchItem) -> Div {
    let t = theme();
    let size = t.icon_size;

    let icon_container = div()
        .w(size)
        .h(size)
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(t.icon_placeholder_background)
        .rounded_sm();

    icon_container.child(
        svg()
            .path(item.icon().path())
            .size_4()
            .text_color(t.icon_placeholder_color),
    )
}
