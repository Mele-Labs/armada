//! The account a small model writes of a Session at work: what is asked of it, and how its answer
//! reads. `docs/concepts/session/working-on.md`.
//!
//! **Nothing here calls a model or touches a Session.** The question is laid from what Fleet
//! detected, and the answer is read into lines the person can be shown. A line that names a file,
//! a path, a code identifier or a tool is dropped on the way in, because the prompt asks the model
//! not to write one and a model is not held to it. The Job's running summary, `so_far`, is asked and
//! read here too: one or two plain sentences, dropped whole where they name code.

use crate::prompts::fill;

/// The most of the person's message the model is shown.
const MESSAGE_AT_MOST: usize = 1_200;
/// The most of one thing the agent said that the model is shown.
const SAID_AT_MOST: usize = 600;
/// The most calls and texts of the turn the model is shown: the latest.
pub(crate) const CALLS_AT_MOST: usize = 60;
pub(crate) const SAID_MOST: usize = 8;
/// The most lines a written account keeps, and the longest one.
const DONE_AT_MOST: usize = 5;
const LINE_AT_MOST: usize = 140;
/// The longest running summary kept.
const SO_FAR_AT_MOST: usize = 280;

/// The prompt as it ships. **Not an editable setting**, as the Session proposer's is not: the answer
/// is read by these field names.
pub(crate) const SHIPPED: &str = "\
A person asked a coding agent to do some work and is waiting on it. Write a short account of the \
agent's progress for his last message, in plain words, for someone who does not read code.

Use the person's own words for the things he asked for. Never write a file name, a path, a \
function or variable name, a command, or the name of a tool. Say what the work is for instead: \
\"the sign-in page\", never \"login.tsx\". Each line is one short sentence with no more than \
twelve words, and none of them narrates (\"Let me\", \"I am going to\"). Leave a line out where you \
cannot tell.

The person's last message:

{message}

What the agent did this turn, oldest first:

{calls}

What the agent said this turn, oldest first:

{said}
{groups}
Answer with nothing but the lines below, in this order.

    done: <one thing finished, and a `done:` line for each other, oldest first; at most five>
    now: <what the agent is on now>
    next: <what the agent does next>{group_line}

Reading and searching are not worth a `done:` line. A thing not yet finished is not done.";

/// The prompt for the Job's running summary. Not an editable setting, for the reason `SHIPPED` is not.
/// **It must not carry the phrase the account's question is picked out by in tests.**
pub(crate) const SO_FAR_SHIPPED: &str = "\
A person has a coding agent working on a Job. He is about to answer a call from it without opening \
the Job, so he needs what it has done so far. Write that in at most two sentences, in plain words, \
for someone who does not read code.

Use the person's own words for the things he asked for. Never write a file name, a path, a \
function or variable name, a command, or the name of a tool. Say what the work is for instead: \
\"the sign-in page\", never \"login.tsx\". State facts in the past tense with no subject: \"Found the \
reads happen once per Job. Added a cache, not keyed yet.\" Say what changed for the work, and leave \
out approvals, plans and checks that passed. Do not narrate, do not praise, do not say what comes \
next, and do not repeat what it is asking him: he can see the question. At most forty words.

The summary so far, which covers the Job up to now. Keep what still matters and shorten it:

{last}

The person's last message:

{message}

What was finished this turn:

{done}

What the agent said this turn, oldest first:

{said}

Answer with nothing but the new summary.";

/// What the model is shown of the Job to write its running summary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct JobSoFar {
    /// The summary kept so far.
    pub last: Option<String>,
    /// The person's last message.
    pub message: Option<String>,
    /// What was finished this turn, from the kept account.
    pub done: Vec<String>,
    /// What the agent said this turn, oldest first.
    pub said: Vec<String>,
}

impl JobSoFar {
    /// The question, filled in.
    pub(crate) fn question(&self) -> String {
        let said: Vec<String> = self.said.iter().map(|one| cut(one, SAID_AT_MOST)).collect();
        let nothing = "(nothing yet)".to_string();
        fill(
            SO_FAR_SHIPPED,
            &[
                ("last", self.last.as_deref().unwrap_or("(nothing yet)")),
                ("message", &self.message.as_deref().map_or(nothing, |one| cut(one, MESSAGE_AT_MOST))),
                ("done", &listed(&self.done, "nothing yet")),
                ("said", &listed(&said, "nothing yet")),
            ],
        )
    }
}

/// The model's answer for the running summary, read: one line, or `None` where it says nothing or
/// names code.
pub(crate) fn read_so_far(reply: &str) -> Option<String> {
    let joined = reply.lines().map(str::trim).filter(|line| !line.is_empty()).collect::<Vec<_>>().join(" ");
    let line = joined.trim_matches('`').trim().trim_matches(['"', '\u{201c}', '\u{201d}']).trim();
    let nothing = line.is_empty() || ["none", "nothing", "n/a"].contains(&line.to_lowercase().trim_end_matches('.'));
    (!nothing && !names_code(line)).then(|| cut(line, SO_FAR_AT_MOST))
}

