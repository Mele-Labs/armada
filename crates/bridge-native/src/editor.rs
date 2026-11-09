//! Which editor opens a path, and how.
//!
//! Mirrors `apps/desktop/src/main/editor.ts`, with `open.ts` as its Electron caller: the first
//! native screen that opens a file mirrors that caller, reading `editor.command` from Fleet's
//! settings ([`configured_from`], `GET /settings`) and calling [`open`]. **Nothing calls it yet.**
//!
//! | order | source | the question it answers |
//! |---|---|---|
//! | 1 | `editor.command` in settings.json | what a person decided, and changes from Bridge |
//! | 2 | `$VISUAL`, then `$EDITOR` | what a developer already told every other tool |
//! | 3 | `open` | what happens when nobody has said anything |
//!
//! **Argv only, never a shell.** The program is argv[0] and the path is the last argument, or
//! wherever `{file}` stands, and nothing is concatenated into a string a shell would parse. A value
//! with `;` or a backtick in it is a program name containing those characters, which will not be
//! found, rather than a second command.
//!
//! **Quotes are not interpreted.** The value splits on whitespace, so `code -w` is the program
//! `code` and the flag `-w` and a quote is an ordinary character. Interpreting them is what a
//! shell does.
//!
//! A Dock launch inherits launchd's environment, which does not source a login shell: `$VISUAL`
//! and `$EDITOR` set in `.zshrc` are not there, and tier 2 is skipped silently, as an unset one is.

use std::fmt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ipc::{SettingValue, SettingsList};

use crate::transport;

/// The setting this reads, a key of `config::settings`' table.
const SETTING: &str = "editor.command";

/// Where an editor was read from. Carried because a failure has to say where the value came from:
/// *"`$EDITOR` names `code`"* sends a person to the right line of the right file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Setting,
    Visual,
    Editor,
    /// Nobody said anything: macOS's own `open`.
    Open,
}

impl fmt::Display for Source {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Source::Setting => "editor.command",
            Source::Visual => "$VISUAL",
            Source::Editor => "$EDITOR",
            Source::Open => "open",
        })
    }
}

/// The program that opens a path, as it was written, and the arguments written before the path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Editor {
    pub command: String,
    pub args: Vec<String>,
    pub from: Source,
}

/// Whitespace-only is unset. An exported empty string is not a decision.
fn said(value: &str) -> Option<Vec<String>> {
    let words: Vec<String> = value.split_whitespace().map(str::to_string).collect();
    (!words.is_empty()).then_some(words)
}

fn editor_of(words: Vec<String>, from: Source) -> Editor {
    let mut words = words.into_iter();
    // `said` answers only a non-empty list.
    Editor { command: words.next().unwrap_or_default(), args: words.collect(), from }
}

/// The editor to use. `env` reads one variable. `$VISUAL` before `$EDITOR` is the older convention and the
/// right way round for the reason it exists: `$EDITOR` may name a line editor for a dumb terminal, and
/// `$VISUAL` names the full-screen one, which is the kind a window asks for.
pub fn choose(env: impl Fn(&str) -> Option<String>, configured: &str) -> Editor {
    if let Some(words) = said(configured) {
        return editor_of(words, Source::Setting);
    }
    for (name, from) in [("VISUAL", Source::Visual), ("EDITOR", Source::Editor)] {
        if let Some(words) = env(name).as_deref().and_then(said) {
            return editor_of(words, from);
        }
    }
    Editor { command: "open".into(), args: Vec::new(), from: Source::Open }
}

/// The arguments with `{file}` and `{line}` filled in, each inside the word it was written in, so
/// `{file}:{line}` stays one argument. **No `{file}` anywhere puts the path last**, which is what
/// `$EDITOR` has always meant.
pub fn with_file(args: &[String], path: &str, line: u32) -> Vec<String> {
    let mut filled: Vec<String> =
        args.iter().map(|word| word.replace("{file}", path).replace("{line}", &line.to_string())).collect();
    if !args.iter().any(|word| word.contains("{file}")) {
        filled.push(path.to_string());
    }
    filled
}

