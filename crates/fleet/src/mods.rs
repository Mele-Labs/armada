//! Mods: folders under the machine's `mods/` that change how Bridge looks.
//! `docs/concepts/mods.md`.
//!
//! **Nothing here can fail Fleet.** A mod is a folder anybody on this machine
//! can write into, so every read is bounded, refuses a link, and turns whatever
//! is wrong into a problem on that mod's own row. A list with one broken mod in
//! it is still a list.
//!
//! This file reads and lists; [`manifest`] and [`stylesheet`] say what is
//! acceptable, [`scaffold`] makes one, [`promote`] copies one onto a branch and
//! [`serving`] is the part that speaks for Fleet.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use ipc::{Instant, ModKind, ModList, ModSummary};

pub(crate) mod manifest;
mod promote;
pub(crate) mod scaffold;
mod serving;
pub(crate) mod stylesheet;

pub use serving::keep_reading;

/// How often the folder is rescanned for a mod a session wrote.
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(2);

const MOST_NAME: usize = 40;

/// Names Bridge's own themes answer to. A mod called one of these would be
/// chosen by an id that already means something else.
const SHIPPED: &[&str] = &["dark", "light", "system", "default"];

const MANIFEST: &str = "mod.toml";
const THEME: &str = "theme.css";
const LAYOUT: &str = "layout.json";

/// Why `name` cannot be a mod's name, or `None`. **The only gate on a path
/// segment**: a name that passes holds no `/`, no `.` and no way to climb.
pub(crate) fn slug_problem(name: &str) -> Option<String> {
    theme_problem(name).or_else(|| {
        SHIPPED
            .contains(&name)
            .then(|| format!("`{name}` is the name of a theme Bridge ships"))
    })
}

/// Why `id` cannot name a theme, a shipped one or a mod's, or `None`.
pub(crate) fn theme_problem(id: &str) -> Option<String> {
    let plain = !id.is_empty()
        && id.len() <= MOST_NAME
        && id.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    (!plain).then(|| {
        format!(
            "a name is 1 to {MOST_NAME} lowercase letters, digits and `-`, starting with a letter or digit"
        )
    })
}

/// The prefix of a theme Bridge ships in its catalogue. No mod's name has a `:`.
const CATALOGUE: &str = "catalogue:";

/// Why `id` cannot be saved as the theme: a plain name, or a catalogue theme's
/// id, which is Bridge's own spelling and is a plain name after its prefix.
pub(crate) fn theme_id_problem(id: &str) -> Option<String> {
    theme_problem(id.strip_prefix(CATALOGUE).unwrap_or(id))
}

/// One mod's folder, read.
pub(crate) struct Examined {
    pub(crate) name: String,
    pub(crate) manifest: manifest::Manifest,
    pub(crate) problems: Vec<String>,
    /// The two files as they were checked, kept only where nothing at all was
    /// wrong with the mod. Whatever is handed on is these bytes and not a second read.
    pub(crate) files: Option<Files>,
    pub(crate) changed_ms: Option<i64>,
}

pub(crate) struct Files {
    pub(crate) manifest: String,
    pub(crate) payload: Payload,
}

/// The file a mod's kind is about, as it was checked.
pub(crate) enum Payload {
    Css(String),
    Layout(String),
}

impl Payload {
    /// The file's name in the mod's folder.
    pub(crate) fn file(&self) -> &'static str {
        match self {
            Payload::Css(_) => THEME,
            Payload::Layout(_) => LAYOUT,
        }
    }

    pub(crate) fn text(&self) -> &str {
        match self {
            Payload::Css(text) | Payload::Layout(text) => text,
        }
    }
}

impl Examined {
    pub(crate) fn valid(&self) -> bool {
        self.problems.is_empty()
    }

    pub(crate) fn summary(&self, enabled: bool) -> ModSummary {
        ModSummary {
            name: self.name.clone(),
            kind: self.manifest.kind,
            version: self.manifest.version.clone(),
            description: self.manifest.description.clone(),
            enabled,
            valid: self.valid(),
            reason: self.problems.first().map(|first| match self.problems.len() {
                1 => first.clone(),
                more => format!("{first} (and {} more)", more - 1),
            }),
            changed_at: self
                .changed_ms
                .map(|ms| Instant::carried(crate::clock::rfc3339_utc(ms))),
        }
    }
}

