mod action;
mod application;
mod calculator;
mod search;
mod submenu;
mod window;

pub use action::{ActionItem, ActionKind};
pub use application::ApplicationItem;
pub use calculator::CalculatorItem;
pub use search::SearchItem;
pub use submenu::{SubmenuItem, SubmenuLayout};
pub use window::WindowItem;

use std::path::PathBuf;

/// A list item that can be displayed in the launcher.
/// This enum abstracts over different types of items that can appear in the list.
#[derive(Clone, Debug)]
pub enum ListItem {
    /// A desktop application
    Application(ApplicationItem),
    /// An open window (for window switching)
    Window(WindowItem),
    /// A functional action (shutdown, reboot, etc.)
    Action(ActionItem),
    /// A submenu that opens a nested view
    Submenu(SubmenuItem),
    /// A calculator result
    Calculator(CalculatorItem),
    /// A web search item
    Search(SearchItem),
}

impl ListItem {
    /// Get the unique identifier for this item.
    pub fn id(&self) -> &str {
        match self {
            Self::Application(app) => &app.id,
            Self::Window(win) => &win.id,
            Self::Action(act) => &act.id,
            Self::Submenu(sub) => &sub.id,
            Self::Calculator(calc) => &calc.id,
            Self::Search(search) => &search.id,
        }
    }

    /// Get the display name for this item.
    pub fn name(&self) -> &str {
        match self {
            Self::Application(app) => &app.name,
            Self::Window(win) => &win.title,
            Self::Action(act) => &act.name,
            Self::Submenu(sub) => &sub.name,
            Self::Calculator(calc) => &calc.expression,
            Self::Search(search) => &search.name,
        }
    }

    /// Get the description/subtitle for this item.
    pub fn description(&self) -> Option<&str> {
        match self {
            Self::Application(app) => app.description.as_deref(),
            Self::Window(win) => Some(&win.description),
            Self::Action(act) => act.description.as_deref(),
            Self::Submenu(sub) => sub.description.as_deref(),
            Self::Calculator(calc) => Some(&calc.display_result),
            Self::Search(_) => None,
        }
    }

    /// Get the icon path for this item.
    pub fn icon_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Application(app) => app.icon_path.as_ref(),
            Self::Window(win) => win.icon_path.as_ref(),
            Self::Action(_) => None,     // Actions use icon names, not paths
            Self::Submenu(_) => None,    // Submenus use icon names, not paths
            Self::Calculator(_) => None, // Calculator uses custom icon
            Self::Search(_) => None,     // Search uses Phosphor icons
        }
    }

    /// Check if this item is a submenu.
    pub fn is_submenu(&self) -> bool {
        matches!(self, Self::Submenu(_))
    }

    /// Check if this item is an application.
    pub fn is_application(&self) -> bool {
        matches!(self, Self::Application(_))
    }

    /// Check if this item is a window.
    pub fn is_window(&self) -> bool {
        matches!(self, Self::Window(_))
    }

    /// Check if this item is an action.
    pub fn is_action(&self) -> bool {
        matches!(self, Self::Action(_))
    }

    /// Check if this item is a calculator result.
    pub fn is_calculator(&self) -> bool {
        matches!(self, Self::Calculator(_))
    }

    /// Get the action label to display (e.g., "Open", "Switch", "Run").
    pub fn action_label(&self) -> &'static str {
        match self {
            Self::Application(_) => "Open",
            Self::Window(_) => "Switch",
            Self::Action(_) => "Run",
            Self::Submenu(_) => "Open",
            Self::Calculator(_) => "Copy",
            Self::Search(_) => "Open",
        }
    }

    /// Get the sort priority for this item type.
    /// Lower values appear first in the list.
    /// Calculator (0) < Search (1) < Windows (2) < Commands/Actions (3) < Applications (4)
    pub fn sort_priority(&self) -> u8 {
        match self {
            Self::Calculator(_) => 0,
            Self::Search(_) => 1,
            Self::Window(_) => 2,
            Self::Submenu(_) => 3,
            Self::Action(_) => 3, // Actions are grouped with Commands
            Self::Application(_) => 4,
        }
    }

    /// Get the section name for this item type.
    pub fn section_name(&self) -> &'static str {
        match self {
            Self::Calculator(_) => "Calculator",
            Self::Search(_) => "Search",
            Self::Window(_) => "Windows",
            Self::Submenu(_) => "Commands",
            Self::Action(_) => "Commands", // Actions are grouped with Commands
            Self::Application(_) => "Applications",
        }
    }
}

// Convenient From implementations

impl From<ApplicationItem> for ListItem {
    fn from(item: ApplicationItem) -> Self {
        Self::Application(item)
    }
}

impl From<WindowItem> for ListItem {
    fn from(item: WindowItem) -> Self {
        Self::Window(item)
    }
}

impl From<ActionItem> for ListItem {
    fn from(item: ActionItem) -> Self {
        Self::Action(item)
    }
}

impl From<SubmenuItem> for ListItem {
    fn from(item: SubmenuItem) -> Self {
        Self::Submenu(item)
    }
}

impl From<CalculatorItem> for ListItem {
    fn from(item: CalculatorItem) -> Self {
        Self::Calculator(item)
    }
}

impl From<SearchItem> for ListItem {
    fn from(item: SearchItem) -> Self {
        Self::Search(item)
    }
}
