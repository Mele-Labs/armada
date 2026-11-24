//! What a text box keeps for the place it belongs to, so closing the box and opening it again
//! finds the words, as Electron's `useKept` does with the `keep` key it is given. The key names
//! the place (`dispatch:request`), the words are cleared when they are sent.
//!
//! The live app also keeps them in a file, so a restart finds them too. The file is read once at
//! start (`persist`); every change is written a moment after the last one, off the main thread,
//! to a temp file renamed over the old one. A run that was not given a file (a walk, a mock, a
//! test) keeps them in memory only and never touches the owner's.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{App, Global, Task};

use crate::log::Note;

/// How long after the last change the file is written.
const QUIET: Duration = Duration::from_millis(300);

/// What Electron's `keep.ts` puts before a key in `localStorage`; the file names its entries the same way,
/// so a later carry-over maps one onto the other.
const PREFIX: &str = "armada:kept:";

#[derive(Default)]
struct Kept {
    words: HashMap<String, String>,
    /// Where they are kept between runs, for the live app alone.
    file: Option<PathBuf>,
    /// A change the file has not seen.
    unwritten: bool,
    /// The wait before the next write; a newer change replaces it, which is the debounce.
    wait: Option<Task<()>>,
}

impl Global for Kept {}

/// The file the live app keeps its drafts in.
pub(crate) fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-drafts.json"))
}

/// What the file holds now, by key.
pub(crate) fn read_file(path: &Path) -> HashMap<String, String> {
    read(path)
}

/// The words kept under `key`, if any stand.
pub fn kept(key: &str, cx: &App) -> Option<String> {
    cx.try_global::<Kept>().and_then(|kept| kept.words.get(key)).filter(|words| !words.is_empty()).cloned()
}

/// Whether any place keeps exactly `words`. A secret must never be one that does.
#[cfg(test)]
pub(crate) fn holds(words: &str, cx: &App) -> bool {
    cx.try_global::<Kept>().is_some_and(|kept| kept.words.values().any(|held| held == words))
}

/// Keep `words` under `key`. Empty words clear it.
pub fn keep(key: &str, words: &str, cx: &mut App) {
    let kept = cx.default_global::<Kept>();
    let changed = if words.is_empty() {
        kept.words.remove(key).is_some()
    } else {
        kept.words.insert(key.to_string(), words.to_string()).as_deref() != Some(words)
    };
    if changed {
        if let Some(file) = kept.file.clone() {
            kept.unwritten = true;
            // Held for a panic's hook to write, until the file holds it.
            if let Some(text) = encoded(&kept.words) {
                crate::crash::hold(&file, text);
            }
            write_soon(cx);
        }
    }
}

/// A send used the words: nothing stays for the place.
pub fn clear(key: &str, cx: &mut App) {
    keep(key, "", cx);
}

/// Every kept draft goes: a screen started over. Only in memory: what is forgotten here is not written,
/// and nothing is written after it, so a walk that starts over cannot take the owner's file with it.
pub fn forget_all(cx: &mut App) {
    cx.set_global(Kept::default());
}

/// Keep the drafts in `file` from here on, and take back what it holds. Called once, at the live app's start,
/// before any box is drawn. `None` leaves them in memory.
pub fn persist(file: Option<PathBuf>, cx: &mut App) {
    let Some(path) = file else { return };
    let words = read(&path);
    cx.set_global(Kept { words, file: Some(path), unwritten: false, wait: None });
    // A change made in the last moments before the app quits is not left to the wait.
    cx.on_app_quit(|cx| {
        let due = cx.try_global::<Kept>().filter(|kept| kept.unwritten).and_then(|kept| kept.file.clone().map(|file| (file, kept.words.clone())));
        async move {
            if let Some((file, words)) = due {
                write(&file, &words);
            }
        }
    })
    .detach();
}

fn read(path: &Path) -> HashMap<String, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return HashMap::new(),
        Err(why) => {
            Note::warn("the kept drafts were not read").field("why", why.to_string()).emit();
            return HashMap::new();
        }
    };
    match ipc::decode::<HashMap<String, String>>("the kept drafts", &bytes) {
        Ok(entries) => entries.into_iter().filter_map(|(key, words)| key.strip_prefix(PREFIX).map(|key| (key.to_string(), words))).filter(|(_, words)| !words.is_empty()).collect(),
        Err(why) => {
            Note::warn("the kept drafts were not understood").field("why", why.to_string()).emit();
            HashMap::new()
        }
    }
}

