//! Settings: an overlay over the app, 90% of the window, opened from the title bar's menu, the macOS menu and
//! ⌘,, and from the project switcher on that project. A port of Classic's System page.

pub mod catalogue;
mod change_band;
mod definition;
pub mod entries;
mod gateway;
mod keyboard;
pub mod layout;
mod macs;
mod mock;
mod mock_workflows;
mod models;
mod mods;
mod prefs;
pub mod overlay;
mod project;
pub mod screen;
mod theme;
pub mod time;
mod view;
pub mod walk;
mod wire;
mod workflow_detail;
mod workflows;

pub use screen::SettingsScreen;