/// What the model is shown of the turn so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TurnSoFar {
    /// The person's last message.
    pub message: String,
    /// What each tool call of the turn did, as `now` words, oldest first.
    pub calls: Vec<String>,
    /// What the agent said this turn, oldest first.
    pub said: Vec<String>,
    /// The approved plan's groups as `(id, title)`, offered only where the Session has no task at work
    /// to name the group by. Empty is no choice to make.
    pub groups: Vec<(String, String)>,
}

impl TurnSoFar {
    /// The question, filled in.
    pub(crate) fn question(&self) -> String {
        let said: Vec<String> = self.said.iter().map(|one| cut(one, SAID_AT_MOST)).collect();
        let (groups, group_line) = match self.groups.is_empty() {
            true => (String::new(), ""),
            false => {
                let held: Vec<String> = self.groups.iter().map(|(id, title)| format!("  {id} — {title}")).collect();
                (
                    format!("\nThe agent follows a plan of these groups. Name the one it is on:\n\n{}\n", held.join("\n")),
                    "\n    group: <the id of the group it is on, spelled exactly as above, or `none`>",
                )
            }
        };
        fill(
            &SHIPPED.replace("{group_line}", group_line),
            &[
                ("message", &cut(&self.message, MESSAGE_AT_MOST)),
                ("calls", &listed(&self.calls, "nothing yet")),
                ("said", &listed(&said, "nothing yet")),
                ("groups", &groups),
            ],
        )
    }
}

/// `lines` as a list, or `(none)` where there are none.
fn listed(lines: &[String], none: &str) -> String {
    match lines.is_empty() {
        true => format!("  ({none})"),
        false => lines.iter().map(|one| format!("  - {one}")).collect::<Vec<_>>().join("\n"),
    }
}

/// What the model wrote, in lines the person can be shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Written {
    pub done: Vec<String>,
    pub now: Option<String>,
    pub next: Option<String>,
    /// The group the model named, if it is one of those it was offered.
    pub group: Option<String>,
}

/// The model's answer, read. `None` where no line of it survives, which writes no account.
/// `groups` are those the question offered.
pub(crate) fn read(reply: &str, groups: &[(String, String)]) -> Option<Written> {
    let mut written = Written::default();
    for line in reply.lines() {
        let line = line.trim().trim_start_matches(['-', '*', ' ']);
        let Some((name, rest)) = line.split_once(':') else { continue };
        let rest = rest.trim();
        match name.trim().to_lowercase().as_str() {
            "done" if written.done.len() < DONE_AT_MOST => written.done.extend(kept(rest)),
            "now" => written.now = kept(rest),
            "next" => written.next = kept(rest),
            "group" => written.group = groups.iter().find(|(id, _)| id == rest).map(|(id, _)| id.clone()),
            _ => {}
        }
    }
    (written != Written::default()).then_some(written)
}

/// A line as it is kept: cut, and absent where it says nothing or names code.
fn kept(line: &str) -> Option<String> {
    let line = line.trim().trim_matches('`').trim();
    let nothing = line.is_empty() || ["none", "nothing", "n/a"].contains(&line.to_lowercase().trim_end_matches('.'));
    (!nothing && !names_code(line)).then(|| cut(line, LINE_AT_MOST))
}

/// Tools whose names are not words of English.
const TOOLS: [&str; 9] =
    ["TodoWrite", "MultiEdit", "NotebookEdit", "WebFetch", "WebSearch", "ToolSearch", "Bash", "Grep", "Glob"];

/// Whether `line` carries a path, a file name, a code identifier or a tool's name. **Judged by the
/// shape of a word**: a slash, a backtick, a call's brackets, `::`, an underscore, a camelCase join
/// (`JavaScript` is a name and is let through), or a name with an extension.
fn names_code(line: &str) -> bool {
    line.split_whitespace().any(|word| {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric() && !"_/\\`:()".contains(c));
        let bare = word.trim_end_matches(['.', ',', ';', ':']);
        let humps = bare
            .char_indices()
            .filter(|(at, c)| *at > 0 && c.is_uppercase() && bare[..*at].ends_with(|before: char| before.is_lowercase()))
            .count();
        let camel = humps > usize::from(bare.starts_with(|c: char| c.is_uppercase()));
        let file = bare.rsplit_once('.').is_some_and(|(stem, ext)| {
            (2..=5).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
                && ext.chars().any(|c| c.is_ascii_alphabetic())
                && stem.chars().any(|c| c.is_alphabetic())
                && stem.chars().all(|c| c.is_alphanumeric() || "-_/".contains(c))
        });
        let path = bare.contains('/') && !bare.eq_ignore_ascii_case("and/or");
        path || bare.contains(['\\', '`', '_']) || bare.contains("::") || bare.contains("()") || camel || file || TOOLS.contains(&bare)
    })
}

fn cut(text: &str, at_most: usize) -> String {
    let text = text.trim();
    match text.chars().count() <= at_most {
        true => text.to_string(),
        false => format!("{}…", text.chars().take(at_most - 1).collect::<String>().trim_end()),
    }
}
