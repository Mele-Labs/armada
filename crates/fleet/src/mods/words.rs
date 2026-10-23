//! What a Words mod's `words.toml` may say: a pattern, an example it matches whole, and an address
//! for each repository that has one. Checked before Bridge or Fleet reads it.
//!
//! **The pattern and the addresses are held to what a taught link was**, because Bridge runs the
//! pattern as a JavaScript expression on every message it draws and opens the address.

use ipc::WordsFile;

use crate::teaching::link;

/// The most `words.toml` may weigh.
pub(crate) const MOST_BYTES: u64 = 16 * 1024;

const MOST_NAME: usize = 60;
const MOST_ADDRESSES: usize = 100;
/// A Manifest id as `armada.yml` spells one, or `*` for every repository.
const MOST_REPOSITORY: usize = 80;

/// `text` read, or the problem with where it stopped: `words.toml line 3: ...`.
pub(crate) fn read(text: &str) -> Result<WordsFile, String> {
    toml::from_str(text).map_err(|why| crate::mods::toml_problem("words.toml", text, &why))
}

/// Every problem in `text`, empty where Bridge may act on it.
pub(crate) fn problems(text: &str) -> Vec<String> {
    let file = match read(text) {
        Ok(file) => file,
        Err(why) => return vec![why],
    };
    let mut found = Vec::new();
    let say = |found: &mut Vec<String>, said: String| found.push(format!("words.toml: {said}"));
    if file.name.trim().is_empty() || file.name.chars().count() > MOST_NAME || file.name.chars().any(char::is_control) {
        say(&mut found, format!("`name` is 1 to {MOST_NAME} characters on one line"));
    }
    if let Some(why) = link::pattern_fault(&file.pattern, &file.example) {
        say(&mut found, why);
    }
    if file.address.len() > MOST_ADDRESSES {
        say(&mut found, format!("a mod holds at most {MOST_ADDRESSES} addresses"));
    }
    let mut seen: Vec<&str> = Vec::new();
    for one in &file.address {
        let plain = !one.repository.is_empty()
            && one.repository.len() <= MOST_REPOSITORY
            && (one.repository == "*"
                || one.repository.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c)));
        if !plain {
            say(&mut found, format!("`repository = \"{}\"` is a Manifest id or `*`", one.repository));
        } else if seen.contains(&one.repository.as_str()) {
            say(&mut found, format!("`{}` has two addresses", one.repository));
        }
        seen.push(&one.repository);
        if !one.template.contains("{match}") {
            say(&mut found, format!("the address for `{}` has no `{{match}}`", one.repository));
        }
        if let Some(why) = link::address_fault(&one.template) {
            say(&mut found, format!("the address for `{}`: {why}", one.repository));
        }
    }
    found
}
