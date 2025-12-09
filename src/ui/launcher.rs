use crate::calculator::copy_to_clipboard;
use crate::clipboard::ClipboardContent;
use crate::compositor::Compositor;
use crate::desktop::launch_application;
use crate::items::ListItem;
use crate::ui::clipboard::delegate::ClipboardListDelegate;
use crate::ui::emoji::EmojiGridDelegate;
use crate::ui::items::ItemListDelegate;
use crate::ui::theme::theme;
use gpui::{
    AnyElement, App, AsyncApp, Context, Entity, FocusHandle, Focusable, KeyBinding, Length,
    ScrollStrategy, Task, WeakEntity, Window, actions, div, image_cache, prelude::*, retain_all,
};
use gpui_component::IndexPath;
use gpui_component::input::{Input, InputState};
use gpui_component::list::{List, ListState};
use gpui_component::{ActiveTheme, Icon, IconName};
use std::sync::Arc;

actions!(
    launcher,
    [
        SelectNext,
        SelectPrev,
        SelectTab,
        SelectTabPrev,
        Confirm,
        Cancel,
        GoBack
    ]
);

/// The current view mode of the launcher.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// Main launcher view showing apps, windows, commands.
    #[default]
    Main,
    /// Emoji picker grid view.
    EmojiPicker,
    /// Clipboard history view.
    ClipboardHistory,
}

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectPrev, Some("LauncherView")),
        KeyBinding::new("down", SelectNext, Some("LauncherView")),
        KeyBinding::new("tab", SelectTab, Some("LauncherView")),
        KeyBinding::new("shift-tab", SelectTabPrev, Some("LauncherView")),
        KeyBinding::new("enter", Confirm, Some("LauncherView")),
        KeyBinding::new("escape", Cancel, Some("LauncherView")),
        KeyBinding::new("backspace", GoBack, Some("LauncherView")),
    ]);
}

pub struct LauncherView {
    /// Current view mode (main or emoji picker).
    view_mode: ViewMode,
    /// Main list state.
    list_state: Entity<ListState<ItemListDelegate>>,
    /// Emoji grid state (created on demand).
    emoji_list_state: Option<Entity<ListState<EmojiGridDelegate>>>,
    /// Clipboard history list state (created on demand).
    clipboard_list_state: Option<Entity<ListState<ClipboardListDelegate>>>,
    input_state: Entity<InputState>,
    focus_handle: FocusHandle,
    #[allow(dead_code)] // Kept alive for blur handler
    on_hide: std::sync::Arc<dyn Fn() + Send + Sync>,
    _search_task: Task<()>,
}

impl LauncherView {
    pub fn new(
        items: Vec<ListItem>,
        compositor: Arc<dyn Compositor>,
        on_hide: impl Fn() + Send + Sync + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut delegate = ItemListDelegate::new(items);

        // Set up callbacks using Arc for sharing
        let on_hide = std::sync::Arc::new(on_hide);
        let on_hide_for_confirm = on_hide.clone();
        let on_hide_for_cancel = on_hide.clone();

        delegate.set_on_confirm(move |item| {
            match item {
                ListItem::Application(app) => {
                    // Convert back to DesktopEntry for launching
                    let entry = crate::desktop::DesktopEntry::new(
                        app.id.clone(),
                        app.name.clone(),
                        app.exec.clone(),
                        None,
                        app.icon_path.clone(),
                        app.description.clone(),
                        vec![],
                        app.terminal,
                        app.desktop_path.clone(),
                    );
                    let _ = launch_application(&entry);
                }
                ListItem::Window(win) => {
                    // Focus the window via compositor
                    if let Err(e) = compositor.focus_window(&win.address) {
                        tracing::warn!(%e, "Failed to focus window");
                    }
                }
                ListItem::Calculator(calc) => {
                    // Copy calculator result to clipboard
                    if let Err(e) = copy_to_clipboard(calc.text_for_clipboard()) {
                        tracing::warn!(%e, "Failed to copy to clipboard");
                    }
                }
                ListItem::Action(act) => {
                    // Execute the action (shutdown, reboot, etc.)
                    if let Err(e) = act.execute() {
                        tracing::warn!(%e, "Failed to execute action");
                    }
                }
                ListItem::Search(search) => {
                    // Open the search URL in the default browser
                    if let Err(e) = std::process::Command::new("xdg-open")
                        .arg(&search.url)
                        .spawn()
                    {
                        tracing::warn!(%e, "Failed to open search URL");
                    }
                }
                _ => {}
            }
            on_hide_for_confirm();
        });
        delegate.set_on_cancel(move || on_hide_for_cancel());

        let list_state = cx.new(|cx| ListState::new(delegate, window, cx));

        let input_state =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search applications..."));

        let list_state_for_subscribe = list_state.clone();
        cx.subscribe(&input_state, move |this, input, event, cx| {
            if let gpui_component::input::InputEvent::Change = event {
                let text = input.read(cx).value().to_string();
                this.async_search(text, list_state_for_subscribe.clone(), cx);
            }
        })
        .detach();

        let focus_handle = cx.focus_handle();

        // Hide when the view loses focus (user clicked outside the window)
        let on_hide_for_blur = on_hide.clone();
        cx.on_blur(&focus_handle, window, move |_this, _window, _cx| {
            on_hide_for_blur();
        })
        .detach();

        Self {
            view_mode: ViewMode::Main,
            list_state,
            emoji_list_state: None,
            clipboard_list_state: None,
            input_state,
            focus_handle,
            on_hide,
            _search_task: Task::ready(()),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input_state.update(cx, |input: &mut InputState, cx| {
            input.focus(window, cx);
        });
    }

    pub fn reset_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list_state.update(cx, |list_state, _cx| {
            list_state.delegate_mut().clear_query();
        });
        self.input_state.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
    }