fn executable(candidate: &Path) -> bool {
    std::fs::metadata(candidate).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// Where that program actually is. **Resolved before anything is started**, so a name that is not
/// there is a sentence for a person and not a failure after the call returned.
///
/// A command carrying a `/` is a path and is taken as one, absolute only: the app's working
/// directory is wherever launchd started it, and a relative path resolved against it means nothing
/// a person chose. Otherwise `PATH` is walked in order, which is the lookup the shell that set the
/// variable would have done.
pub fn where_is(command: &str, path_var: Option<&str>) -> Option<PathBuf> {
    if command.contains('/') {
        let file = PathBuf::from(command);
        return (command.starts_with('/') && executable(&file)).then_some(file);
    }
    path_var?
        .split(':')
        .filter(|directory| !directory.is_empty())
        .map(|directory| Path::new(directory).join(command))
        .find(|candidate| executable(candidate))
}

/// Why a named editor could not be used, in the voice a row reads it in. **A configured editor that
/// is not there says so rather than falling through to `open`**: the path would appear and the
/// setting would look ignored, which teaches a person the wrong thing about their own configuration.
pub fn not_found(editor: &Editor) -> String {
    let Editor { command, from, .. } = editor;
    if !command.contains('/') {
        format!("{from} names {command}, and nothing on PATH is called that.")
    } else if command.starts_with('/') {
        format!("{from} names {command}, and nothing executable is there.")
    } else {
        format!(
            "{from} names {command}, which is relative to nothing — an editor is named by an absolute path or by a name on PATH."
        )
    }
}

/// Start a program on an argv, detached, and stop caring about it. **No stdio**, because an editor
/// outlives the press that opened it and a pipe nobody reads blocks its child on a full buffer; and
/// **a process group of its own**, so launchd stopping this app's job does not take the editor with
/// it. Failures after the start are the editor's to report in its own window.
pub fn start_detached(program: &Path, argv: &[String]) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|why| format!("{} would not start: {why}", program.display()))?;
    // Reaped on a thread of its own, so a finished editor is not left a zombie for as long as the app runs.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// The editor command Fleet has in force: `editor.command`'s `value` in a `GET /settings` answer,
/// or empty where the list has no such key or it is not text.
pub fn configured(list: &SettingsList) -> String {
    match list.settings.iter().find(|one| one.key == SETTING).map(|one| &one.value) {
        Some(SettingValue::Text(command)) => command.clone(),
        _ => String::new(),
    }
}

/// Read `editor.command` from the Fleet on `port`. Awaited on a task, never on the main thread.
pub async fn configured_from(port: u16) -> Result<String, String> {
    match transport::get(port, "/settings").await? {
        (200, body) => {
            ipc::decode::<SettingsList>("settings", &body).map(|list| configured(&list)).map_err(|why| why.to_string())
        }
        (status, _) => Err(format!("Fleet answered {status}")),
    }
}

