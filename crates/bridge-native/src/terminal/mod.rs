//! A Job's terminals: shells Fleet keeps, each drawn as a pane the person types in and watches
//! (`t1`, `t2`, ...), and the terminal Job that opens straight onto one.
//!
//! | Part | File |
//! |---|---|
//! | The grid, scrollback and selection, on `alacritty_terminal` | `screen.rs` |
//! | The bytes a key sends | `keys.rs` |
//! | The screen's colours from the tokens | `colors.rs` |
//! | Fleet's shells, or a walk's | `shells.rs` |
//! | The pane: its head, the drawn screen, keys and the pointer | `pane.rs` |

pub mod colors;
pub mod keys;
pub mod pane;
pub mod screen;
pub mod shells;

pub use pane::{PaneEvent, Phase, TerminalPane};

use gpui::{App, KeyBinding};

/// The keys a terminal takes before the app does: ⌘C copies its selection, ⌘V pastes into the
/// shell, ⌘A selects the whole history. The menu's Edit items reach the same acts.
pub fn init(cx: &mut App) {
    use gpui_component::input::{Copy, Paste, SelectAll};
    cx.bind_keys([
        KeyBinding::new("cmd-c", Copy, Some(pane::KEYS)),
        KeyBinding::new("cmd-v", Paste, Some(pane::KEYS)),
        KeyBinding::new("cmd-a", SelectAll, Some(pane::KEYS)),
    ]);
}
