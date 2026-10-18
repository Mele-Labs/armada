//! A shell line taken apart: its segments, its quotes and its redirects, each judged by `segment`.

use super::*;

/// A shell line.
///
/// **Every segment, never the first word.** `cd crates && rm -rf target` has
/// `cd` at the front of it, and a classifier reading one word would run it.
pub(super) fn shell(line: &str, slots: &[&str], own: Own) -> Option<Because> {
    match shell_guarded(line, slots, own, None)? {
        Guarded::Because { because, .. } => Some(because),
        Guarded::Blocked { .. } => Some(Because::ReachesOutside),
    }
}

/// [`shell`], and for a session (`folders`) whether a segment's words name the owner's own
/// files. **Read per segment, after the variables a line set are put back**, so `d=~/Music; ls $d`
/// is read as the `ls ~/Music` it is, and the `~` of an awk script is not read as a path at all.
pub(super) fn shell_guarded(line: &str, slots: &[&str], own: Own, folders: Option<Folders>) -> Option<Guarded> {
    let only = |because: Because| Some(Guarded::Because { because, reaching: None, to: None });
    let line = line.trim();
    if line.is_empty() {
        return only(Because::Unreadable);
    }
    if line.contains('`') || line.contains("$(") {
        return only(Because::Unreadable);
    }
    // Every class at once, and never worth reading further.
    if word_in(line, "sudo") || word_in(line, "doas") {
        return only(Because::Destructive);
    }
    // **`&&` and `||` become separators and a lone `&` does not.** Splitting on
    // every `&` cuts `2>&1` in half and leaves a segment ending in a bare `>`,
    // which then reads as a truncating redirect — `cargo build 2>&1 | tail -5`
    // asked, as a destructive line, until a case caught it.
    let quiet = unquoted(line);
    // Same length as the line, so a cut in one is a cut in the other.
    let chained = quiet.replace("&&", " ;").replace("||", " ;");
    let (mut cut, mut quiet_parts, mut whole_parts) = (String::new(), Vec::new(), Vec::new());
    let mut whole = String::new();
    for (c, original) in chained.chars().zip(line.chars()) {
        if matches!(c, ';' | '|' | '\n') {
            quiet_parts.push(std::mem::take(&mut cut));
            whole_parts.push(std::mem::take(&mut whole));
        } else {
            cut.push(c);
            whole.push(original);
        }
    }
    quiet_parts.push(cut);
    whole_parts.push(whole);
    let kept: Vec<usize> = (0..quiet_parts.len())
        .filter(|at| !quiet_parts[*at].trim().is_empty())
        .collect();
    let segments: Vec<String> =
        with_variables(kept.iter().map(|at| quiet_parts[*at].trim()).collect());
    // **Read with its quotes**, which the segments above lose: a `/` left alone by a quoted `|`
    // is no path.
    let whole: Vec<String> =
        with_variables(kept.iter().map(|at| whole_parts[*at].trim()).collect());
    // The owner's files it names, whether the segment naming each only reads, and which of them
    // the list has not answered.
    let reached: Vec<(String, ReachesTo)> = match folders {
        Some(_) => whole
            .iter()
            .flat_map(|said| {
                let to = if segment_only_reads(said) { ReachesTo::Read } else { ReachesTo::Write };
                reaches_into_owners_files(said).into_iter().map(move |word| (word, to))
            })
            .collect(),
        None => Vec::new(),
    };
    let mut unlisted: Option<(Option<String>, ReachesTo)> = None;
    if let Some(folders) = folders {
        for (word, to) in &reached {
            let at = folders.absolute(word);
            match at.as_deref().map(|at| folders.listed(at)) {
                Some(Listed::Blocked(folder)) => return Some(Guarded::Blocked { folder }),
                Some(Listed::Allowed) => {}
                _ => {
                    unlisted.get_or_insert((at, *to));
                }
            }
        }
    }
    // A `cd` into the slot keeps a relative path inside it; one anywhere else,
    // or one that cannot be read, loses track of where the line is.
    let slot = (!slots.is_empty()).then(|| Slot {
        slots,
        relative: segments.iter().all(|said| {
            // `(cd x && ...)` moves the subshell, so its `(` is not the program.
            let mut words = said.split_whitespace().map(|word| word.trim_start_matches(['(', '{'])).filter(|word| !word.is_empty());
            match words.next() {
                Some("cd" | "pushd") => words.next().is_some_and(|to| cd_stays(to, slots)),
                Some("popd") => false,
                _ => true,
            }
        }),
    });
    // A redirect into a file the reach covers is that reach, put to the owner or let in by the
    // list. **A removal is not**: an allowed folder never turns an `rm` into a run.
    let covered: Vec<&str> = reached.iter().map(|(word, _)| word.trim_matches(|c| c == '\'' || c == '"')).collect();
    let rest = segments.iter().find_map(|said| segment(said, slot, own, &covered));
    match (rest, unlisted) {
        (Some(because @ (Because::Destructive | Because::Unreadable)), _) => only(because),
        (_, Some((reaching, to))) => {
            Some(Guarded::Because { because: Because::ReachesOutside, reaching, to: Some(to) })
        }
        (rest, None) => rest.map(|because| Guarded::Because { because, reaching: None, to: None }),
    }
}

