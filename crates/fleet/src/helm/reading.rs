//! What a shell segment names and does, read by [`super::deciding`]: the variables a line set,
//! the files in the owner's home a read may touch, and the verbs that only look.

use ipc::CallKind;

use super::deciding::{reaches_outside, read_refused, truncating_redirect, Slot};

/// The segments with each variable the line assigned put back where it is used. `$$` is a
/// process id and reads as a number. **A variable the line never set stays a `$`**, which no
/// path check reads as inside anything, so `> $UNSET` is still refused. The last assignment wins,
/// as it does in the shell.
pub(super) fn with_variables(segments: Vec<&str>) -> Vec<String> {
    let mut set: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut out = Vec::new();
    for said in segments {
        let said = said.replace("$$", "1");
        let said = substituted(&said, &set);
        let words: Vec<&str> = said.split_whitespace().collect();
        let assignments = words.iter().take_while(|word| assignment(word).is_some()).count();
        if assignments == words.len() {
            for word in &words {
                if let Some((name, value)) = assignment(word) {
                    let value = value.trim_matches(|c| c == '\'' || c == '"');
                    set.insert(name.to_string(), value.to_string());
                }
            }
        }
        out.push(said);
    }
    out
}

/// `NAME=value`, where `NAME` is a shell identifier.
fn assignment(word: &str) -> Option<(&str, &str)> {
    let (name, value) = word.split_once('=')?;
    let mut chars = name.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic() || first == '_')
        .then_some(())
        .filter(|_| chars.all(|c| c.is_ascii_alphanumeric() || c == '_'))?;
    Some((name, value))
}

