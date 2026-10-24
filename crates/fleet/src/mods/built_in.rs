//! Built-in mods: every folder under the repository's `packages/mods/`, where a merged promote
//! puts one, compiled into Fleet by `build.rs` and listed beside this machine's own.
//!
//! **An id of its own**: `built-in:` and the folder's name, which no slug can be, so a switch, a
//! theme preference or a check names the built-in mod or the local one and never both. **Checked
//! by the rules a local mod is**, from the text compiled in, so one that breaks a rule is a row
//! with its problem and is never applied.

use std::sync::Arc;

/// Ahead of a built-in mod's folder name in its id. No slug has a `:`.
pub(crate) const PREFIX: &str = "built-in:";

/// Each folder's name and its files, as `build.rs` found them under `packages/mods/`.
const COMPILED: &[(&str, &[(&str, &[u8])])] = include!(concat!(env!("OUT_DIR"), "/built_in_mods.rs"));

/// One built-in mod: its folder's name and the files it holds.
#[derive(Clone, Debug)]
pub struct BuiltIn {
    name: String,
    files: Vec<(String, Vec<u8>)>,
}

impl BuiltIn {
    #[cfg(test)]
    pub(crate) fn new(name: &str, files: &[(&str, &str)]) -> BuiltIn {
        let files = files.iter().map(|(file, text)| (file.to_string(), text.as_bytes().to_vec())).collect();
        BuiltIn { name: name.to_string(), files }
    }

    /// The folder's name, which its `mod.toml` repeats.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn id(&self) -> String {
        format!("{PREFIX}{}", self.name)
    }

    /// `file`, held to what [`super::read_small`] holds a file on disk to: there, at most `most`
    /// bytes, and UTF-8.
    pub(super) fn read(&self, file: &str, most: u64) -> Result<String, String> {
        let Some((_, bytes)) = self.files.iter().find(|(held, _)| held == file) else {
            return Err(format!("{file} is missing"));
        };
        if bytes.len() as u64 > most {
            return Err(format!("{file} is larger than {most} bytes"));
        }
        String::from_utf8(bytes.clone()).map_err(|_| format!("{file} is not UTF-8 text"))
    }
}

/// The built-in mods a Fleet lists. Empty by default, so a case lists only what it plants.
#[derive(Clone, Debug, Default)]
pub struct BuiltIns(Arc<Vec<BuiltIn>>);

impl BuiltIns {
    /// The set this build was compiled with.
    pub fn compiled() -> BuiltIns {
        let held = |files: &[(&str, &[u8])]| files.iter().map(|(file, bytes)| (file.to_string(), bytes.to_vec())).collect();
        BuiltIns::of(COMPILED.iter().map(|(name, files)| BuiltIn { name: name.to_string(), files: held(files) }).collect())
    }

    pub(crate) fn of(mods: Vec<BuiltIn>) -> BuiltIns {
        BuiltIns(Arc::new(mods))
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &BuiltIn> {
        self.0.iter()
    }

    /// The built-in mod `id` names, where it names one.
    pub(crate) fn named(&self, id: &str) -> Option<&BuiltIn> {
        let name = id.strip_prefix(PREFIX)?;
        self.0.iter().find(|one| one.name == name)
    }
}