/// The file's whole text.
fn encoded(words: &HashMap<String, String>) -> Option<String> {
    let entries: HashMap<String, &String> = words.iter().map(|(key, words)| (format!("{PREFIX}{key}"), words)).collect();
    ipc::encode(&entries).ok()
}

/// The whole file, to a temp file beside it and then over it, so a crash part way leaves the old one whole.
fn write(path: &Path, words: &HashMap<String, String>) {
    let result = encoded(words).ok_or_else(|| "the drafts did not encode".to_string()).and_then(|text| {
        crate::crash::write_atomic(path, &text).map(|()| crate::crash::settle(path, &text)).map_err(|why| why.to_string())
    });
    if let Err(why) = result {
        Note::warn("the kept drafts were not written").field("why", why).emit();
    }
}

/// The file is written once the changes have stopped for a moment.
fn write_soon(cx: &mut App) {
    let wait = cx.spawn(async move |cx| {
        cx.background_executor().timer(QUIET).await;
        let due = cx.update(|cx| {
            let kept = cx.try_global::<Kept>()?;
            let file = kept.file.clone()?;
            let words = kept.words.clone();
            cx.global_mut::<Kept>().unwritten = false;
            Some((file, words))
        });
        if let Some((file, words)) = due {
            cx.background_executor().spawn(async move { write(&file, &words) }).await;
        }
    });
    cx.global_mut::<Kept>().wait = Some(wait);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn a_draft_written_is_found_again_when_the_app_starts_over_from_the_file(cx: &mut TestAppContext) {
        let path = std::env::temp_dir().join(format!("armada-drafts-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        cx.update(|cx| {
            persist(Some(path.clone()), cx);
            keep("dispatch:request", "Half a thought", cx);
            keep("operator:compose", "Another", cx);
            clear("operator:compose", cx);
        });
        // A burst of changes is one write, a moment after the last.
        cx.executor().advance_clock(QUIET * 2);
        cx.run_until_parked();
        let text = std::fs::read_to_string(&path).expect("the file was written");
        assert!(text.contains("armada:kept:dispatch:request"), "an entry is named as Electron's is: {text}");
        assert!(!text.contains("operator:compose"), "a cleared key is dropped from the file: {text}");

        // The app starts again: nothing in memory, the file read.
        cx.update(|cx| {
            forget_all(cx);
            assert_eq!(kept("dispatch:request", cx), None);
            persist(Some(path.clone()), cx);
            assert_eq!(kept("dispatch:request", cx).as_deref(), Some("Half a thought"));
            assert_eq!(kept("operator:compose", cx), None);
        });
        let _ = std::fs::remove_file(&path);
    }

    #[gpui::test]
    fn the_sheet_the_operator_a_note_and_a_walk_note_all_come_back_after_a_restart(cx: &mut TestAppContext) {
        let path = std::env::temp_dir().join(format!("armada-drafts-kinds-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let kinds = [("dispatch:request", "Fix the toast\nand the sheet"), ("operator:compose", "{\"text\":\"Ask\",\"tags\":[]}"), ("annotate:note", "The gap is wrong"), ("walk-note:dispatch", "Too tight")];
        cx.update(|cx| {
            persist(Some(path.clone()), cx);
            for (key, words) in kinds {
                keep(key, words, cx);
            }
        });
        cx.executor().advance_clock(QUIET * 2);
        cx.run_until_parked();
        cx.update(|cx| {
            forget_all(cx);
            persist(Some(path.clone()), cx);
            for (key, words) in kinds {
                assert_eq!(kept(key, cx).as_deref(), Some(words), "{key}");
            }
        });
        let _ = std::fs::remove_file(&path);
    }

    #[gpui::test]
    fn words_typed_just_before_an_abort_are_written_from_what_is_held(cx: &mut TestAppContext) {
        let path = std::env::temp_dir().join(format!("armada-drafts-abort-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        cx.update(|cx| {
            persist(Some(path.clone()), cx);
            keep("dispatch:request", "typed and then the app aborted", cx);
        });
        // The wait has not passed, so nothing is written; the hook's flush is what writes it.
        assert!(!path.exists());
        crate::crash::flush(false);
        assert!(read(&path).get("dispatch:request").is_some_and(|words| words == "typed and then the app aborted"));
        let _ = std::fs::remove_file(&path);
    }
}
