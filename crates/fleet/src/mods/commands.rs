//! What a Commands mod's `commands.toml` may say: rows for Bridge's command palette, each doing
//! one of four things. Checked before Bridge reads it.

use ipc::CommandsFile;

/// The most `commands.toml` may weigh.
pub(crate) const MOST_BYTES: u64 = 16 * 1024;

/// `text` read, with each row's blanks filled in, or the problem with where it stopped.
pub(crate) fn read(text: &str) -> Result<CommandsFile, String> {
    toml::from_str(text).map_err(|why| crate::mods::toml_problem("commands.toml", text, &why))
}

/// Every problem in `text`, empty where Bridge may act on it.
pub(crate) fn problems(text: &str) -> Vec<String> {
    match read(text) {
        Ok(_) => Vec::new(),
        Err(why) => vec![why],
    }
}
