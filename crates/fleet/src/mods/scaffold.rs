//! Making a mod: the folder, its two files, and a git repository with one commit.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use ipc::ModKind;

use super::{manifest, slug_problem, FLEET, LAYOUT, MANIFEST, THEME, TRACKER, TWEAK, WORDS, COMMANDS};

/// Why a scaffold did not happen.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// The request names something that cannot be a mod. A 422.
    Unacceptable(String),
    /// Something under it failed. Nothing is left behind.
    Failed(String),
}

/// The starter stylesheet: the shipped dark values for the tokens most worth
/// changing, so it validates and draws as before until someone edits it.
const STARTER: &str = "\
/* Token values only, set in :root. Check it with validate_mod. */
:root {
  --bg-base: #0F1419;
  --bg-raised: #161C23;
  --fg-default: #E4E9EF;
  --accent: #4A9EDB;
}
";

/// The starter layout: the version and no region, so it validates and changes nothing until
/// someone names a region.
const STARTER_LAYOUT: &str = "{\n  \"version\": 1\n}\n";

/// The starter Words file: a pattern and no address, so it links nothing until a repository has one.
const STARTER_WORDS: &str = "\
# A pattern Bridge links in a Session, and where it opens in each repository.
name = \"Example key\"
pattern = \"EX-[0-9]+\"
example = \"EX-1\"

# [[address]]
# repository = \"a-manifest-id\"
# template = \"https://example.com/browse/{match}\"
# found = \"person\"
";

/// The starter Fleet file: no rule, so every notice goes as it always has.
const STARTER_FLEET: &str = "\
# What Fleet does with a notice: tell, quiet, batch or call, for a job, a repo or everywhere.
# [[rule]]
# notice = \"behind_base\"
# does = \"batch\"
# where = \"everywhere\"
";

/// The starter Commands file: one row that passes, since a file with none is refused, and the other three acts commented.
const STARTER_COMMANDS: &str = "\
# Rows added to Bridge's command palette. Each row does exactly one of prompt, url, shell or chain.
# A {name} in a prompt, url or shell line is a blank the person fills in the palette.
# aliases are searched and never drawn. hotkey is a chord as the registry spells one: ⌥⌘S.

# Opens an address.
[[command]]
name = \"Open the docs\"
url = \"https://example.com/docs\"

# Starts a Job on a prompt.
# [[command]]
# name = \"Review a pull request for security\"
# aliases = [\"sec\"]
# hotkey = \"⌥⌘S\"
# prompt = \"Review {pr} for security issues\"

# Runs a line in the picked repository's checkout.
# [[command]]
# name = \"Audit dependencies\"
# shell = \"pnpm audit\"

# Runs other palette rows by id, in order.
# [[command]]
# name = \"Add a task\"
# chain = [\"nav:projects\", \"act:add_task\"]
";

/// The starter tweak: no declaration, so the chosen theme draws as it did.
const STARTER_TWEAK: &str = "/* Token values only, set in :root. Check it with validate_mod. */\n:root {\n}\n";

/// What a teaching sets on a scaffold: the file's text in place of the starter, and where the mod
/// came from.
pub(crate) struct Taught<'a> {
    pub(crate) text: &'a str,
    pub(crate) taught_on: Option<&'a str>,
    pub(crate) entry: Option<&'a str>,
}
/// The starter tracker: every section a tracker needs, over a command line that is not there, so
/// it passes the file's checks and fails its try until someone names the real one.
const STARTER_TRACKER: &str = r#"# How Armada lists, searches and reads this tracker. Check it with validate_mod, then
# try it with try_tracker_mod against a repository before a Manifest names it.

[guide]
language = "The tracker's query language"
guide = """
How a query is written for this tracker, with examples, and what it cannot ask.
"""
open = "state:open"
assigned_to_me = "state:open assignee:me"
fields = []

[search]
transport = "cli"
argv = ["tracker-cli", "search", "--json", "{query}"]
map = """
map({reference: .id, title: .title, url: .url, closed: (.state == "closed")})
"""

[read]
transport = "cli"
argv = ["tracker-cli", "show", "--json", "{reference}"]
map = """
{reference: .id, title: .title, url: .url, closed: (.state == "closed"), body: .body}
"""
"#;

/// Make `mods_dir/name` and commit it. **The folder is made with `create_dir`**,
/// which refuses one that exists in the same step it makes this one, so a
/// concurrent scaffold of the same name cannot get into the other's folder. `made_by`, the Job that
/// asked, is recorded in `mod.toml`.
pub(crate) fn make(
    mods_dir: &Path,
    name: &str,
    description: Option<&str>,
    kind: ModKind,
    made_by: Option<&str>,
) -> Result<PathBuf, Refused> {
    made(mods_dir, name, description, kind, None, made_by)
}

