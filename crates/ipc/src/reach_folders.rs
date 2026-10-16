//! The folders the owner let every session reach past its slots, and the ones he blocked: the
//! value of the `features.reachFolders` setting, written by an "always" answer on a session's card
//! and by Settings removing one. `docs/concepts/session.md`, *Modes*.
//!
//! **Here for `key_bindings`' reason**: JSON is read only in `ipc` and `store`. Which entry wins
//! is the guard's, in `fleet`.

use serde::{Deserialize, Serialize};

/// The most folders either list keeps. A list that long is a hand edit, not a person answering.
pub const MOST_FOLDERS: usize = 256;

/// Both lists, each an absolute path per entry, in the order they were added.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachFolders {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub block: Vec<String>,
}

impl ReachFolders {
    /// The lists `text` holds. **Lenient**: text that is not the shape reads as empty lists, and
    /// an entry that is not an absolute path is dropped, so a hand edit gone wrong asks again
    /// rather than letting anything through.
    pub fn read(text: &str) -> ReachFolders {
        let read: ReachFolders = serde_json::from_str(text).unwrap_or_default();
        let kept = |all: Vec<String>| -> Vec<String> {
            all.into_iter()
                .filter(|one| one.starts_with('/'))
                .take(MOST_FOLDERS)
                .collect()
        };
        ReachFolders {
            allow: kept(read.allow),
            block: kept(read.block),
        }
    }

    /// `folder` on one list and off the other, which is what a second answer about it means.
    pub fn with(mut self, folder: &str, blocked: bool) -> ReachFolders {
        self.allow.retain(|one| one != folder);
        self.block.retain(|one| one != folder);
        let list = if blocked { &mut self.block } else { &mut self.allow };
        list.push(folder.to_string());
        list.truncate(MOST_FOLDERS);
        self
    }
}
