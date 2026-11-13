//! Where a Job's arrangement is kept between runs, behind one trait so what keeps it can change (a file,
//! Fleet) without the panes knowing. Nothing here names a file or a wire. A store held in memory is
//! installed until something else is.
#![allow(dead_code)] // PR 2 draws the panes from this.

use std::cell::RefCell;
use std::collections::HashMap;

use gpui::{App, BorrowAppContext, Global, Task};

use super::pane_tree::Arrangement;

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

    #[gpui::test]
    fn another_store_installed_takes_the_place_of_memory(cx: &mut TestAppContext) {
        let kept = default_arrangement();
        cx.update(|cx| write("job-panes:job:a", &kept, cx));
        cx.update(|cx| install(MemoryStore::default(), cx));
        let task = cx.update(|cx| read("job-panes:job:a", cx));
        assert_eq!(cx.foreground_executor().block_test(task), None);
    }
}
