//! Where a Job's arrangement is kept between runs, behind one trait so what keeps it can change without the
//! panes knowing. A store held in memory is installed until something else is: the live app installs a
//! [`FileStore`], one file a key on this Mac (his pick on 14 Nov, until Fleet serves arrangements).
//! A walk, a mock and a test keep memory, so they never read or write his files.
#![allow(dead_code)] // The shared layouts come after the panes are drawn.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{App, BorrowAppContext, Global, Task};

use super::pane_tree::{self as tree, Arrangement};
use crate::log::Note;

/// A place arrangements are kept by key. `write` and `forget` are fire and forget: the caller does not wait.
pub(crate) trait ArrangementStore: 'static {
    /// The arrangement kept under `key`, or None where there is none or what is there is not one.
    fn read(&self, key: &str, cx: &mut App) -> Task<Option<Arrangement>>;
    /// Keeps `arrangement` under `key`.
    fn write(&self, key: &str, arrangement: &Arrangement, cx: &mut App);
    /// Lets go of what is kept under `key`.
    fn forget(&self, key: &str, cx: &mut App);
}

/// The key a Job's own arrangement is kept under, as Classic's `job-panes:job:<id>`.
pub(crate) fn job_key(job_id: &str) -> String {
    format!("job-panes:job:{job_id}")
}

/// The key the layouts shared by every Job are kept under, as Classic's `job-panes:shared`.
pub(crate) const SHARED_KEY: &str = "job-panes:shared";

/// Arrangements held in memory only.
#[derive(Default)]
pub(crate) struct MemoryStore(RefCell<HashMap<String, Arrangement>>);

impl ArrangementStore for MemoryStore {
    fn read(&self, key: &str, _cx: &mut App) -> Task<Option<Arrangement>> {
        Task::ready(self.0.borrow().get(key).cloned())
    }

    fn write(&self, key: &str, arrangement: &Arrangement, _cx: &mut App) {
        self.0.borrow_mut().insert(key.to_string(), arrangement.clone());
    }

    fn forget(&self, key: &str, _cx: &mut App) {
        self.0.borrow_mut().remove(key);
    }
}

/// Arrangements kept in a folder, a JSON file a key, as Classic writes one in `localStorage`. What a file holds is
/// read back through [`tree::revive`], so a file that is not an arrangement of known kinds is no arrangement.
/// Written whole to a file beside it and moved into place, so a crash mid-write leaves the last one.
pub(crate) struct FileStore {
    folder: PathBuf,
}

impl FileStore {
    pub(crate) fn new(folder: PathBuf) -> Self {
        FileStore { folder }
    }

    /// The live app's: `job-panes` beside the Jobs list's own files.
    pub(crate) fn live() -> Option<Self> {
        crate::jobs::deck_file().map(|deck| FileStore::new(deck.with_file_name("job-panes")))
    }

    /// A key's file: what is not a letter, a digit, `-` or `_` reads as `_`.
    fn file(&self, key: &str) -> PathBuf {
        let name: String = key.chars().map(|one| if one.is_ascii_alphanumeric() || one == '-' || one == '_' { one } else { '_' }).collect();
        self.folder.join(format!("{name}.json"))
    }

    fn read_now(&self, key: &str) -> Option<Arrangement> {
        let text = std::fs::read_to_string(self.file(key)).ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        tree::revive(&value)
    }

    fn write_now(&self, key: &str, arrangement: &Arrangement) -> std::io::Result<()> {
        let text = serde_json::to_string(arrangement).map_err(std::io::Error::other)?;
        std::fs::create_dir_all(&self.folder)?;
        let path = self.file(key);
        let fresh = path.with_extension("json.new");
        std::fs::write(&fresh, text)?;
        std::fs::rename(&fresh, &path)
    }
}

impl ArrangementStore for FileStore {
    fn read(&self, key: &str, _cx: &mut App) -> Task<Option<Arrangement>> {
        Task::ready(self.read_now(key))
    }

    fn write(&self, key: &str, arrangement: &Arrangement, _cx: &mut App) {
        if let Err(why) = self.write_now(key, arrangement) {
            Note::warn("a Job's panes were not kept").field("key", key).field("why", why.to_string()).emit();
        }
    }

    fn forget(&self, key: &str, _cx: &mut App) {
        let _ = std::fs::remove_file(self.file(key));
    }
}

/// The store the app keeps arrangements in: memory until `install` puts another in its place.
struct Stored(Box<dyn ArrangementStore>);

impl Default for Stored {
    fn default() -> Self {
        Stored(Box::new(MemoryStore::default()))
    }
}

impl Global for Stored {}

/// Puts `store` where arrangements are kept from now on. What the last one held is not carried over.
pub(crate) fn install(store: impl ArrangementStore, cx: &mut App) {
    cx.set_global(Stored(Box::new(store)));
}