/// `line` with every `;`, `|`, `&`, `<` and `>` inside quotes turned to a
/// space. **Quoted text is an argument, not shell**: a commit message's
/// `<noreply@...>` and a `sed 's|a|b|'` script asked, as an overwrite, on
/// every commit a Session made (8 Oct 2026).
pub(super) fn unquoted(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    line.chars()
        .map(|c| {
            // `"a\|b"` is grep's alternation: an escaped `|` inside quotes is text as well.
            if escaped {
                escaped = false;
                return match quote {
                    Some(_) if matches!(c, ';' | '|' | '&' | '<' | '>' | '\n') => ' ',
                    _ => c,
                };
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
                Some(_) if matches!(c, ';' | '|' | '&' | '<' | '>' | '\n') => ' ',
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

/// Whether `word` appears in `line` as a word rather than inside a longer one,
/// so `evaluate.sh` is not an `eval`.
pub(super) fn word_in(line: &str, word: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        .any(|said| said == word)
}

/// The slots a session's line may write in, and whether a relative path in it is
/// still read against one of them.
#[derive(Clone, Copy)]
pub(in crate::helm) struct Slot<'a> {
    pub(in crate::helm) slots: &'a [&'a str],
    pub(in crate::helm) relative: bool,
}

impl Slot<'_> {
    pub(in crate::helm) fn holds(&self, word: &str) -> bool {
        in_the_slot(word, self.slots, self.relative)
    }
}

/// One segment of a shell line, already split from the rest.
pub(super) fn segment(said: &str, slot: Option<Slot>, own: Own, covered: &[&str]) -> Option<Because> {
    if truncating_redirect(said)
        && !redirect_targets(said).iter().all(|target| {
            slot.is_some_and(|slot| slot.holds(target))
                || covered.contains(&target.trim_matches(|c| c == '\'' || c == '"'))
        })
    {
        return Some(Because::Destructive);
    }
    let (program, arguments) = command_of(said)?;
    // **The shell builtins, and only in the position that makes them one.**
    // `pnpm --dir x exec vitest run` carries `exec` as a subcommand and is the
    // ordinary way this repository runs its tests; reading the word anywhere on
    // the line made every one of them ask.
    if program == "eval" || program == "exec" {
        return Some(Because::Unreadable);
    }
    match program {
        "git" => return git(&arguments, slot, own),
        "gh" => return forge(&arguments),
        "curl" | "wget" | "http" => return fetching(&arguments),
        _ => {}
    }
    const GONE: &[&str] = &[
        "rm",
        "rmdir",
        "shred",
        "unlink",
        "dd",
        "mkfs",
        "truncate",
        "kill",
        "killall",
        "pkill",
        "chown",
        "chmod",
        "mv",
        "launchctl",
        "diskutil",
        "systemctl",
    ];
    const SHARED: &[&str] = &["scp", "rsync", "ssh", "sftp"];
    const OFF_MACHINE: &[&str] = &["aws", "gcloud", "az", "kubectl", "terraform", "op"];
    const REMOVES: &[&str] = &["rm", "rmdir", "unlink"];
    if REMOVES.contains(&program) {
        let operands: Vec<&&str> = arguments
            .iter()
            .filter(|word| !word.starts_with('-'))
            .collect();
        let held = slot.is_some_and(|slot| {
            !operands.is_empty() && operands.iter().all(|word| slot.holds(word))
        });
        if !held {
            return Some(Because::Destructive);
        }
    } else if program == "kill" && kills_only_its_own(&arguments, own) {
        // A process it started is its own work, as a file in its slot is.
    } else if matches!(program, "chmod" | "chown") {
        if !changes_only_held(program, &arguments, slot) {
            return Some(Because::Destructive);
        }
    } else if queries(program, &arguments) {
        // `launchctl print` and `diskutil list` change nothing.
    } else if GONE.contains(&program) {
        return Some(Because::Destructive);
    }
    if SHARED.contains(&program) {
        return Some(Because::PushesToShared);
    }
    if OFF_MACHINE.contains(&program) {
        return Some(Because::WritesOffMachine);
    }
    // **A `push` subcommand, whatever pushed it.** Written as a subcommand test
    // rather than a list of programs because the list would be a list of
    // vendors, which is `adapters`' to hold and not this module's.
    if arguments.iter().find(|word| !word.starts_with('-')) == Some(&"push") {
        return Some(Because::PushesToShared);
    }
    match publishing(program, &arguments) {
        true => Some(Because::PushesToShared),
        false => None,
    }
}

/// A `>` that is not `>>` and not `>&`. **It destroys what the file held**,
/// however ordinary the command in front of it.
pub(in crate::helm) fn truncating_redirect(said: &str) -> bool {
    let written: Vec<char> = said.chars().collect();
    written.iter().enumerate().any(|(at, c)| {
        *c == '>'
            && at.checked_sub(1).and_then(|back| written.get(back)) != Some(&'>')
            && written.get(at + 1) != Some(&'>')
            && written.get(at + 1) != Some(&'&')
            && !into_a_device(&written[at + 1..])
    })
}

/// The paths each truncating `>` in `said` writes to.
pub(super) fn redirect_targets(said: &str) -> Vec<&str> {
    let written: Vec<char> = said.chars().collect();
    let mut targets = Vec::new();
    for (at, c) in said.char_indices() {
        let index = said[..at].chars().count();
        if c == '>'
            && index.checked_sub(1).and_then(|back| written.get(back)) != Some(&'>')
            && written.get(index + 1) != Some(&'>')
            && written.get(index + 1) != Some(&'&')
            && !into_a_device(&written[index + 1..])
        {
            targets.push(said[at + 1..].split_whitespace().next().unwrap_or(""));
        }
    }
    targets
}

/// A redirect into `/dev/null`, `/dev/stderr` and the rest. **A device holds
/// nothing to destroy**, and `2>/dev/null` is on most lines a session runs.
pub(super) fn into_a_device(after: &[char]) -> bool {
    let target: String = after.iter().collect();
    target.trim_start().starts_with("/dev/")
}
