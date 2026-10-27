//! The owner's folder list and the reading of a path: where a word of a command reaches, and whether that is outside.

use super::*;

/// The owner's folder list, the folders and kinds of call this session was allowed until it ends,
/// and the home a `~` or `$HOME` stands for.
#[derive(Clone, Copy, Default)]
pub struct Folders<'a> {
    pub allow: &'a [String],
    pub block: &'a [String],
    pub session: &'a [String],
    pub kinds: &'a [KindAllowed],
    pub home: &'a str,
}

/// A kind of call the owner allowed for this session alone, narrowed to a folder where he chose
/// one. In memory only, as the session's folders are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindAllowed {
    pub kind: CallKind,
    pub folder: Option<String>,
}

/// What the list says about one path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Listed {
    Allowed,
    Blocked(String),
    Unlisted,
}

impl Folders<'_> {
    /// **The deepest entry holding `path` wins, and a block wins a tie.** The session's own folders
    /// are read after the owner's list, so any block of his wins over them.
    pub(super) fn listed(&self, path: &str) -> Listed {
        let Some(path) = resolved(path, "/") else {
            return Listed::Unlisted;
        };
        let holds = |entry: &String| folder_holds(entry, &path);
        let deepest = |list: &[String]| {
            list.iter()
                .filter_map(|entry| {
                    let at = resolved(entry, "/")?;
                    path.starts_with(&at).then(|| (at.components().count(), entry.clone()))
                })
                .max_by_key(|(depth, _)| *depth)
        };
        match (deepest(self.allow), deepest(self.block)) {
            (Some((allowed, _)), Some((blocked, folder))) if blocked >= allowed => Listed::Blocked(folder),
            (None, Some((_, folder))) => Listed::Blocked(folder),
            (Some(_), _) => Listed::Allowed,
            _ if self.session.iter().any(holds) => Listed::Allowed,
            _ => Listed::Unlisted,
        }
    }

    /// Whether a kind allowed for this session takes a reach into `path` by a segment of `kind`:
    /// the same program, the same script where the entry names one, and inside its folder where it
    /// has one. **Read only after [`Folders::listed`]**, so a block of the owner's still wins.
    pub(super) fn kind_allowed(&self, kind: Option<&CallKind>, path: &str) -> bool {
        let (Some(kind), Some(path)) = (kind, resolved(path, "/")) else {
            return false;
        };
        self.kinds.iter().any(|allowed| {
            allowed.kind.program == kind.program
                && allowed.kind.script.as_ref().is_none_or(|script| kind.script.as_ref() == Some(script))
                && allowed.folder.as_deref().is_none_or(|folder| folder_holds(folder, &path))
        })
    }

    /// `word` as an absolute path, `~` and `$HOME` read as the home, with `.` and `..` folded.
    /// `None` for a relative path or one spelled through another variable.
    pub(super) fn absolute(&self, word: &str) -> Option<String> {
        let word = word.rsplit('=').next().unwrap_or(word);
        let word = word.trim_matches(|c| c == '\'' || c == '"');
        // A search path such as `$HOME/.rbenv/shims:$PATH` names its first folder.
        let word = word.split(':').next().unwrap_or(word);
        let whole = ["~", "${HOME}", "$HOME"]
            .iter()
            .find_map(|home| word.strip_prefix(home).filter(|rest| rest.is_empty() || rest.starts_with('/')))
            .map(|rest| format!("{}{rest}", self.home.trim_end_matches('/')))
            .unwrap_or_else(|| word.to_string());
        if !whole.starts_with('/') || whole.contains('$') {
            return None;
        }
        let mut parts: Vec<&str> = Vec::new();
        for part in whole.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop()?;
                }
                _ => parts.push(part),
            }
        }
        Some(format!("/{}", parts.join("/")))
    }
}

/// Whether the folder `entry` holds `path`, both read where they land, a part at a time.
fn folder_holds(entry: &str, path: &std::path::Path) -> bool {
    resolved(entry, "/").is_some_and(|at| path.starts_with(&at))
}