/// The mod in `mods_dir/name`. **`name` must have passed [`slug_problem`]**,
/// or be a folder's own name the listing is describing.
pub(crate) fn examine(mods_dir: &Path, name: &str) -> Examined {
    let mut found = Examined {
        name: name.to_string(),
        manifest: manifest::Manifest::default(),
        problems: Vec::new(),
        files: None,
        changed_ms: None,
    };
    if let Some(why) = slug_problem(name) {
        found.problems.push(why);
        return found;
    }
    let dir = mods_dir.join(name);
    match std::fs::symlink_metadata(&dir) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            found.problems.push("the mod's folder is a link, not a folder".to_string());
            return found;
        }
        Err(_) => {
            found.problems.push("the mod's folder cannot be read".to_string());
            return found;
        }
    }
    let mut stamps = Vec::new();
    let mut texts = (None, None);
    match read_small(&dir.join(MANIFEST), manifest::MOST_BYTES, MANIFEST) {
        Ok((text, ms)) => {
            stamps.push(ms);
            let (read, problems) = manifest::read(&text, name);
            found.manifest = read;
            found.problems.extend(problems);
            texts.0 = Some(text);
        }
        Err(why) => found.problems.push(why),
    }
    // A manifest that names no kind this build has is read as a theme, as it always was, so what
    // is said about a broken mod's stylesheet does not change.
    let layout = found.manifest.kind == Some(ModKind::Layout);
    let (file, most) = if layout { (LAYOUT, ipc::layout::MOST_BYTES as u64) } else { (THEME, stylesheet::MOST_BYTES) };
    match read_small(&dir.join(file), most, file) {
        Ok((text, ms)) => {
            stamps.push(ms);
            found.problems.extend(if layout { ipc::layout::problems(&text) } else { stylesheet::problems(&text) });
            texts.1 = Some(text);
        }
        Err(why) => found.problems.push(why),
    }
    found.changed_ms = stamps.into_iter().max();
    if let (true, (Some(manifest), Some(text))) = (found.problems.is_empty(), texts) {
        let payload = if layout { Payload::Layout(text) } else { Payload::Css(text) };
        found.files = Some(Files { manifest, payload });
    }
    found
}

/// A regular file of at most `most` bytes, and when it was last written.
///
/// **Opened without following a link**, so a `theme.css` that points at some
/// other file reads as a problem and not as that file. Opened without blocking,
/// so a named pipe is refused rather than waited on.
fn read_small(path: &Path, most: u64, what: &str) -> Result<(String, i64), String> {
    let file: File = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|why| match why.kind() {
            std::io::ErrorKind::NotFound => format!("{what} is missing"),
            _ if why.raw_os_error() == Some(libc::ELOOP) => format!("{what} is a link"),
            _ => format!("{what} cannot be read ({})", why.kind()),
        })?;
    let meta = file
        .metadata()
        .map_err(|why| format!("{what} cannot be read ({})", why.kind()))?;
    if !meta.is_file() {
        return Err(format!("{what} is not a regular file"));
    }
    if meta.len() > most {
        return Err(format!("{what} is larger than {most} bytes"));
    }
    let mut bytes = Vec::new();
    file.take(most + 1)
        .read_to_end(&mut bytes)
        .map_err(|why| format!("{what} cannot be read ({})", why.kind()))?;
    let text = String::from_utf8(bytes).map_err(|_| format!("{what} is not UTF-8 text"))?;
    let ms = meta
        .modified()
        .ok()
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_millis() as i64);
    Ok((text, ms))
}

/// Every folder in `mods_dir`, by name. A folder that does not exist yet is an
/// empty list, and one that will not read is the same: there is nothing to show.
///
/// **Files and names that begin with `.` are not mods** and are not listed. A
/// folder whose name is not a slug is listed, invalid, so the owner can see why
/// the thing they made is not offered.
pub(crate) fn scan(mods_dir: &Path, switches: &BTreeMap<String, bool>) -> ModList {
    let Ok(entries) = std::fs::read_dir(mods_dir) else {
        return ModList::default();
    };
    let mut mods: Vec<ModSummary> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir() || kind.is_symlink()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .map(|name| {
            let enabled = switches.get(&name).copied().unwrap_or(true);
            examine(mods_dir, &name).summary(enabled)
        })
        .collect();
    mods.sort_by(|a, b| a.name.cmp(&b.name));
    ModList { mods }
}

/// The list as last published, so a rescan says something only when it moved.
#[derive(Default)]
pub(crate) struct Told(Mutex<Option<ModList>>);

impl Told {
    /// Take `now` as the last published. **True where it differs from one held
    /// before**: the first scan is a baseline, since a client reads `list_mods`
    /// on connect.
    pub(crate) fn moved(&self, now: &ModList) -> bool {
        let mut last = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let was = last.replace(now.clone());
        was.is_some_and(|was| was != *now)
    }

    /// Take `now` as published by something that published it.
    pub(crate) fn published(&self, now: &ModList) {
        *self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(now.clone());
    }
}
