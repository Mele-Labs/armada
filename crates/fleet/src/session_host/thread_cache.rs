//! The threads Bridge has opened, kept drawn so opening one again costs a `stat` and the tail.
//! **Reading a 9 MB transcript from the start on every click took 2.6 to 4.4 s.**
//!
//! An entry is dropped when its session ends or when nobody has asked for it in [`KEPT_FOR`].
//! The watcher keeps its own offset: it publishes what it reads as events, and a click that
//! had already taken those rows would leave a window that is subscribed without them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adapters::terminal_thread::{self, Followed, Subagent};
use ipc::SessionRow;

/// How long a thread nobody asks for stays in memory.
const KEPT_FOR: Duration = Duration::from_secs(30 * 60);

/// A session, and one of its subagents where it is a subagent's thread.
type Key = (String, Option<String>);

struct Entry {
    asked_at: Instant,
    /// Locked on its own, so a first read of one big transcript holds up no other session's.
    followed: Arc<Mutex<Option<Followed>>>,
}

#[derive(Default)]
pub struct ThreadCache {
    entries: Mutex<HashMap<Key, Entry>>,
}

impl ThreadCache {
    /// Session `id`'s thread as its transcript stands: the file, its rows and where they reach.
    /// **Blocks on the file**, so it runs off the async runtime.
    pub fn thread(&self, home: &str, id: &str) -> (Option<PathBuf>, Vec<SessionRow>, u64) {
        let Some(file) = terminal_thread::find(home, id) else {
            return (None, Vec::new(), 0);
        };
        let read = self.followed((id.to_string(), None), &file, Followed::thread, |one| {
            (one.rows().to_vec(), one.next())
        });
        match read {
            Ok((rows, next)) => (Some(file), rows, next),
            Err(_) => (Some(file), Vec::new(), 0),
        }
    }

    /// Subagent `agent`'s thread, as `read_subagent` draws it. Blocks on the file.
    pub fn subagent(&self, file: &Path, id: &str, agent: &str) -> std::io::Result<Subagent> {
        let key = (id.to_string(), Some(agent.to_string()));
        self.followed(key, file, Followed::subagent, |one| {
            let (finished, report) = one.finished();
            Subagent {
                rows: one.rows().to_vec(),
                finished,
                report,
            }
        })
    }

    /// Drop everything kept for session `id`.
    pub fn forget(&self, id: &str) {
        self.entries.lock().expect("held across no panic").retain(|key, _| key.0 != id);
    }

    fn followed<T>(
        &self,
        key: Key,
        file: &Path,
        start: fn(&Path) -> std::io::Result<Followed>,
        read: impl FnOnce(&Followed) -> T,
    ) -> std::io::Result<T> {
        let slot = {
            let mut entries = self.entries.lock().expect("held across no panic");
            entries.retain(|_, one| one.asked_at.elapsed() < KEPT_FOR);
            let one = entries.entry(key).or_insert_with(|| Entry {
                asked_at: Instant::now(),
                followed: Arc::default(),
            });
            one.asked_at = Instant::now();
            Arc::clone(&one.followed)
        };
        let mut held = slot.lock().expect("held across no panic");
        let caught_up = match held.as_mut().filter(|one| one.follows(file)) {
            Some(one) => one.catch_up().map(|()| ()),
            None => start(file).map(|one| *held = Some(one)),
        };
        if let Err(why) = caught_up {
            *held = None;
            return Err(why);
        }
        Ok(read(held.as_ref().expect("just kept")))
    }
}
