//! A style set kept by its content: `<attachments>/styles/<hash>.json`, written once and never
//! again. The drawing then stays as it was when the app's CSS moves on, and what a Job's record
//! carries is the hash and not the CSS, since the record is published whole on every change.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use base64::Engine;
use ipc::{KeptFont, KeptSheet, KeptStyles};
use sha2::{Digest, Sha256};

use super::sheet;
use super::walk::inside;

/// Most the sheets of one set weigh together.
pub(crate) const SET_AT_MOST: usize = 512 * 1024;
/// Most font bytes of one set. Past it the faces are left out and the frame uses system fonts.
pub(crate) const FONTS_AT_MOST: u64 = 1024 * 1024;

/// The sheets at `paths`, in the order given, as a set a frame can adopt. `root` is the checkout's
/// canonical path and `dirty` the files it holds uncommitted changes to. Where `fit` is set a sheet
/// that would take the set past its cap is left out and the rest kept, so the nearest come first in
/// `paths`; otherwise it is an error, since the agent named them.
pub(crate) fn assemble(root: &Path, paths: &[String], dirty: &BTreeSet<String>, fit: bool) -> Result<KeptStyles, String> {
    let (mut sheets, mut fonts, mut total) = (Vec::new(), Vec::<KeptFont>::new(), 0usize);
    let mut faces: Vec<sheet::Face> = Vec::new();
    for path in paths {
        let file = inside(root, path).filter(|file| file.is_file()).ok_or_else(|| format!("{path} is not a file of the checkout"))?;
        let made = match sheet::make(root, &file) {
            Ok(made) => made,
            Err(_) if fit => continue,
            Err(why) => return Err(format!("{path}: {why}")),
        };
        if total + made.css.len() > SET_AT_MOST {
            if fit {
                continue;
            }
            return Err(format!("{path} brings the set past {SET_AT_MOST} bytes, the most a set keeps"));
        }
        total += made.css.len();
        let edited = made.read.iter().filter_map(|one| one.strip_prefix(root).ok()).any(|one| dirty.contains(&one.to_string_lossy().into_owned()));
        sheets.push(KeptSheet { path: path.clone(), css: made.css, edited });
        for face in made.faces {
            if !faces.iter().any(|one| (&one.family, &one.weight, &one.style) == (&face.family, &face.weight, &face.style)) {
                faces.push(face);
            }
        }
    }
    let weight: u64 = faces.iter().map(|face| std::fs::metadata(&face.file).map_or(0, |meta| meta.len())).sum();
    if weight <= FONTS_AT_MOST {
        for face in faces {
            if let Ok(bytes) = std::fs::read(&face.file) {
                let bytes = base64::engine::general_purpose::STANDARD.encode(bytes);
                fonts.push(KeptFont { family: face.family, weight: face.weight, style: face.style, bytes });
            }
        }
    }
    Ok(KeptStyles { sheets, fonts })
}

/// The first 32 hex characters of the set's digest, which names its file.
pub(crate) fn hash_of(set: &KeptStyles) -> io::Result<String> {
    let written = ipc::encode(set).map_err(|why| io::Error::other(why.to_string()))?;
    Ok(Sha256::digest(written.as_bytes()).iter().take(16).map(|byte| format!("{byte:02x}")).collect())
}

pub(crate) fn folder(attachments: &str) -> PathBuf {
    Path::new(attachments).join("styles")
}

/// Keep `set` under its hash, unless the file is there, and answer the hash.
pub(crate) fn keep(folder: &Path, set: &KeptStyles) -> io::Result<String> {
    let hash = hash_of(set)?;
    let file = folder.join(format!("{hash}.json"));
    if file.exists() {
        return Ok(hash);
    }
    std::fs::create_dir_all(folder)?;
    let written = ipc::encode(set).map_err(|why| io::Error::other(why.to_string()))?;
    let staged = folder.join(format!("{hash}.writing"));
    std::fs::write(&staged, written)?;
    std::fs::rename(staged, file)?;
    Ok(hash)
}

/// Whether `hash` is what [`hash_of`] writes: 32 lower-case hex characters, so it cannot name a path.
pub(crate) fn is_hash(hash: &str) -> bool {
    hash.len() == 32 && hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The set kept under `hash`, or `None` where there is none or it will not read.
pub(crate) fn read(folder: &Path, hash: &str) -> Option<KeptStyles> {
    if !is_hash(hash) {
        return None;
    }
    let bytes = std::fs::read(folder.join(format!("{hash}.json"))).ok()?;
    ipc::decode::<KeptStyles>("a kept style set", &bytes).ok()
}

