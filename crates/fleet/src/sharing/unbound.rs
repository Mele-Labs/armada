//! Pieces whose remote matched no repository, kept until one is chosen.
//!
//! One file a piece in Kit's `unbound/`, which is a one-piece package, so the text that was added
//! is the text that is kept. The file's name is a hash of that text: adding the same piece twice
//! keeps one.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::package::{self, Piece};

/// The folder under Kit's home.
const FOLDER: &str = "unbound";
/// Characters of the hash a name keeps. Sixteen hex digits.
const NAME_LEN: usize = 16;

fn file_of(kit: &Path, id: &str) -> PathBuf {
    kit.join(FOLDER).join(format!("{id}.toml"))
}

/// Whether `id` is a name this module made: nothing else reaches a path.
fn is_made_here(id: &str) -> bool {
    id.len() == NAME_LEN && id.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Keep `piece` and answer the id it is kept under.
pub(super) fn keep(kit: &Path, piece: &Piece) -> Result<String, String> {
    let text = package::written(vec![piece.clone()])?;
    let digest: String = Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
    let id = digest[..NAME_LEN].to_string();
    let at = file_of(kit, &id);
    super::write_over(&at, &text).map_err(|why| format!("{} could not be written: {why}", at.display()))?;
    Ok(id)
}

/// The piece kept under `id`. `None` for an id nothing is kept under or one that does not read.
pub(super) fn held(kit: &Path, id: &str) -> Option<Piece> {
    if !is_made_here(id) {
        return None;
    }
    let text = std::fs::read_to_string(file_of(kit, id)).ok()?;
    package::read(&text).ok()?.into_iter().next()
}

/// Every piece kept, by id.
pub(super) fn all(kit: &Path) -> Vec<(String, Piece)> {
    let Ok(entries) = std::fs::read_dir(kit.join(FOLDER)) else { return Vec::new() };
    let mut ids: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.path().file_stem().map(|stem| stem.to_string_lossy().to_string()))
        .filter(|id| is_made_here(id))
        .collect();
    ids.sort();
    ids.into_iter().filter_map(|id| held(kit, &id).map(|piece| (id, piece))).collect()
}

/// Forget `id`, once it is bound.
pub(super) fn forget(kit: &Path, id: &str) -> std::io::Result<()> {
    match std::fs::remove_file(file_of(kit, id)) {
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