/// [`because_in_a_session_owning`], and the owner's folder list read first: a path in a blocked
/// folder is refused with no card, one in an allowed folder passes the reach, and anything else is
/// put to him with the path it named.
pub fn guarded_in_a_session(
    asking: &AskingToRun,
    held: &[&str],
    own: Own,
    folders: Folders,
) -> Option<Guarded> {
    let asked = |because: Because, reaching: Option<String>, to: ReachesTo| {
        Some(Guarded::Because { because, reaching, to: Some(to), kind: None })
    };
    let tool = asking.tool_name.as_str();
    if WRITES_A_FILE.contains(&tool) {
        let path = asking
            .input
            .get("file_path")
            .or_else(|| asking.input.get("notebook_path"))
            .and_then(|named| named.as_str());
        if path.is_some_and(|path| {
            in_scratch(path) || held.iter().any(|one| inside_the_directory(path, one))
        }) {
            return None;
        }
        // **Any write outside the slots and scratch is put to the owner** (16 Oct 2026), a new
        // file as much as an overwrite.
        let Some(reaching) = path.and_then(|path| folders.absolute(path)) else {
            return because(asking).map(|because| Guarded::Because { because, reaching: None, to: None, kind: None });
        };
        return match folders.listed(&reaching) {
            Listed::Blocked(folder) => Some(Guarded::Blocked { folder }),
            Listed::Allowed => None,
            Listed::Unlisted => asked(
                because(asking).unwrap_or(Because::ReachesOutside),
                Some(reaching),
                ReachesTo::Write,
            ),
        };
    }
    if tool == "Bash" {
        let line = flattened(asking.detail().unwrap_or_default());
        return shell_guarded(&line, &slots_of(held), own, Some(folders));
    }
    let named = ["file_path", "path"]
        .iter()
        .filter_map(|key| asking.input.get(*key)?.as_str());
    let reads = matches!(tool, "Read" | "Grep" | "Glob" | "LS");
    let walks = matches!(tool, "Grep" | "Glob");
    let to = if reads { ReachesTo::Read } else { ReachesTo::Write };
    for path in named {
        if !(reaches_outside(path) && (!reads || read_refused(path, walks))) {
            continue;
        }
        let reaching = folders.absolute(path);
        match reaching.as_deref().map(|at| folders.listed(at)) {
            Some(Listed::Blocked(folder)) => return Some(Guarded::Blocked { folder }),
            Some(Listed::Allowed) => {}
            _ => return asked(Because::ReachesOutside, reaching, to),
        }
    }
    because(asking).map(|because| Guarded::Because { because, reaching: None, to: None, kind: None })
}

/// The web pages a shell line opens in the owner's own browser: `open`,
/// `xdg-open` or `python -m webbrowser` with an `http(s)` address. A file or a
/// folder is not a page, and neither is a word that merely contains one.
pub fn pages_opened(line: &str) -> Vec<String> {
    let chained = flattened(line).replace("&&", ";").replace("||", ";");
    let mut pages = Vec::new();
    for segment in chained.split(|c| c == ';' || c == '|' || c == '\n') {
        let words: Vec<&str> = segment
            .split_whitespace()
            .map(|word| word.trim_matches(|c| c == '\'' || c == '"'))
            .collect();
        let opens = match words.first().copied() {
            Some("open" | "xdg-open") => true,
            Some(python) if python.starts_with("python") => {
                words.windows(2).any(|pair| pair == ["-m", "webbrowser"])
            }
            _ => false,
        };
        if opens {
            pages.extend(
                words
                    .iter()
                    .filter(|word| word.starts_with("http://") || word.starts_with("https://"))
                    .map(|word| word.to_string()),
            );
        }
    }
    pages
}

/// Whether a path a session names reaches into the owner's home, another
/// volume or the whole disk. **A path that names Armada is let through**: it
/// is most likely looking for something of Armada's (the owner, 8 Oct 2026).
/// The system's own folders (`/opt`, `/usr`, `/tmp`) are no one's files.
pub(in crate::helm) fn reaches_outside(word: &str) -> bool {
    let word = word.rsplit('=').next().unwrap_or(word);
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    if word.to_lowercase().contains("armada") {
        return false;
    }
    let root = word.trim_end_matches('/');
    // Only `/` itself: jq's `//` is an operator, not the disk.
    word == "/"
        || matches!(root, "/Users" | "/Volumes")
        || word.starts_with('~')
        || word.contains("$HOME")
        || word.contains("${HOME}")
        || word.starts_with("/Users/")
        || word.starts_with("/Volumes/")
}

/// Folders under the owner's home that macOS puts a privacy prompt in front of, and the ones
/// holding his credentials. Compared without case, as the disk does.
pub(super) const GUARDED: &[&str] = &[
    "desktop",
    "documents",
    "downloads",
    "pictures",
    "movies",
    "music",
    "library/mobile documents",
    "library/mail",
    "library/messages",
    "library/safari",
    "library/calendars",
    "library/application support/addressbook",
    "library/keychains",
    ".ssh",
    ".aws",
    ".gnupg",
    ".netrc",
    ".config/gh",
];

/// Whether a read of `word`, a path [`reaches_outside`] named, is still refused: a folder
/// [`GUARDED`] names, another volume, or a search (`walks`) from the root, `/Users` or the
/// home, which reaches every one of them. **Every other read outside the slots runs** (the
/// owner, 16 Oct 2026: "reads are fine, writes are the ones we need to be careful of").
pub(in crate::helm) fn read_refused(word: &str, walks: bool) -> bool {
    let word = word.rsplit('=').next().unwrap_or(word);
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    if word.to_lowercase().contains("armada") {
        return false;
    }
    if word == "/Volumes" || word.starts_with("/Volumes/") {
        return true;
    }
    let under = ["~", "$HOME", "${HOME}"]
        .iter()
        .find_map(|home| word.strip_prefix(home))
        .map(str::to_string)
        .or_else(|| {
            let rest = word.strip_prefix("/Users/")?;
            Some(rest.find('/').map_or(String::new(), |at| rest[at..].to_string()))
        });
    let Some(under) = under else {
        return walks && matches!(word.trim_end_matches('/'), "" | "/Users");
    };
    let under = under.trim_matches('/').to_lowercase();
    if under.split('/').any(|part| part == "..") {
        return true;
    }
    (walks && under.is_empty())
        || GUARDED
            .iter()
            .any(|guarded| under == *guarded || under.starts_with(&format!("{guarded}/")))
}