/// The arrangement kept under `key` in the installed store.
pub(crate) fn read(key: &str, cx: &mut App) -> Task<Option<Arrangement>> {
    cx.update_default_global(|stored: &mut Stored, cx| stored.0.read(key, cx))
}

/// Keeps `arrangement` under `key` in the installed store.
pub(crate) fn write(key: &str, arrangement: &Arrangement, cx: &mut App) {
    cx.update_default_global(|stored: &mut Stored, cx| stored.0.write(key, arrangement, cx));
}

/// Lets go of what the installed store keeps under `key`.
pub(crate) fn forget(key: &str, cx: &mut App) {
    cx.update_default_global(|stored: &mut Stored, cx| stored.0.forget(key, cx));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_view::pane_tree::{add_tab, default_arrangement, Extras};
    use gpui::TestAppContext;

    #[test]
    fn a_jobs_key_is_the_one_classic_keeps_it_under() {
        assert_eq!(job_key("abc"), "job-panes:job:abc");
        assert_eq!(SHARED_KEY, "job-panes:shared");
    }

    #[gpui::test]
    fn the_installed_store_writes_reads_and_forgets_by_key(cx: &mut TestAppContext) {
        let first = default_arrangement();
        let focus = first.focus.clone();
        let second = add_tab(&first, &focus, "files", Extras::default()).into_owned();
        let key = job_key("a");
        let read_back = |cx: &mut TestAppContext, key: &str| {
            let task = cx.update(|cx| read(key, cx));
            cx.foreground_executor().block_test(task)
        };

        // Memory is the store until another is installed.
        assert_eq!(read_back(cx, &key), None);
        cx.update(|cx| write(&key, &first, cx));
        assert_eq!(read_back(cx, &key), Some(first.clone()));
        assert_eq!(read_back(cx, &job_key("b")), None);

        cx.update(|cx| write(&key, &second, cx));
        assert_eq!(read_back(cx, &key), Some(second));

        cx.update(|cx| forget(&key, cx));
        assert_eq!(read_back(cx, &key), None);
    }

    /// A folder of its own under the system's temporary one, gone when the test ends.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("armada-pane-store-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Folder(path)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[gpui::test]
    fn the_file_store_keeps_an_arrangement_across_a_restart(cx: &mut TestAppContext) {
        let folder = Folder::new("restart");
        let kept = default_arrangement();
        let focus = kept.focus.clone();
        let kept = add_tab(&kept, &focus, "files", Extras::default()).into_owned();
        let key = job_key("a");
        cx.update(|cx| install(FileStore::new(folder.0.clone()), cx));
        cx.update(|cx| write(&key, &kept, cx));
        assert!(folder.0.join("job-panes_job_a.json").exists(), "written to a file named by its key");

        // A restart: a store made fresh over the same folder reads what the last one wrote.
        cx.update(|cx| install(FileStore::new(folder.0.clone()), cx));
        let task = cx.update(|cx| read(&key, cx));
        assert_eq!(cx.foreground_executor().block_test(task), Some(kept));

        cx.update(|cx| forget(&key, cx));
        let task = cx.update(|cx| read(&key, cx));
        assert_eq!(cx.foreground_executor().block_test(task), None);
    }

    #[gpui::test]
    fn a_file_that_is_not_an_arrangement_reads_as_none(cx: &mut TestAppContext) {
        let folder = Folder::new("broken");
        std::fs::create_dir_all(&folder.0).expect("folder");
        std::fs::write(folder.0.join("job-panes_job_a.json"), "{\"root\": 3}").expect("written");
        std::fs::write(folder.0.join("job-panes_job_b.json"), "not json").expect("written");
        let mut unknown = serde_json::to_value(default_arrangement()).expect("encodes");
        unknown["root"]["a"]["tabs"][0]["kind"] = serde_json::Value::from("no-such-kind");
        std::fs::write(folder.0.join("job-panes_job_c.json"), unknown.to_string()).expect("written");
        cx.update(|cx| install(FileStore::new(folder.0.clone()), cx));
        for job in ["a", "b", "c", "d"] {
            let task = cx.update(|cx| read(&job_key(job), cx));
            assert_eq!(cx.foreground_executor().block_test(task), None, "job {job}");
        }
    }

    #[gpui::test]
    fn another_store_installed_takes_the_place_of_memory(cx: &mut TestAppContext) {
        let kept = default_arrangement();
        cx.update(|cx| write("job-panes:job:a", &kept, cx));
        cx.update(|cx| install(MemoryStore::default(), cx));
        let task = cx.update(|cx| read("job-panes:job:a", cx));
        assert_eq!(cx.foreground_executor().block_test(task), None);
    }
}
