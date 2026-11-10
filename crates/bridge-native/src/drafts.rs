//! What a text box keeps for the place it belongs to, so closing the box and opening it again
//! finds the words, as Electron's `useKept` does with the `keep` key it is given. The key names
//! the place (`dispatch:request`), the words are cleared when they are sent.

use std::collections::HashMap;

use gpui::{App, Global};

#[derive(Default)]
struct Kept(HashMap<String, String>);

impl Global for Kept {}

/// The words kept under `key`, if any stand.
pub fn kept(key: &str, cx: &App) -> Option<String> {
    cx.try_global::<Kept>().and_then(|kept| kept.0.get(key)).filter(|words| !words.is_empty()).cloned()
}

/// Keep `words` under `key`. Empty words clear it.
pub fn keep(key: &str, words: &str, cx: &mut App) {
    let kept = cx.default_global::<Kept>();
    if words.is_empty() {
        kept.0.remove(key);
    } else {
        kept.0.insert(key.to_string(), words.to_string());
    }
}

/// A send used the words: nothing stays for the place.
pub fn clear(key: &str, cx: &mut App) {
    keep(key, "", cx);
}

/// Every kept draft goes: a screen started over.
pub fn forget_all(cx: &mut App) {
    cx.set_global(Kept::default());
}
