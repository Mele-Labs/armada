//! `mod.toml`: four `key = "value"` lines.
//!
//! **A subset of TOML read by hand.** A mod's file has no table, no array
//! and no number, and a TOML crate would be a dependency in the daemon to read
//! four strings. Anything outside the subset is a problem that names the line,
//! so a file that is real TOML but not this one is refused rather than half read.

use ipc::ModKind;

/// The most `mod.toml` may weigh.
pub(crate) const MOST_BYTES: u64 = 4 * 1024;

const MOST_VERSION: usize = 32;
pub(crate) const MOST_DESCRIPTION: usize = 200;

/// What was read, field by field. A field that was wrong is `None` and has a
/// problem beside it, so a list can still show the ones that were right.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub(crate) kind: Option<ModKind>,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
}

/// `text` read for the mod in the folder called `folder`, and everything wrong with it.
pub(crate) fn read(text: &str, folder: &str) -> (Manifest, Vec<String>) {
    let mut problems = Vec::new();
    let mut name = None;
    let mut kind = None;
    let mut version = None;
    let mut description = None;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, rest)) = line.split_once('=') else {
            problems.push(format!("mod.toml line {number}: expected `key = \"value\"`"));
            continue;
        };
        let key = key.trim();
        let value = match string(rest.trim()) {
            Ok(value) => value,
            Err(why) => {
                problems.push(format!("mod.toml line {number}: {why}"));
                continue;
            }
        };
        let slot = match key {
            "name" => &mut name,
            "kind" => &mut kind,
            "version" => &mut version,
            "description" => &mut description,
            other => {
                problems.push(format!(
                    "mod.toml line {number}: `{other}` is not a key a mod uses"
                ));
                continue;
            }
        };
        if slot.replace(value).is_some() {
            problems.push(format!("mod.toml line {number}: `{key}` is given twice"));
        }
    }
    let mut manifest = Manifest::default();
    match name {
        None => problems.push("mod.toml has no `name`".to_string()),
        Some(name) if name != folder => problems.push(format!(
            "mod.toml names `{name}` but the folder is `{folder}`"
        )),
        Some(_) => {}
    }
    match kind.as_deref() {
        None => problems.push("mod.toml has no `kind`".to_string()),
        Some("theme") => manifest.kind = Some(ModKind::Theme),
        Some("layout") => manifest.kind = Some(ModKind::Layout),
        Some(other) => problems.push(format!("mod.toml `kind` is `{other}`, which is not a kind of mod this build has")),
    }
    match version {
        None => problems.push("mod.toml has no `version`".to_string()),
        Some(version) if !version_is_plain(&version) => problems.push(format!(
            "mod.toml `version` must be 1 to {MOST_VERSION} letters, digits, `.`, `+` or `-`"
        )),
        Some(version) => manifest.version = Some(version),
    }
    match description {
        Some(text) if text.chars().count() > MOST_DESCRIPTION => problems.push(format!(
            "mod.toml `description` is longer than {MOST_DESCRIPTION} characters"
        )),
        other => manifest.description = other,
    }
    (manifest, problems)
}

fn version_is_plain(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= MOST_VERSION
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".+-".contains(c))
}

/// A double-quoted string, with `\"` and `\\` the only escapes, and perhaps a
/// `#` comment after it.
fn string(text: &str) -> Result<String, String> {
    let Some(inner) = text.strip_prefix('"') else {
        return Err("the value must be a double-quoted string".to_string());
    };
    let mut out = String::new();
    let mut chars = inner.chars();
    loop {
        match chars.next() {
            None => return Err("the string is not closed".to_string()),
            Some('"') => break,
            Some('\\') => match chars.next() {
                Some(c @ ('"' | '\\')) => out.push(c),
                _ => return Err("only \\\" and \\\\ may follow a backslash".to_string()),
            },
            Some(c) if c.is_control() => {
                return Err("a string holds no control characters".to_string())
            }
            Some(c) => out.push(c),
        }
    }
    let after = chars.as_str().trim();
    if after.is_empty() || after.starts_with('#') {
        Ok(out)
    } else {
        Err("nothing may follow the string but a comment".to_string())
    }
}