/// The directories of `held` that are slots. Before its first lease a session runs in
/// the repository's root, the owner's own checkout, and nothing is taken away
/// there unasked.
pub(super) fn slots_of<'a>(held: &[&'a str]) -> Vec<&'a str> {
    held.iter()
        .copied()
        .filter(|directory| directory.contains("/.armada/slots/"))
        .collect()
}

/// Whether `word`, a path a shell line removes or overwrites, is inside
/// one of `slots`. **Removing a file in a slot it holds is the session's own work**
/// (the owner, 8 Oct 2026); its root, a `..`, a `~` or a `$` is never read as
/// inside, and a relative path only where the line has not moved with `cd`.
pub(super) fn in_the_slot(word: &str, slots: &[&str], relative: bool) -> bool {
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    // `$TMPDIR` is scratch space spelled by variable, and the one `$` read as inside.
    if let Some(under) = word.strip_prefix("$TMPDIR/").or_else(|| word.strip_prefix("${TMPDIR}/")) {
        return !under.is_empty() && !under.contains(['$', '~']) && !under.split('/').any(|part| part == "..");
    }
    if word.is_empty() || word.contains('$') || word.starts_with('~') {
        return false;
    }
    // Scratch space is no one's work: a patch kept in `/tmp` between two
    // steps asked, on every try (8 Oct 2026).
    if in_scratch(word) {
        return true;
    }
    let below = |slot: &&str| {
        inside_the_directory(word, slot) && resolved(word, slot) != resolved(slot, "/")
    };
    if word.starts_with('/') {
        return slots.iter().any(below);
    }
    relative && !matches!(word.trim_end_matches('/'), "" | "." | "*") && slots.iter().any(below)
}

/// Whether a `cd` to `to` lands in one of `slots`, its root included: unlike a
/// removal, standing at the root of a slot is the slot's own.
pub(super) fn cd_stays(to: &str, slots: &[&str]) -> bool {
    let to = to.trim_matches(|c| c == '\'' || c == '"');
    !to.is_empty()
        && !to.contains(['$', '~'])
        && slots.iter().any(|slot| inside_the_directory(to, slot))
}

/// Where a session may write and remove besides its slot.
pub(super) const SCRATCH: &[&str] = &[
    "/tmp/",
    "/private/tmp/",
    "/var/folders/",
    "/private/var/folders/",
];

/// Whether `path` is under `directory`, a relative path being read against it.
/// **Read where it lands**: a `..` and a symlink are followed first, so
/// `/tmp/../etc/hosts` is not scratch and `slot/a/../b` is the slot's own.
pub(super) fn inside_the_directory(path: &str, directory: &str) -> bool {
    match (resolved(path, directory), resolved(directory, "/")) {
        (Some(path), Some(directory)) => path.starts_with(directory),
        _ => false,
    }
}

/// Whether `path` is an absolute path inside the system's scratch folders.
pub(super) fn in_scratch(path: &str) -> bool {
    path.starts_with('/')
        && resolved(path, "/").is_some_and(|real| {
            let real = real.to_string_lossy();
            SCRATCH.iter().any(|scratch| real.starts_with(scratch))
        })
}

/// Where `path` lands, a relative one read against `base`: every `.` and `..`
/// folded away and every symlink that exists followed. The part of the path
/// that does not exist yet is read as written, so a write that creates a file
/// is placed by its directory. `None` where a `..` climbs past the root.
pub(super) fn resolved(path: &str, base: &str) -> Option<std::path::PathBuf> {
    let whole = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{base}/{path}")
    };
    let mut at = std::path::PathBuf::from("/");
    let mut missing = false;
    for part in whole.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if !at.pop() {
                    return None;
                }
                missing = false;
            }
            _ => {
                at.push(part);
                if !missing {
                    match std::fs::canonicalize(&at) {
                        Ok(real) => at = real,
                        Err(_) => missing = true,
                    }
                }
            }
        }
    }
    Some(at)
}

/// A shell line with its heredoc bodies cut out and its substitutions turned
/// into segments, so the commands inside them are read like any other.
/// `eval` and `exec` are still read where they stand.
pub(super) fn flattened(line: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut ending: Option<String> = None;
    for each in line.lines() {
        if let Some(tag) = &ending {
            if each.trim() == tag {
                ending = None;
            }
            continue;
        }
        kept.push(each);
        if let Some((_, after)) = each.split_once("<<") {
            let tag: String = after
                .trim_start_matches('-')
                .trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !tag.is_empty() {
                ending = Some(tag);
            }
        }
    }
    let kept = kept.join("\n");
    if kept.contains("$(") || kept.contains('`') {
        return kept.replace("$(", ";").replace(['`', ')'], ";");
    }
    kept
}