/// [`make`], with the file a teaching wrote where there would be a starter.
pub(crate) fn make_taught(
    mods_dir: &Path,
    name: &str,
    description: Option<&str>,
    kind: ModKind,
    taught: Option<&Taught>,
) -> Result<PathBuf, Refused> {
    made(mods_dir, name, description, kind, taught, None)
}

fn made(
    mods_dir: &Path,
    name: &str,
    description: Option<&str>,
    kind: ModKind,
    taught: Option<&Taught>,
    made_by: Option<&str>,
) -> Result<PathBuf, Refused> {
    if let Some(why) = slug_problem(name) {
        return Err(Refused::Unacceptable(why));
    }
    let description = description.map(str::trim).unwrap_or_default();
    if description.chars().count() > manifest::MOST_DESCRIPTION
        || description.chars().any(char::is_control)
    {
        return Err(Refused::Unacceptable(format!(
            "a description is at most {} characters on one line",
            manifest::MOST_DESCRIPTION
        )));
    }
    if let Some(on) = taught.and_then(|one| one.taught_on) {
        if on.chars().count() > manifest::MOST_TAUGHT_ON || on.chars().any(char::is_control) {
            return Err(Refused::Unacceptable(format!(
                "what Teach was pressed on is at most {} characters on one line",
                manifest::MOST_TAUGHT_ON
            )));
        }
    }
    std::fs::create_dir_all(mods_dir)
        .map_err(|why| Refused::Failed(format!("the mods folder could not be made: {why}")))?;
    let dir = mods_dir.join(name);
    std::fs::create_dir(&dir).map_err(|why| match why.kind() {
        std::io::ErrorKind::AlreadyExists => Refused::Unacceptable(format!("a mod called `{name}` already exists")),
        _ => Refused::Failed(format!("the mod's folder could not be made: {why}")),
    })?;
    match written(&dir, name, description, kind, taught, made_by) {
        Ok(()) => Ok(dir),
        Err(why) => {
            // Ours, made a moment ago and holding only what this wrote.
            let _ = std::fs::remove_dir_all(&dir);
            Err(Refused::Failed(why))
        }
    }
}

fn written(
    dir: &Path,
    name: &str,
    description: &str,
    kind: ModKind,
    taught: Option<&Taught>,
    made_by: Option<&str>,
) -> Result<(), String> {
    let escape = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
    let (word, file, starter) = match kind {
        ModKind::Theme => ("theme", THEME, STARTER),
        ModKind::Layout => ("layout", LAYOUT, STARTER_LAYOUT),
        ModKind::Words => ("words", WORDS, STARTER_WORDS),
        ModKind::Fleet => ("fleet", FLEET, STARTER_FLEET),
        ModKind::Tweak => ("tweak", TWEAK, STARTER_TWEAK),
        ModKind::Tracker => ("tracker", TRACKER, STARTER_TRACKER),
        ModKind::Commands => ("commands", COMMANDS, STARTER_COMMANDS),
    };
    let mut toml = format!(
        "name = \"{name}\"\nkind = \"{word}\"\nversion = \"0.1.0\"\ndescription = \"{}\"\n",
        escape(description)
    );
    if let Some(on) = taught.and_then(|one| one.taught_on) {
        toml.push_str(&format!("taught_on = \"{}\"\n", escape(on)));
    }
    if let Some(entry) = taught.and_then(|one| one.entry) {
        toml.push_str(&format!("entry = \"{}\"\n", escape(entry)));
    }
    if let Some(job) = made_by {
        toml.push_str(&format!("made_by = \"{}\"\n", escape(job)));
    }
    let starter = taught.map_or(starter, |one| one.text);
    for (file, text) in [(MANIFEST, toml.as_str()), (file, starter)] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(file))
            .and_then(|mut made| made.write_all(text.as_bytes()))
            .map_err(|why| format!("{file} could not be written: {why}"))?;
    }
    git(dir, &["init", "--quiet", "--initial-branch=main"])?;
    git(dir, &["add", "--", MANIFEST, file])?;
    git(dir, &["commit", "--quiet", "-m", &format!("Start the {name} {word}")])
}

/// `file` of the mod in `dir`, committed as it stands. **Best effort**: the file is what counts, and
/// a history that would not take the commit is no reason to refuse the change.
pub(crate) fn committed(dir: &Path, file: &str, message: &str) -> Result<(), String> {
    git(dir, &["add", "--", file])?;
    git(dir, &["commit", "--quiet", "-m", message])
}

/// One git command in `dir`, with no configuration but what is passed here.
///
/// **The user's own git configuration is turned off**, so a global hook, a
/// signing requirement or a template cannot decide whether a scaffold works,
/// and the identity is Armada's rather than whatever the owner has set.
fn git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Armada",
            "-c",
            "user.email=armada@localhost",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|why| format!("git could not be run: {why}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "git {} failed: {}",
        args.first().copied().unwrap_or_default(),
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}