    /// Enter emoji picker mode.
    fn enter_emoji_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Clear search input and update placeholder
        self.input_state.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.set_placeholder("Search emojis...", window, cx);
        });

        // Create emoji delegate
        let on_hide = self.on_hide.clone();
        let mut delegate = EmojiGridDelegate::new();

        delegate.set_on_select(move |emoji| {
            if let Err(e) = copy_to_clipboard(&emoji.emoji) {
                tracing::warn!(%e, "Failed to copy emoji to clipboard");
            }
            on_hide();
        });

        let emoji_list_state = cx.new(|cx| ListState::new(delegate, window, cx));

        // Subscribe to input changes for emoji filtering
        let emoji_state_for_search = emoji_list_state.clone();
        cx.subscribe(&self.input_state, move |_this, input, event, cx| {
            if let gpui_component::input::InputEvent::Change = event {
                let query = input.read(cx).value().to_string();
                emoji_state_for_search.update(cx, |list_state, cx| {
                    list_state.delegate_mut().set_query(query);
                    list_state.delegate_mut().filter();
                    cx.notify();
                });
            }
        })
        .detach();

        self.emoji_list_state = Some(emoji_list_state);
        self.view_mode = ViewMode::EmojiPicker;
        cx.notify();
    }

    /// Exit emoji picker mode and return to main view.
    fn exit_emoji_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_mode = ViewMode::Main;
        self.emoji_list_state = None;

        // Clear search, reset placeholder, and reset main list
        self.input_state.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.set_placeholder("Search applications...", window, cx);
        });
        self.list_state.update(cx, |list_state, _cx| {
            list_state.delegate_mut().clear_query();
        });
        cx.notify();
    }

    /// Enter clipboard history mode.
    fn enter_clipboard_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Clear search input and update placeholder
        self.input_state.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.set_placeholder("Search clipboard history...", window, cx);
        });

        // Create clipboard delegate
        let on_hide = self.on_hide.clone();
        let mut delegate = ClipboardListDelegate::new();

        delegate.set_on_select(move |item| {
            // Copy the selected item back to clipboard
            let result = match &item.content {
                ClipboardContent::Text(text) => copy_to_clipboard(text),
                ClipboardContent::FilePaths(paths) => {
                    let text = paths
                        .iter()
                        .filter_map(|p| p.to_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    copy_to_clipboard(&text)
                }
                ClipboardContent::RichText { plain, .. } => copy_to_clipboard(plain),
                ClipboardContent::Image(_) => {
                    // For images, we'll use arboard directly
                    match arboard::Clipboard::new() {
                        Ok(mut clipboard) => {
                            if let ClipboardContent::Image(bytes) = &item.content {
                                if let Ok(img) = image::load_from_memory(bytes) {
                                    let rgba = img.to_rgba8();
                                    let (width, height) = rgba.dimensions();
                                    let img_data = arboard::ImageData {
                                        width: width as usize,
                                        height: height as usize,
                                        bytes: rgba.into_raw().into(),
                                    };
                                    clipboard.set_image(img_data).map_err(|e| e.to_string())
                                } else {
                                    Err("Failed to decode image".to_string())
                                }
                            } else {
                                Err("Not an image".to_string())
                            }
                        }
                        Err(e) => Err(e.to_string()),
                    }
                }
            };

            if let Err(e) = result {
                tracing::warn!(%e, "Failed to copy clipboard item");
            }
            on_hide();
        });

        let clipboard_back = cx.entity().downgrade();
        delegate.set_on_back(move || {
            if let Some(_this) = clipboard_back.upgrade() {
                // This will be handled by the cancel/go_back action
            }
        });

        let clipboard_list_state = cx.new(|cx| ListState::new(delegate, window, cx));

        // Subscribe to input changes for clipboard filtering
        let clipboard_state_for_search = clipboard_list_state.clone();
        cx.subscribe(&self.input_state, move |_this, input, event, cx| {
            if let gpui_component::input::InputEvent::Change = event {
                let query = input.read(cx).value().to_string();
                clipboard_state_for_search.update(cx, |list_state, cx| {
                    list_state.delegate_mut().set_query(query);
                    cx.notify();
                });
            }
        })
        .detach();

        self.clipboard_list_state = Some(clipboard_list_state);
        self.view_mode = ViewMode::ClipboardHistory;
        cx.notify();
    }

    /// Exit clipboard history mode and return to main view.
    fn exit_clipboard_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_mode = ViewMode::Main;
        self.clipboard_list_state = None;

        // Clear search, reset placeholder, and reset main list
        self.input_state.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.set_placeholder("Search applications...", window, cx);
        });
        self.list_state.update(cx, |list_state, _cx| {
            list_state.delegate_mut().clear_query();
        });
        cx.notify();
    }

    /// Handle back action (backspace or back button).
    fn go_back(&mut self, _: &GoBack, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::EmojiPicker => {
                // Check if input is empty before going back
                let is_empty = self.input_state.read(cx).value().is_empty();
                if is_empty {
                    self.exit_emoji_mode(window, cx);
                }
            }
            ViewMode::ClipboardHistory => {
                // Check if input is empty before going back
                let is_empty = self.input_state.read(cx).value().is_empty();
                if is_empty {
                    self.exit_clipboard_mode(window, cx);
                }
            }
            ViewMode::Main => {}
        }
    }

    fn async_search(
        &mut self,
        query: String,
        list_state: Entity<ListState<ItemListDelegate>>,
        cx: &mut Context<Self>,
    ) {
        // Get items Arc for background processing
        let items = list_state.read(cx).delegate().items();
        let query_clone = query.clone();

        // Update query immediately (without filtering)
        list_state.update(cx, |list_state, _cx| {
            list_state.delegate_mut().set_query_only(query.clone());
        });

        let background = cx.background_executor().clone();

        self._search_task = cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            // Run filtering on background thread
            let filtered_indices = background
                .spawn(async move { ItemListDelegate::filter_items_sync(&items, &query_clone) })
                .await;

            // Apply results on main thread
            let _ = cx.update(|cx| {
                list_state.update(cx, |list_state, cx| {
                    list_state
                        .delegate_mut()
                        .apply_filter_results(query, filtered_indices);
                    cx.notify();
                });
            });
        });
    }

    fn select_next(&mut self, _: &SelectNext, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Main => {
                self.list_state.update(cx, |list_state, cx| {
                    let delegate = list_state.delegate_mut();
                    let count = delegate.filtered_count();
                    if count == 0 {
                        return;
                    }
                    let current = delegate.selected_index().unwrap_or(0);
                    let next = if current + 1 >= count { 0 } else { current + 1 };
                    delegate.set_selected(next);
                    let (section, row) = delegate.global_to_section_row(next);
                    list_state.scroll_to_item(
                        IndexPath::new(row).section(section),
                        ScrollStrategy::Top,
                        window,
                        cx,
                    );
                    cx.notify();
                });
            }
            ViewMode::EmojiPicker => {
                if let Some(ref emoji_state) = self.emoji_list_state {
                    emoji_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_right(); // Linear navigation in grid
                        if let Some(row) = delegate.selected_row() {
                            list_state.scroll_to_item(
                                IndexPath::new(row),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
            ViewMode::ClipboardHistory => {
                if let Some(ref clipboard_state) = self.clipboard_list_state {
                    clipboard_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_down();
                        if let Some(idx) = delegate.selected_index() {
                            list_state.scroll_to_item(
                                IndexPath::new(idx),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
        }
    }

    fn select_prev(&mut self, _: &SelectPrev, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Main => {
                self.list_state.update(cx, |list_state, cx| {
                    let delegate = list_state.delegate_mut();
                    let count = delegate.filtered_count();
                    if count == 0 {
                        return;
                    }
                    let current = delegate.selected_index().unwrap_or(0);
                    let prev = if current == 0 { count - 1 } else { current - 1 };
                    delegate.set_selected(prev);
                    let (section, row) = delegate.global_to_section_row(prev);
                    list_state.scroll_to_item(
                        IndexPath::new(row).section(section),
                        ScrollStrategy::Top,
                        window,
                        cx,
                    );
                    cx.notify();
                });
            }
            ViewMode::EmojiPicker => {
                if let Some(ref emoji_state) = self.emoji_list_state {
                    emoji_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_left(); // Linear navigation in grid
                        if let Some(row) = delegate.selected_row() {
                            list_state.scroll_to_item(
                                IndexPath::new(row),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
            ViewMode::ClipboardHistory => {
                if let Some(ref clipboard_state) = self.clipboard_list_state {
                    clipboard_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_up();
                        if let Some(idx) = delegate.selected_index() {
                            list_state.scroll_to_item(
                                IndexPath::new(idx),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
        }
    }

    /// Tab moves to next item linearly (for both main view and emoji grid).
    fn select_tab(&mut self, _: &SelectTab, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Main => {
                self.list_state.update(cx, |list_state, cx| {
                    let delegate = list_state.delegate_mut();
                    let count = delegate.filtered_count();
                    if count == 0 {
                        return;
                    }
                    let current = delegate.selected_index().unwrap_or(0);
                    let next = if current + 1 >= count { 0 } else { current + 1 };
                    delegate.set_selected(next);
                    let (section, row) = delegate.global_to_section_row(next);
                    list_state.scroll_to_item(
                        IndexPath::new(row).section(section),
                        ScrollStrategy::Top,
                        window,
                        cx,
                    );
                    cx.notify();
                });
            }
            ViewMode::EmojiPicker => {
                if let Some(ref emoji_state) = self.emoji_list_state {
                    emoji_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_right(); // Move to next item linearly
                        if let Some(row) = delegate.selected_row() {
                            list_state.scroll_to_item(
                                IndexPath::new(row),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
            ViewMode::ClipboardHistory => {
                if let Some(ref clipboard_state) = self.clipboard_list_state {
                    clipboard_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_down();
                        if let Some(idx) = delegate.selected_index() {
                            list_state.scroll_to_item(
                                IndexPath::new(idx),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
        }
    }

    /// Shift+Tab moves to previous item linearly.
    fn select_tab_prev(&mut self, _: &SelectTabPrev, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Main => {
                self.list_state.update(cx, |list_state, cx| {
                    let delegate = list_state.delegate_mut();
                    let count = delegate.filtered_count();
                    if count == 0 {
                        return;
                    }
                    let current = delegate.selected_index().unwrap_or(0);
                    let prev = if current == 0 { count - 1 } else { current - 1 };
                    delegate.set_selected(prev);
                    let (section, row) = delegate.global_to_section_row(prev);
                    list_state.scroll_to_item(
                        IndexPath::new(row).section(section),
                        ScrollStrategy::Top,
                        window,
                        cx,
                    );
                    cx.notify();
                });
            }
            ViewMode::EmojiPicker => {
                if let Some(ref emoji_state) = self.emoji_list_state {
                    emoji_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_left(); // Move to previous item linearly
                        if let Some(row) = delegate.selected_row() {
                            list_state.scroll_to_item(
                                IndexPath::new(row),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
            ViewMode::ClipboardHistory => {
                if let Some(ref clipboard_state) = self.clipboard_list_state {
                    clipboard_state.update(cx, |list_state, cx| {
                        let delegate = list_state.delegate_mut();
                        delegate.select_up();
                        if let Some(idx) = delegate.selected_index() {
                            list_state.scroll_to_item(
                                IndexPath::new(idx),
                                ScrollStrategy::Top,
                                window,
                                cx,
                            );
                        }
                        cx.notify();
                    });
                }
            }
        }
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Main => {
                // Check if selected item is a submenu
                let selected_item = self.list_state.read(cx).delegate().selected_item();

                if let Some(ListItem::Submenu(ref submenu)) = selected_item {
                    if submenu.id == "submenu-emojis" {
                        self.enter_emoji_mode(window, cx);
                        return;
                    } else if submenu.id == "submenu-clipboard" {
                        self.enter_clipboard_mode(window, cx);
                        return;
                    }
                }

                // Default confirm for other items
                self.list_state.update(cx, |list_state, _cx| {
                    list_state.delegate_mut().do_confirm();
                });
            }
            ViewMode::EmojiPicker => {
                if let Some(ref emoji_state) = self.emoji_list_state {
                    emoji_state.update(cx, |list_state, _cx| {
                        list_state.delegate_mut().do_confirm();
                    });
                }
            }
            ViewMode::ClipboardHistory => {
                if let Some(ref clipboard_state) = self.clipboard_list_state {
                    clipboard_state.update(cx, |list_state, _cx| {
                        list_state.delegate_mut().do_confirm();
                    });
                }
            }
        }
    }

    fn cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        match self.view_mode {
            ViewMode::Main => {
                self.list_state.update(cx, |list_state, _cx| {
                    list_state.delegate_mut().do_cancel();
                });
            }
            ViewMode::EmojiPicker => {
                self.exit_emoji_mode(window, cx);
            }
            ViewMode::ClipboardHistory => {
                self.exit_clipboard_mode(window, cx);
            }
        }
    }
}

impl Focusable for LauncherView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme();

        // Build input prefix based on view mode
        let input_prefix: AnyElement = match self.view_mode {
            ViewMode::Main => Icon::new(IconName::Search)
                .text_color(cx.theme().muted_foreground)
                .mr_2()
                .into_any_element(),
            ViewMode::EmojiPicker => div()
                .id("back-button")
                .cursor_pointer()
                .mr_2()
                .on_click(cx.listener(|this, _event, window, cx| {
                    this.exit_emoji_mode(window, cx);
                }))
                .child(Icon::new(IconName::ArrowLeft).text_color(cx.theme().muted_foreground))
                .into_any_element(),
            ViewMode::ClipboardHistory => div()
                .id("back-button")
                .cursor_pointer()
                .mr_2()
                .on_click(cx.listener(|this, _event, window, cx| {
                    this.exit_clipboard_mode(window, cx);
                }))
                .child(Icon::new(IconName::ArrowLeft).text_color(cx.theme().muted_foreground))
                .into_any_element(),
        };

        // Build list content based on view mode
        let list_content: AnyElement = match self.view_mode {
            ViewMode::Main => image_cache(retain_all("app-icons"))
                .flex_1()
                .overflow_hidden()
                .py_2()
                .child(List::new(&self.list_state))
                .into_any_element(),
            ViewMode::EmojiPicker => {
                if let Some(ref emoji_state) = self.emoji_list_state {
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .py_2()
                        .child(List::new(emoji_state))
                        .into_any_element()
                } else {
                    div().flex_1().into_any_element()
                }
            }
            ViewMode::ClipboardHistory => {
                if let Some(ref clipboard_state) = self.clipboard_list_state {
                    // Get selected item for preview
                    let selected_item =
                        clipboard_state.read(cx).delegate().selected_item().cloned();

                    div()
                        .flex_1()
                        .overflow_hidden()
                        .flex()
                        .flex_row()
                        // Left column: list (50%)
                        .child(
                            div()
                                .w(Length::Definite(gpui::DefiniteLength::Fraction(0.5)))
                                .h_full()
                                .child(List::new(clipboard_state)),
                        )
                        // Vertical separator
                        .child(div().w(gpui::px(1.0)).h_full().bg(t.window_border))
                        // Right column: preview (50%)
                        .child(
                            div()
                                .flex_1()
                                .h_full()
                                .bg(t.item_background)
                                .rounded(t.item_border_radius)
                                .overflow_hidden()
                                .child(crate::ui::clipboard::render_preview_panel(
                                    selected_item.as_ref(),
                                )),
                        )
                        .into_any_element()
                } else {
                    div().flex_1().into_any_element()
                }
            }
        };

        // Fullscreen backdrop - clicking it closes the launcher
        let on_hide = self.on_hide.clone();
        div()
            .id("launcher-backdrop")
            .key_context("LauncherView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_prev))
            .on_action(cx.listener(Self::select_tab))
            .on_action(cx.listener(Self::select_tab_prev))
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::go_back))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            // Click on backdrop to close
            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _cx| {
                on_hide();
            })
            // Centered launcher panel
            .child(
                div()
                    .id("launcher-panel")
                    .w(t.window_width)
                    .h(t.window_height)
                    .flex()
                    .flex_col()
                    .bg(t.window_background)
                    .rounded(t.window_border_radius)
                    .border_1()
                    .border_color(t.window_border)
                    .overflow_hidden()
                    // Stop click propagation to backdrop
                    .on_mouse_down(gpui::MouseButton::Left, |_event, _window, _cx| {
                        // Do nothing - just stop propagation
                    })
                    // Search input section
                    .child(
                        div()
                            .w_full()
                            .px_2()
                            .py_3()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(
                                Input::new(&self.input_state)
                                    .appearance(false)
                                    .cleanable(true)
                                    .prefix(input_prefix),
                            ),
                    )
                    // List section
                    .child(list_content),
            )
    }
}
