mod application;
mod base;
mod calculator;
mod delegate;
mod search;

pub use application::render_application;
pub use base::{item_container, render_action_indicator, render_icon, render_text_content};
pub use calculator::render_calculator;
pub use delegate::ItemListDelegate;
pub use search::render_search;

use crate::assets::PhosphorIcon;
use crate::items::ListItem;
use crate::ui::theme::theme;
use gpui::{Div, SharedString, Stateful, div, prelude::*, svg};

/// Render any list item based on its type.
/// This is the main dispatch function for item rendering.
pub fn render_item(item: &ListItem, selected: bool, row: usize) -> Stateful<Div> {
    match item {
        ListItem::Application(app) => render_application(app, selected, row),
        ListItem::Window(win) => render_window(win, selected, row),
        ListItem::Action(act) => render_action(act, selected, row),
        ListItem::Submenu(sub) => render_submenu(sub, selected, row),
        ListItem::Calculator(calc) => render_calculator(calc, selected, row),
        ListItem::Search(search) => render_search(search, selected, row),
    }
}

// Placeholder renderers for future item types

fn render_window(win: &crate::items::WindowItem, selected: bool, row: usize) -> Stateful<Div> {
    let mut item = item_container(row, selected)
        .child(render_icon(win.icon_path.as_ref()))
        .child(render_text_content(
            &win.title,
            Some(&win.description),
            selected,
        ));

    if selected {
        item = item.child(render_action_indicator("Switch"));
    }

    item
}

fn render_action(act: &crate::items::ActionItem, selected: bool, row: usize) -> Stateful<Div> {
    let mut item = item_container(row, selected)
        .child(render_phosphor_icon(act.icon_name.as_deref()))
        .child(render_text_content(
            &act.name,
            act.description.as_deref(),
            selected,
        ));

    if selected {
        item = item.child(render_action_indicator("Run"));
    }

    item
}

fn render_submenu(sub: &crate::items::SubmenuItem, selected: bool, row: usize) -> Stateful<Div> {
    let mut item = item_container(row, selected)
        .child(render_phosphor_icon(sub.icon_name.as_deref()))
        .child(render_text_content(
            &sub.name,
            sub.description.as_deref(),
            selected,
        ));

    if selected {
        // Show arrow instead of "Open" to indicate submenu
        item = item.child(render_action_indicator("→"));
    }

    item
}

/// Render a Phosphor icon from embedded SVG assets.
fn render_phosphor_icon(icon_name: Option<&str>) -> Div {
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

    // Try to get the Phosphor icon
    if let Some(icon) = icon_name.and_then(PhosphorIcon::from_name) {
        icon_container.child(
            svg()
                .path(icon.path())
                .size_4()
                .text_color(t.icon_placeholder_color),
        )
    } else {
        // Fallback to placeholder
        icon_container.child(
            div()
                .text_sm()
                .text_color(t.icon_placeholder_color)
                .child(SharedString::from("?")),
        )
    }
}
