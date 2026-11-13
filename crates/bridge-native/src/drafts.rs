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

/// The words kept under `key`, if any stand.
pub fn kept(key: &str, cx: &App) -> Option<String> {
    cx.try_global::<Kept>().and_then(|kept| kept.words.get(key)).filter(|words| !words.is_empty()).cloned()
}

/// Keep `words` under `key`. Empty words clear it.
pub fn keep(key: &str, words: &str, cx: &mut App) {
    let kept = cx.default_global::<Kept>();
    let changed = if words.is_empty() {
        kept.words.remove(key).is_some()
    } else {
        kept.words.insert(key.to_string(), words.to_string()).as_deref() != Some(words)
    };
    if changed && kept.file.is_some() {
        kept.unwritten = true;
        write_soon(cx);
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

/// The whole file, to a temp file beside it and then over it, so a crash part way leaves the old one whole.
fn write(path: &Path, words: &HashMap<String, String>) {
    let entries: HashMap<String, &String> = words.iter().map(|(key, words)| (format!("{PREFIX}{key}"), words)).collect();
    let result = ipc::encode(&entries).map_err(|why| why.to_string()).and_then(|text| {
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, text).and_then(|()| std::fs::rename(&temp, path)).map_err(|why| why.to_string())
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
}
