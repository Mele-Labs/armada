//! Settings: an overlay over the app, 90% of the window, opened from the title bar's menu, the macOS menu and
//! ⌘,, and from the project switcher on that project. A port of Classic's System page.

pub mod catalogue;
pub(crate) mod change_band;
pub(crate) mod definition;
mod delete;
pub mod entries;
pub(crate) mod gateway;
mod keyboard;
pub mod layout;
pub(crate) mod macs;
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
pub(crate) mod view;
pub mod walk;
pub mod ways;
pub(crate) mod wire;
mod workflow_detail;
mod workflows;
mod lanes;

pub use screen::SettingsScreen;
