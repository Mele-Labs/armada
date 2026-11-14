//! Which keys a web page keeps and which go back to Bridge, ported from Armada Classic's
//! `main/capture/pane-keys.ts` (`isMainKey`). A page holding the keyboard hears everything, and Bridge's
//! own window hears nothing while it does, so jumping to another Job, ⌘K and Esc would stop working.
//! Pure, so it is read without a window.

/// A key press as AppKit reports it: the hardware key's virtual code, and the modifiers held. The code
/// names the key by its place on the keyboard, as `KeyboardEvent.code` does, so ⌥ and ⇧ do not rewrite it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Press {
    pub code: u16,
    pub command: bool,
    pub option: bool,
    pub control: bool,
    pub shift: bool,
}

// Virtual key codes (`kVK_ANSI_*` and friends in Carbon's `Events.h`).
const KEY_A: u16 = 0;
const KEY_H: u16 = 4;
const KEY_Z: u16 = 6;
const KEY_X: u16 = 7;
const KEY_C: u16 = 8;
const KEY_V: u16 = 9;
const KEY_Q: u16 = 12;
const KEY_W: u16 = 13;
const KEY_COMMA: u16 = 43;
const KEY_M: u16 = 46;
const KEY_TAB: u16 = 48;
const KEY_SPACE: u16 = 49;
const KEY_BACKQUOTE: u16 = 50;
pub const KEY_ESCAPE: u16 = 53;

/// What a page needs for itself under ⌘: editing its text. Its field would otherwise lose copy,
/// paste, cut, select all and undo.
const EDITING: [u16; 5] = [KEY_C, KEY_V, KEY_X, KEY_A, KEY_Z];

/// What the system owns under ⌘, and the app's menu: quitting, closing, minimising, hiding and
/// settings. They are not handed on, so the menu still gets them.
const SYSTEM: [u16; 8] = [KEY_Q, KEY_W, KEY_M, KEY_H, KEY_COMMA, KEY_BACKQUOTE, KEY_TAB, KEY_SPACE];

/// Whether a press is Bridge's own window's: Esc alone, or any press with ⌘ held that is neither
/// editing the page's text nor the system's. A press with ⌃ and no ⌘ is a text field's on macOS.
pub fn is_bridges(press: Press) -> bool {
    if press.code == KEY_ESCAPE {
        return !press.command && !press.option && !press.control && !press.shift;
    }
    if !press.command || press.control {
        return false;
    }
    !EDITING.contains(&press.code) && !SYSTEM.contains(&press.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn plain(code: u16) -> Press {
        Press { code, command: false, option: false, control: false, shift: false }
    }

    const fn with_command(code: u16) -> Press {
        Press { command: true, ..plain(code) }
    }

    #[test]
    fn escape_alone_goes_back_and_escape_with_a_modifier_stays() {
        assert!(is_bridges(plain(KEY_ESCAPE)));
        assert!(!is_bridges(Press { shift: true, ..plain(KEY_ESCAPE) }));
        assert!(!is_bridges(with_command(KEY_ESCAPE)));
    }

    #[test]
    fn a_command_press_goes_back_unless_it_edits_text_or_the_system_owns_it() {
        // ⌘1 to ⌘9 switch tabs, ⌘K the palette, ⌘[ ⌘] and ⌘T: Bridge's.
        for code in [18, 19, 20, 21, 23, 22, 26, 28, 25, 40, 33, 30, 17] {
            assert!(is_bridges(with_command(code)), "code {code}");
        }
        for code in EDITING {
            assert!(!is_bridges(with_command(code)), "editing {code}");
        }
        for code in SYSTEM {
            assert!(!is_bridges(with_command(code)), "system {code}");
        }
    }

    #[test]
    fn option_with_command_still_goes_back_and_control_with_command_does_not() {
        assert!(is_bridges(Press { option: true, ..with_command(124) }));
        assert!(!is_bridges(Press { control: true, ..with_command(18) }));
    }

    #[test]
    fn typing_stays_in_the_page() {
        assert!(!is_bridges(plain(0)));
        assert!(!is_bridges(plain(36)));
        assert!(!is_bridges(Press { shift: true, ..plain(48) }));
        assert!(!is_bridges(Press { control: true, ..plain(0) }));
    }
}
