//! Settings: an overlay over the app, 90% of the window, opened from the title bar's menu, the macOS menu and
//! ⌘,, and from the project switcher on that project. A port of Classic's System page.

pub mod entries;
mod mock;
pub mod overlay;
mod project;
pub mod screen;
mod view;
pub mod walk;
mod wire;

pub use screen::SettingsScreen;
