//! Settings: an overlay over the app, 90% of the window, opened from the title bar's menu, the macOS menu and
//! ⌘,, and from the project switcher on that project. A port of Classic's System page.

pub mod catalogue;
pub mod entries;
mod keyboard;
mod mock;
mod mods;
pub mod overlay;
mod project;
pub mod screen;
mod theme;
mod view;
pub mod walk;
mod wire;

pub use screen::SettingsScreen;
