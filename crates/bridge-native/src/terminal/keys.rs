//! The bytes a key sends to the shell, as xterm.js sends them in Electron (`XtermScreen.tsx:170`).
//!
//! **Only keys that are not text are answered here.** A letter, a symbol, an Option-composed
//! character and a dead key's result come through the screen's text input, as they do in xterm.js's
//! hidden textarea; Control chords, named keys and the arrows are read from the key. ⌘ chords are never
//! the shell's: they stay the app's, so ⌘K, ⌘T and ⌘1 work from a terminal. Option is not Meta, as
//! xterm.js's `macOptionIsMeta` is off, so ⌥ types the character macOS gives it.

use alacritty_terminal::term::TermMode;
use gpui::Keystroke;

/// The xterm modifier parameter: 1 plus Shift 1, Alt 2, Control 4. None for a bare key.
fn modifier(key: &Keystroke) -> Option<u8> {
    let m = &key.modifiers;
    let code = 1 + u8::from(m.shift) + 2 * u8::from(m.alt) + 4 * u8::from(m.control);
    (code > 1).then_some(code)
}

/// `ESC [ 1 ; m <final>` with modifiers, else the bare form: `ESC O <final>` in application cursor
/// mode, `ESC [ <final>` otherwise.
fn cursor(key: &Keystroke, last: u8, app: bool) -> Vec<u8> {
    match modifier(key) {
        Some(m) => format!("\x1b[1;{m}{}", last as char).into_bytes(),
        None if app => vec![0x1b, b'O', last],
        None => vec![0x1b, b'[', last],
    }
}

/// `ESC [ n ~`, or `ESC [ n ; m ~` with modifiers.
fn tilde(key: &Keystroke, n: u8) -> Vec<u8> {
    match modifier(key) {
        Some(m) => format!("\x1b[{n};{m}~").into_bytes(),
        None => format!("\x1b[{n}~").into_bytes(),
    }
}

/// What the key sends, or None where the key is the app's or comes as text.
pub fn bytes(key: &Keystroke, mode: TermMode) -> Option<Vec<u8>> {
    let m = &key.modifiers;
    if m.platform || m.function && key.key.len() == 1 {
        return None;
    }
    let app = mode.contains(TermMode::APP_CURSOR);
    let sent = match key.key.as_str() {
        "enter" if m.alt => b"\x1b\r".to_vec(),
        "enter" => b"\r".to_vec(),
        "escape" => b"\x1b".to_vec(),
        "tab" if m.shift => b"\x1b[Z".to_vec(),
        "tab" => b"\t".to_vec(),
        "backspace" if m.control => b"\x08".to_vec(),
        "backspace" if m.alt => b"\x1b\x7f".to_vec(),
        "backspace" => b"\x7f".to_vec(),
        "space" if m.control => vec![0],
        "up" => cursor(key, b'A', app),
        "down" => cursor(key, b'B', app),
        "right" => cursor(key, b'C', app),
        "left" => cursor(key, b'D', app),
        "home" => cursor(key, b'H', app),
        "end" => cursor(key, b'F', app),
        "insert" => tilde(key, 2),
        "delete" => tilde(key, 3),
        "pageup" => tilde(key, 5),
        "pagedown" => tilde(key, 6),
        "f1" => cursor(key, b'P', true),
        "f2" => cursor(key, b'Q', true),
        "f3" => cursor(key, b'R', true),
        "f4" => cursor(key, b'S', true),
        "f5" => tilde(key, 15),
        "f6" => tilde(key, 17),
        "f7" => tilde(key, 18),
        "f8" => tilde(key, 19),
        "f9" => tilde(key, 20),
        "f10" => tilde(key, 21),
        "f11" => tilde(key, 23),
        "f12" => tilde(key, 24),
        one if m.control && one.chars().count() == 1 => return control(one.chars().next()?),
        _ => return None,
    };
    Some(sent)
}

/// A Control chord: `⌃A` to `⌃Z` are 1 to 26, and the symbols xterm gives a code of their own.
fn control(key: char) -> Option<Vec<u8>> {
    let code = match key.to_ascii_lowercase() {
        letter @ 'a'..='z' => letter as u8 - b'a' + 1,
        '@' | '2' | ' ' => 0,
        '[' | '3' => 0x1b,
        '\\' | '4' => 0x1c,
        ']' | '5' => 0x1d,
        '^' | '6' => 0x1e,
        '_' | '-' | '/' | '7' => 0x1f,
        '8' | '?' => 0x7f,
        _ => return None,
    };
    Some(vec![code])
}

/// Pasted words as the shell takes them: line ends as Enter, inside the bracketed-paste marks when a
/// program asked for them (xterm.js's `prepareTextForTerminal` and `bracketTextForPaste`).
pub fn pasted(text: &str, mode: TermMode) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if mode.contains(TermMode::BRACKETED_PASTE) {
        format!("\x1b[200~{text}\x1b[201~").into_bytes()
    } else {
        text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(spelled: &'static str) -> Keystroke {
        Keystroke::parse(spelled).unwrap_or_else(|_| panic!("{spelled} reads"))
    }

    fn sent(spelled: &'static str) -> Option<Vec<u8>> {
        bytes(&key(spelled), TermMode::default())
    }

    #[test]
    fn named_keys_send_xterms_bytes() {
        assert_eq!(sent("enter"), Some(b"\r".to_vec()));
        assert_eq!(sent("escape"), Some(b"\x1b".to_vec()));
        assert_eq!(sent("backspace"), Some(b"\x7f".to_vec()));
        assert_eq!(sent("tab"), Some(b"\t".to_vec()));
        assert_eq!(sent("shift-tab"), Some(b"\x1b[Z".to_vec()));
        assert_eq!(sent("delete"), Some(b"\x1b[3~".to_vec()));
        assert_eq!(sent("f1"), Some(b"\x1bOP".to_vec()));
        assert_eq!(sent("f5"), Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn arrows_follow_application_cursor_mode_and_carry_modifiers() {
        assert_eq!(sent("up"), Some(b"\x1b[A".to_vec()));
        assert_eq!(bytes(&key("up"), TermMode::APP_CURSOR), Some(b"\x1bOA".to_vec()));
        assert_eq!(sent("alt-left"), Some(b"\x1b[1;3D".to_vec()));
        assert_eq!(sent("ctrl-right"), Some(b"\x1b[1;5C".to_vec()));
        assert_eq!(sent("shift-pageup"), Some(b"\x1b[5;2~".to_vec()));
    }

    #[test]
    fn control_chords_are_c0_codes() {
        assert_eq!(sent("ctrl-c"), Some(vec![3]));
        assert_eq!(sent("ctrl-d"), Some(vec![4]));
        assert_eq!(sent("ctrl-["), Some(vec![0x1b]));
        assert_eq!(sent("ctrl-space"), Some(vec![0]));
    }

    #[test]
    fn letters_and_command_chords_are_not_sent_from_the_key() {
        assert_eq!(sent("a"), None, "a letter comes as text");
        assert_eq!(sent("cmd-k"), None, "⌘K is the app's");
        assert_eq!(sent("cmd-t"), None);
        assert_eq!(sent("cmd-enter"), None);
    }

    #[test]
    fn a_paste_turns_line_ends_to_enter_and_brackets_when_asked() {
        assert_eq!(pasted("a\nb\r\nc", TermMode::default()), b"a\rb\rc".to_vec());
        assert_eq!(pasted("ls", TermMode::BRACKETED_PASTE), b"\x1b[200~ls\x1b[201~".to_vec());
    }
}