/// `said` with each `$name` and `${name}` that `set` holds replaced by its value.
fn substituted(said: &str, set: &std::collections::HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = said;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let (name, used) = match after.strip_prefix('{') {
            Some(braced) => match braced.split_once('}') {
                Some((name, _)) => (name, name.len() + 2),
                None => ("", 0),
            },
            None => {
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                let used = name.len();
                match set.get(&name) {
                    Some(value) => {
                        out.push_str(value);
                        rest = &after[used..];
                        continue;
                    }
                    None => ("", 0),
                }
            }
        };
        match set.get(name) {
            Some(value) if used > 0 => {
                out.push_str(value);
                rest = &after[used..];
            }
            _ => {
                out.push('$');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether one segment names the owner's own files and is refused for it: anything but a read,
/// and a read only where [`read_refused`] says so. **A `~` inside quotes and a `$` inside single
/// quotes are text**, as the shell leaves them: an awk script's `$5 ~ /x$/` is no path. The words
/// refused, in the order the segment names them, each with whether it is the script the segment
/// runs; empty where it runs.
pub(super) fn reaches_into_owners_files(said: &str) -> Vec<(String, bool)> {
    let said = literal_text(said).replace("\\ ", "\u{0}");
    // A quoted path is one word: `"~/Library/Application Support/x"` is no `~/Library/Application`.
    let words = words_of(&said);
    let script = script_run(&words);
    let sets = settings(&words);
    let named: Vec<(String, bool)> = words
        .into_iter()
        .enumerate()
        .filter(|(at, word)| *at >= sets && reaches_outside(word))
        .map(|(at, word)| (word.replace('\u{0}', " "), Some(at) == script))
        .collect();
    if named.is_empty() {
        return named;
    }
    let walks = only_reads(&said).then(|| walks(&said));
    // **Running a script is reading it** (the owner, 19 Oct 2026): it asks only where a read would.
    named
        .into_iter()
        .filter(|(word, runs)| match (walks, *runs) {
            (Some(walks), _) => read_refused(word, walks),
            (None, true) => read_refused(word, false),
            (None, false) => true,
        })
        .collect()
}

/// How many of a segment's first words only set a variable: `NAME=value` before the program, or
/// after `export`. **Setting one touches no file** (22 Oct 2026): `export PATH="$HOME/x:$PATH"`
/// asked, as a write, on every line that put rbenv on the path. Where the value is used, the
/// segment that uses it is read with it filled in.
fn settings(words: &[String]) -> usize {
    let declares = words.first().is_some_and(|word| DECLARES.contains(&word.as_str()));
    let skip = usize::from(declares);
    skip + words[skip..].iter().take_while(|word| assignment(word).is_some()).count()
}

/// What sets every assignment after it rather than running a program with it.
const DECLARES: &[&str] = &["export", "local", "readonly", "declare", "typeset"];

/// Which of a segment's words is the script it runs: its program, the operand handed to
/// `bash`, `sh` or `zsh`, or what `source` and `.` read.
fn script_run(words: &[String]) -> Option<usize> {
    let mut rest = words
        .iter()
        .enumerate()
        .skip_while(|(_, word)| assignment(word).is_some() || KEYWORDS.contains(&word.as_str()));
    let (at, program) = rest.next()?;
    let program = program.trim_start_matches('(');
    match program.rsplit('/').next().unwrap_or(program) {
        "source" | "." => rest.next().map(|(at, _)| at),
        "bash" | "sh" | "zsh" => rest.find(|(_, word)| !word.starts_with('-')).map(|(at, _)| at),
        _ => Some(at),
    }
}

/// The kind of call a segment is: the program it runs and, for an interpreter, the first operand
/// that is not a flag, both by basename. **What "Allow calls like this" matches**, so the same
/// script run with other arguments is the same kind.
pub(super) fn kind_of(said: &str) -> Option<CallKind> {
    const INTERPRETERS: &[&str] = &["node", "deno", "bun", "python", "python3", "ruby", "perl", "bash", "sh", "zsh"];
    let (program, arguments) = command_of(said)?;
    let mut operands = arguments
        .iter()
        .map(|word| word.trim_matches(|c| c == '\'' || c == '"'))
        .filter(|word| !word.is_empty() && !word.starts_with('-'));
    let script = match INTERPRETERS.contains(&program) {
        true => match (program, operands.next()) {
            // `deno run x.ts` and `bun run x.ts` name the script after the subcommand.
            ("deno" | "bun", Some("run")) => operands.next(),
            (_, first) => first,
        },
        false => None,
    };
    let script = script.map(|script| script.rsplit('/').next().unwrap_or(script).to_string());
    Some(CallKind { program: program.to_string(), script })
}

/// `said` cut at blanks outside quotes, each quoted run kept with its quotes.
fn words_of(said: &str) -> Vec<String> {
    let (mut words, mut word, mut quote) = (Vec::new(), String::new(), None::<char>);
    for c in said.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                continue;
            }
            _ => {}
        }
        word.push(c);
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// Whether a reading segment searches below the paths it names, rather than looking at them.
fn walks(said: &str) -> bool {
    let Some((program, arguments)) = command_of(said) else {
        return false;
    };
    let recursive = arguments.iter().any(|word| {
        *word == "--recursive"
            || (word.starts_with('-') && !word.starts_with("--") && word.contains(['r', 'R']))
    });
    match program {
        "find" | "du" | "tree" | "rg" => true,
        "grep" | "egrep" | "fgrep" | "ls" => recursive,
        _ => false,
    }
}

/// `line` with each `~` inside quotes and each `$` inside single quotes turned to `_`.
fn literal_text(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    line.chars()
        .map(|c| {
            if escaped {
                escaped = false;
                return c;
            }
            match quote {
                Some(q) if c == q => {
                    quote = None;
                    c
                }
                Some('"') if c == '\\' => {
                    escaped = true;
                    c
                }
                Some(_) if c == '~' => '_',
                Some('\'') if c == '$' => '_',
                Some(_) => c,
                None if c == '\\' => {
                    escaped = true;
                    c
                }
                None if c == '\'' || c == '"' => {
                    quote = Some(c);
                    c
                }
                None => c,
            }
        })
        .collect()
}

/// [`only_reads`] for a segment as it was written, read as [`reaches_into_owners_files`] reads it.
pub(super) fn segment_only_reads(said: &str) -> bool {
    only_reads(&literal_text(said).replace("\\ ", "\u{0}"))
}

/// Whether a segment only reads: a program that reads, and no redirect into a file.
fn only_reads(said: &str) -> bool {
    const READERS: &[&str] = &[
        "cat", "ls", "head", "tail", "wc", "grep", "egrep", "fgrep", "rg", "stat", "file", "find",
        "du", "tree", "realpath", "readlink", "diff", "cmp", "command", "which", "type", "jq",
        "strings", "cut", "nl", "shasum", "md5", "xxd", "od", "mdls", "cd", "pushd",
    ];
    if truncating_redirect(said) || said.contains(">>") {
        return false;
    }
    // `F=~/x` alone sets a name, which the segments after it are read with.
    let Some((program, arguments)) = command_of(said) else {
        return said.split_whitespace().all(|word| assignment(word).is_some());
    };
    // `find` writes with these, and runs anything with the others.
    let find_acts = program == "find"
        && arguments.iter().any(|word| {
            matches!(*word, "-delete" | "-exec" | "-execdir" | "-ok" | "-okdir")
                || word.starts_with("-fprint")
                || *word == "-fls"
        });
    // `sed` prints unless it edits in place (`-i`, `-ni`, `-i.bak`, `--in-place`).
    let sed_edits = program == "sed"
        && arguments.iter().any(|word| {
            word.starts_with("--in-place")
                || (word.starts_with('-') && !word.starts_with("--") && word[1..].contains('i'))
        });
    // `sqlite3 -readonly` opens a database it cannot change.
    let reads_a_database = program == "sqlite3" && arguments.contains(&"-readonly");
    (READERS.contains(&program) && !find_acts) || (program == "sed" && !sed_edits) || reads_a_database
}

/// [`writes`] for a segment as it was written, read as [`reaches_into_owners_files`] reads it.
pub(super) fn segment_writes(said: &str) -> bool {
    writes(&literal_text(said).replace("\\ ", "\u{0}"))
}

/// Whether a segment is known to change files: a redirect into one, a program that writes or
/// removes what it is handed, or an editor told to edit in place. **What is neither this nor
/// [`only_reads`] asks to run**, never to write (the owner, 30 Oct 2026).
fn writes(said: &str) -> bool {
    const WRITERS: &[&str] = &[
        "cp", "mv", "install", "ln", "tee", "touch", "rsync", "dd", "truncate", "ditto", "rm", "rmdir",
        "unlink", "shred", "mkdir", "chmod", "chown",
    ];
    if truncating_redirect(said) || said.contains(">>") {
        return true;
    }
    let Some((program, arguments)) = command_of(said) else {
        return false;
    };
    let in_place = arguments.iter().any(|word| {
        word.starts_with("--in-place") || (word.starts_with('-') && !word.starts_with("--") && word[1..].contains('i'))
    });
    let find_acts = program == "find"
        && arguments.iter().any(|word| {
            matches!(*word, "-delete" | "-exec" | "-execdir" | "-ok" | "-okdir")
                || word.starts_with("-fprint")
                || *word == "-fls"
        });
    WRITERS.contains(&program) || (matches!(program, "sed" | "perl") && in_place) || find_acts
}

/// The words in front of a segment's program: a leading `env` or `command`, and the keywords that
/// open a clause.
const KEYWORDS: &[&str] = &[
    "env", "command", "do", "then", "else", "elif", "if", "while", "until", "time", "!", "{", "(",
];

/// The program a segment runs, and its arguments: the first word that is not an assignment, a
/// leading `env` or `command`, or the keyword that opens a clause (`do rm x` is an `rm`).
pub(super) fn command_of(said: &str) -> Option<(&str, Vec<&str>)> {
    let mut rest = words_in(said)
        .into_iter()
        .skip_while(|word| assignment(word).is_some() || KEYWORDS.contains(word))
        .peekable();
    let program = rest.next()?;
    // A subshell's `(` is not the program: `(cd x && ls)` runs `cd`.
    let program = program.trim_start_matches('(');
    let program = program.rsplit('/').next().unwrap_or(program);
    Some((program, rest.collect()))
}

/// [`words_of`], each word a slice of `said`. **A quoted path with a space is one operand**: a
/// chmod of `"…/Application Support/…"` was read as two, and refused (27 Oct 2026).
fn words_in(said: &str) -> Vec<&str> {
    let (mut words, mut from, mut quote) = (Vec::new(), None::<usize>, None::<char>);
    for (at, c) in said.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c.is_whitespace() => {
                if let Some(start) = from.take() {
                    words.push(&said[start..at]);
                }
                continue;
            }
            _ => {}
        }
        from.get_or_insert(at);
    }
    if let Some(start) = from {
        words.push(&said[start..]);
    }
    words
}