/// Open `path` in the editor `configured` and the environment name, at `line`. Err is why not, in a row's voice.
pub fn open(configured: &str, path: &str, line: u32) -> Result<(), String> {
    let editor = choose(|name| std::env::var(name).ok(), configured);
    let path_var = std::env::var("PATH").ok();
    let program = where_is(&editor.command, path_var.as_deref()).ok_or_else(|| not_found(&editor))?;
    start_detached(&program, &with_file(&editor.args, path, line))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| value.to_string())
    }

    fn words(of: &[&str]) -> Vec<String> {
        of.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn editor_command_wins_over_visual_and_editor() {
        let chosen = choose(env(&[("VISUAL", "vim"), ("EDITOR", "nano")]), "code -g {file}:{line}");
        assert_eq!(chosen, Editor { command: "code".into(), args: words(&["-g", "{file}:{line}"]), from: Source::Setting });
    }

    #[test]
    fn it_falls_to_visual_then_editor_where_the_setting_is_empty() {
        assert_eq!(choose(env(&[("VISUAL", "vim"), ("EDITOR", "nano")]), "  ").from, Source::Visual);
        assert_eq!(
            choose(env(&[("EDITOR", "code -w")]), ""),
            Editor { command: "code".into(), args: words(&["-w"]), from: Source::Editor }
        );
    }

    #[test]
    fn a_blank_variable_is_not_a_decision() {
        assert_eq!(choose(env(&[("VISUAL", " "), ("EDITOR", "nano")]), "").from, Source::Editor);
    }

    #[test]
    fn nobody_having_said_anything_is_open() {
        assert_eq!(choose(env(&[]), ""), Editor { command: "open".into(), args: Vec::new(), from: Source::Open });
    }

    #[test]
    fn quotes_are_ordinary_characters_and_the_value_splits_on_whitespace() {
        let chosen = choose(env(&[]), "\"my editor\" --flag");
        assert_eq!(chosen.command, "\"my");
        assert_eq!(chosen.args, words(&["editor\"", "--flag"]));
    }

    #[test]
    fn file_and_line_fill_inside_the_word_they_are_written_in() {
        assert_eq!(with_file(&words(&["-g", "{file}:{line}"]), "/repo/a.ts", 12), words(&["-g", "/repo/a.ts:12"]));
    }

    #[test]
    fn the_path_goes_last_where_nothing_names_file() {
        assert_eq!(with_file(&words(&["-w"]), "/repo/a.ts", 1), words(&["-w", "/repo/a.ts"]));
    }

    #[test]
    fn a_line_nobody_gave_is_the_first() {
        assert_eq!(with_file(&words(&["{file}:{line}"]), "/repo/a.ts", 1), words(&["/repo/a.ts:1"]));
    }

    #[test]
    fn a_path_with_spaces_and_shell_characters_stays_one_argument() {
        let path = "/repo/my file; rm -rf $HOME.ts";
        assert_eq!(with_file(&words(&["-g", "{file}:{line}"]), path, 3), vec!["-g".to_string(), format!("{path}:3")]);
        assert_eq!(with_file(&[], path, 1), vec![path.to_string()]);
    }

    #[test]
    fn a_relative_path_is_taken_for_nothing_and_an_absolute_one_must_be_executable() {
        assert_eq!(where_is("./code", Some("/usr/bin")), None);
        assert_eq!(where_is("bin/code", Some("/usr/bin")), None);
        assert_eq!(where_is("/usr/bin/true", None), Some(PathBuf::from("/usr/bin/true")));
        assert_eq!(where_is("/nowhere/at/all", None), None);
        // A file that is not executable is not an editor.
        assert_eq!(where_is("/etc/hosts", None), None);
    }

    #[test]
    fn a_name_is_looked_for_on_path_in_order_and_with_no_path_nowhere() {
        assert_eq!(where_is("true", Some("/nowhere::/usr/bin:/bin")), Some(PathBuf::from("/usr/bin/true")));
        assert_eq!(where_is("true", Some("/nowhere")), None);
        assert_eq!(where_is("true", None), None);
    }

    #[test]
    fn a_value_with_a_semicolon_is_a_program_name_and_is_not_found() {
        assert_eq!(where_is("true;", Some("/usr/bin")), None);
        assert_eq!(where_is("`true`", Some("/usr/bin")), None);
    }

    #[test]
    fn a_failure_names_where_the_value_came_from() {
        let named = |command: &str, from| Editor { command: command.into(), args: Vec::new(), from };
        assert_eq!(not_found(&named("code", Source::Editor)), "$EDITOR names code, and nothing on PATH is called that.");
        assert_eq!(
            not_found(&named("/opt/code", Source::Setting)),
            "editor.command names /opt/code, and nothing executable is there."
        );
        assert_eq!(
            not_found(&named("bin/code", Source::Visual)),
            "$VISUAL names bin/code, which is relative to nothing — an editor is named by an absolute path or by a name on PATH."
        );
    }

    #[test]
    fn the_setting_is_read_off_the_list_fleet_serves() {
        let list = |value: &str| {
            let wire = format!(
                r#"{{"path":"/s.json","refused":null,"settings":[{{"key":"editor.command","group":"Tools","section":"","title":"Editor","description":"","tip":null,"kind":{{"kind":"text"}},"default":"","value":{value},"saved":null,"applies":"live","pending_restart":false,"overridden_by_env":null}}]}}"#
            );
            ipc::decode::<SettingsList>("settings", wire.as_bytes()).expect("a settings list")
        };
        assert_eq!(configured(&list(r#""code -g {file}:{line}""#)), "code -g {file}:{line}");
        assert_eq!(configured(&list("null")), "");
        let none = ipc::decode::<SettingsList>("settings", br#"{"path":"/s.json","refused":null,"settings":[]}"#).expect("a list");
        assert_eq!(configured(&none), "");
    }
}
