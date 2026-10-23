//! What forgetting a Logbook entry takes with it: the mods that entry's teaching made.
//!
//! A mod records the entry it came from as `entry` in its `mod.toml`, so the entry is the one thing
//! that says the teaching was the person's and was withdrawn. A mod written without an entry (a
//! teaching on Bridge's own interface) is never taken here.

use std::path::Path;

use crate::mods;

/// Remove every mod under `mods_dir` whose `mod.toml` names `entry_id` as its entry, and answer
/// their names. **Called inside forget's store lock**, so no save can write a mod for the entry
/// between the entry going and the mods following it. A folder that is a link is left alone: only a
/// folder this machine made is removed.
pub(crate) fn forget_effects(mods_dir: &Path, entry_id: &str) -> Vec<String> {
    let Ok(listing) = std::fs::read_dir(mods_dir) else {
        return Vec::new();
    };
    let mut gone = Vec::new();
    for name in listing.filter_map(Result::ok).map(|one| one.file_name().to_string_lossy().into_owned()) {
        let real = std::fs::symlink_metadata(mods_dir.join(&name)).is_ok_and(|meta| meta.is_dir());
        if real && mods::entry_of(mods_dir, &name).as_deref() == Some(entry_id) && std::fs::remove_dir_all(mods_dir.join(&name)).is_ok() {
            gone.push(name);
        }
    }
    gone.sort();
    gone
}