/// `chmod` and `chown` naming only files in a slot the session holds, as `rm` does. The first
/// operand is the mode or the owner, and is left out only where it cannot be a path.
pub(super) fn changes_only_held(program: &str, arguments: &[&str], slot: Option<Slot>) -> bool {
    let mut operands: Vec<&str> = arguments
        .iter()
        .copied()
        .filter(|word| !word.starts_with('-'))
        .collect();
    let mode_like = |word: &str| {
        !word.contains('/')
            && word
                .chars()
                .all(|c| c.is_ascii_digit() || "ugoarwxXst+-=,".contains(c))
    };
    if operands.len() >= 2 {
        let first = operands[0];
        let spec = match program {
            "chmod" => mode_like(first),
            _ => !first.contains('/'),
        };
        if spec {
            operands.remove(0);
        }
    }
    slot.is_some_and(|slot| !operands.is_empty() && operands.iter().all(|word| slot.holds(word)))
}

/// A program asked a question that changes nothing: the verbs below read the service manager or
/// the disks. Every other verb of these programs is as destructive as the program is.
pub(super) fn queries(program: &str, arguments: &[&str]) -> bool {
    let verbs: &[&str] = match program {
        "launchctl" => &[
            "print", "list", "blame", "print-cache", "print-disabled", "help", "version",
            "managerpid", "manageruid", "managername", "error",
        ],
        "diskutil" => &["list", "info", "activity"],
        "systemctl" => &[
            "status", "show", "cat", "list-units", "list-unit-files", "is-active", "is-enabled",
        ],
        _ => return false,
    };
    arguments
        .iter()
        .find(|word| !word.starts_with('-'))
        .is_some_and(|verb| verbs.contains(verb))
}

