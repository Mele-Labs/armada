//! The program that shows a Kit piece's file: the person's editor, or Finder.
//!
//! **Argv only, never a shell.** The program is argv[0] and the path is the last argument, or
//! wherever `{file}` stands, so a setting with `;` in it names a program that will not be found
//! rather than a second command. The value splits on whitespace and quotes are ordinary characters.
//!
//! | order | source |
//! |---|---|
//! | 1 | `editor.command` in settings.json |
//! | 2 | `$VISUAL`, then `$EDITOR` |
//! | 3 | the system's own opener |
//!
//! **A named editor that is not there is refused** rather than falling through to the opener, which
//! would open the file and make the setting look ignored.

use std::fmt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The system's opener, by its absolute path: a `PATH` Fleet was started with may not hold it.
const OPENER: &str = "/usr/bin/open";

/// Where an editor was read from, which a refusal says so the person edits the right line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    Setting,
    Visual,
    Editor,
    Opener,
}

impl fmt::Display for Source {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Source::Setting => "editor.command",
            Source::Visual => "$VISUAL",
            Source::Editor => "$EDITOR",
            Source::Opener => "the system opener",
        })
    }
}

/// The program that opens a path, as it was written, and the arguments written before the path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Editor {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) from: Source,
}

/// A program and what it is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Launch {
    pub(crate) program: PathBuf,
    pub(crate) argv: Vec<String>,
}

/// Whitespace only is unset. An exported empty string is not a decision.
fn said(value: &str) -> Option<Vec<String>> {
    let words: Vec<String> = value.split_whitespace().map(str::to_string).collect();
    (!words.is_empty()).then_some(words)
}

fn editor_of(words: Vec<String>, from: Source) -> Editor {
    let mut words = words.into_iter();
    // `said` answers only a non-empty list.
    Editor { command: words.next().unwrap_or_default(), args: words.collect(), from }
}

/// The editor to use. `env` reads one variable. `$VISUAL` goes before `$EDITOR`, which may name a
/// line editor for a dumb terminal.
pub(crate) fn choose(env: impl Fn(&str) -> Option<String>, configured: &str) -> Editor {
    if let Some(words) = said(configured) {
        return editor_of(words, Source::Setting);
    }
    for (name, from) in [("VISUAL", Source::Visual), ("EDITOR", Source::Editor)] {
        if let Some(words) = env(name).as_deref().and_then(said) {
            return editor_of(words, from);
        }
    }
    Editor { command: OPENER.to_string(), args: Vec::new(), from: Source::Opener }
}

/// The arguments with `{file}` and `{line}` filled in, each inside the word it was written in. No
/// `{file}` anywhere puts the path last, which is what `$EDITOR` has always meant.
fn with_file(args: &[String], path: &str) -> Vec<String> {
    let mut filled: Vec<String> =
        args.iter().map(|word| word.replace("{file}", path).replace("{line}", "1")).collect();
    if !args.iter().any(|word| word.contains("{file}")) {
        filled.push(path.to_string());
    }
    filled
}

fn executable(candidate: &Path) -> bool {
    std::fs::metadata(candidate).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// Where that program is. A command with a `/` is a path and is taken as one, absolute only, since
/// a relative one would resolve against wherever Fleet started. Otherwise `path_var` is walked in order.
fn where_is(command: &str, path_var: &str) -> Option<PathBuf> {
    if command.contains('/') {
        return (command.starts_with('/') && executable(Path::new(command))).then(|| PathBuf::from(command));
    }
    path_var
        .split(':')
        .filter(|directory| !directory.is_empty())
        .map(|directory| Path::new(directory).join(command))
        .find(|candidate| executable(candidate))
}

/// Why a named editor could not be used, in the voice a row reads it in.
fn not_found(editor: &Editor) -> String {
    let Editor { command, from, .. } = editor;
    if !command.contains('/') {
        format!("{from} names {command}, and nothing on PATH is called that.")
    } else if command.starts_with('/') {
        format!("{from} names {command}, and nothing executable is there.")
    } else {
        format!("{from} names {command}, which is relative. An editor is named by an absolute path or a name on PATH.")
    }
}

/// What starts the editor on `path`, or why none can.
pub(crate) fn in_editor(
    configured: &str,
    env: impl Fn(&str) -> Option<String>,
    path_var: &str,
    path: &str,
) -> Result<Launch, String> {
    let editor = choose(env, configured);
    let program = where_is(&editor.command, path_var).ok_or_else(|| not_found(&editor))?;
    Ok(Launch { program, argv: with_file(&editor.args, path) })
}

/// What shows `path` in Finder.
pub(crate) fn in_finder(path: &str) -> Launch {
    Launch { program: PathBuf::from(OPENER), argv: vec!["-R".to_string(), path.to_string()] }
}

/// Start it, detached. **No stdio**, because an editor outlives the press that opened it and a pipe
/// nobody reads blocks its child, and **a process group of its own**, so stopping Fleet does not
/// take the editor with it. A failure after the start is the editor's to report in its own window.
pub(crate) fn start(launch: &Launch) -> Result<(), String> {
    let mut child = Command::new(&launch.program)
        .args(&launch.argv)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|why| format!("{} would not start: {why}", launch.program.display()))?;
    // Reaped on a thread of its own, so a finished editor is not left a zombie for as long as Fleet runs.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
