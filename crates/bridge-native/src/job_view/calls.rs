//! The one line a run of tool calls reads as, ported from `calls-line.ts`
//! (`packages/components/src/compositions/SessionThread/calls-line.ts`). Fleet serves the run and its
//! rows; the words of the line are worked out from each row's text, which is Bridge's to word and
//! not a view Fleet holds (`docs/spikes/native-jobs-inventory/job-view.md`, A0.3).

/// What a tool does, by the name Bridge knows it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Edit,
    Read,
    Search,
    Run,
    Agent,
    Web,
    Watch,
    Ask,
}

impl Family {
    /// The verb while the run goes on, and the verb once it is done.
    fn verbs(self) -> (&'static str, &'static str) {
        match self {
            Family::Edit => ("Editing", "Edited"),
            Family::Read => ("Reading", "Read"),
            Family::Search => ("Searching", "Searched"),
            Family::Run => ("Running", "Ran"),
            Family::Agent => ("Briefing", "Briefed"),
            Family::Web => ("Reading", "Read"),
            Family::Watch => ("Watching", "Watched"),
            Family::Ask => ("Waiting for you to answer", "You answered"),
        }
    }
}

fn family_of(tool: &str) -> Option<Family> {
    Some(match tool {
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => Family::Edit,
        "Read" => Family::Read,
        "Grep" | "Glob" => Family::Search,
        "Bash" => Family::Run,
        "Agent" | "Task" => Family::Agent,
        "WebFetch" | "WebSearch" => Family::Web,
        "start_watch" => Family::Watch,
        "AskUserQuestion" => Family::Ask,
        _ => return None,
    })
}

/// Commands whose second word names what they do: `cargo check`, `git diff`.
const WITH_SUBCOMMAND: [&str; 13] = ["git", "cargo", "pnpm", "npm", "yarn", "npx", "bun", "armada", "gh", "make", "go", "uv", "brew"];
/// Lines that only set the scene for the one after them.
const SETTING_THE_SCENE: [&str; 7] = ["cd", "pushd", "popd", "export", "set", "unset", "source"];
const AT_MOST: usize = 40;

struct Call<'a> {
    tool: &'a str,
    detail: &'a str,
}

/// A row's text is `<Tool> <input>`; an MCP tool is `mcp__<server>__<tool>` and the last part names it.
fn tool_of(text: &str) -> Call<'_> {
    let (raw, detail) = match text.find(char::is_whitespace) {
        Some(at) => (&text[..at], text[at..].trim()),
        None => (text, ""),
    };
    Call { tool: raw.rsplit("__").next().unwrap_or(raw), detail }
}

fn base_name(detail: &str) -> &str {
    detail.split_whitespace().next().unwrap_or("").rsplit('/').find(|part| !part.is_empty()).unwrap_or("")
}

/// `NAME=value`, the environment assignment a shell line may start with.
fn is_assignment(word: &str) -> bool {
    match word.split_once('=') {
        Some((name, _)) => !name.is_empty() && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_'),
        None => false,
    }
}

/// The words of the segment of a shell line that does the work, after any `cd x &&` or assignment.
fn body_of(line: &str) -> Vec<&str> {
    let mut start = 0;
    let bytes = line.as_bytes();
    let mut segments = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let two = bytes.get(at..at + 2);
        if two == Some(b"&&") || two == Some(b"||") {
            segments.push(&line[start..at]);
            at += 2;
            start = at;
        } else if bytes[at] == b';' || bytes[at] == b'\n' {
            segments.push(&line[start..at]);
            at += 1;
            start = at;
        } else {
            at += 1;
        }
    }
    segments.push(&line[start..]);
    for segment in segments {
        let words: Vec<&str> = segment.split_whitespace().filter(|word| !is_assignment(word)).collect();
        if let Some(first) = words.first() {
            if !SETTING_THE_SCENE.contains(first) {
                return words;
            }
        }
    }
    Vec::new()
}

/// The first one or two words of what a shell line runs.
pub fn command_of(line: &str) -> String {
    let words = body_of(line);
    let Some(first) = words.first() else { return String::new() };
    let name = first.rsplit('/').next().unwrap_or(first);
    match words.get(1) {
        Some(second) if WITH_SUBCOMMAND.contains(&name) && !second.starts_with('-') => format!("{name} {second}"),
        _ => name.to_string(),
    }
}

/// Whether a shell line is one of the runs that count as a check.
fn is_check(line: &str) -> bool {
    let words = body_of(line);
    let said = words.join(" ");
    let boundary = |prefix: &str| {
        said.strip_prefix(prefix).is_some_and(|rest| rest.chars().next().is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_')))
    };
    ["armada check", "cargo check", "cargo test", "cargo clippy", "pnpm test", "pnpm exec vitest", "pnpm exec tsc", "npm test", "vitest", "tsc", "pytest"]
        .iter()
        .any(|prefix| boundary(prefix))
}

fn clip(said: &str) -> String {
    if said.chars().count() > AT_MOST {
        let kept: String = said.chars().take(AT_MOST - 1).collect();
        format!("{kept}…")
    } else {
        said.to_string()
    }
}

fn about_of(family: Family, calls: &[&Call<'_>]) -> String {
    let mut details: Vec<&str> = Vec::new();
    for call in calls {
        if !details.contains(&call.detail) {
            details.push(call.detail);
        }
    }
    let last = calls.last().expect("a family is named by a call");
    match family {
        Family::Edit | Family::Read => {
            let mut files: Vec<&str> = Vec::new();
            for call in calls {
                let name = base_name(call.detail);
                if !name.is_empty() && !files.contains(&name) {
                    files.push(name);
                }
            }
            match files.as_slice() {
                [] => "a file".into(),
                [one] => (*one).into(),
                _ => "multiple files".into(),
            }
        }
        Family::Search | Family::Watch => clip(last.detail),
        Family::Run => {
            if details.len() > 1 {
                if details.iter().all(|line| is_check(line)) {
                    format!("{} checks", details.len())
                } else {
                    "multiple commands".into()
                }
            } else {
                clip(&command_of(details.first().copied().unwrap_or("")))
            }
        }
        Family::Agent => "an agent".into(),
        Family::Web => "the web".into(),
        Family::Ask => String::new(),
    }
}

/// The line for a run of calls: what the last call Bridge knows by name says the run is doing, and
/// the calls of its kind say how many places it touched. A run of nothing known reads as its last
/// tool's name. `working` picks the verb's tense.
pub fn calls_line(texts: &[&str], working: bool) -> String {
    let calls: Vec<Call<'_>> = texts.iter().map(|text| tool_of(text)).collect();
    let Some(last) = calls.iter().filter_map(|call| family_of(call.tool)).next_back() else {
        return calls.last().map_or(String::new(), |call| call.tool.to_string());
    };
    let of: Vec<&Call<'_>> = calls.iter().filter(|call| family_of(call.tool) == Some(last)).collect();
    // A briefed agent is named by its description, which is already a phrase.
    if last == Family::Agent {
        let described = of.last().map_or("", |call| call.detail);
        if !described.is_empty() {
            return clip(described);
        }
    }
    let (running, finished) = last.verbs();
    let verb = if working { running } else { finished };
    let about = about_of(last, &of);
    if about.is_empty() {
        verb.to_string()
    } else {
        format!("{verb} {about}")
    }
}

/// One call on a line of its own: a tool Bridge has no verb for says its whole call.
pub fn call_line(text: &str, working: bool) -> String {
    if family_of(tool_of(text).tool).is_none() {
        clip(text)
    } else {
        calls_line(&[text], working)
    }
}

/// The kind a call's row draws: its icon and the colour of its rail.
pub(super) fn kind_of(text: &str) -> super::tool_icons::Kind {
    use super::tool_icons::Kind;
    match family_of(tool_of(text).tool) {
        Some(Family::Read) => Kind::Read,
        Some(Family::Run) => Kind::Run,
        Some(Family::Edit) => Kind::Edit,
        Some(Family::Search) => Kind::Search,
        Some(Family::Agent) => Kind::Agent,
        Some(Family::Web) => Kind::Web,
        Some(Family::Watch | Family::Ask) | None => Kind::Other,
    }
}

/// What a running call works on, for the row's second line: the command's first line after `$ `,
/// the path, or the pattern. None for a kind that says it in its phrase.
pub(super) fn working_on(text: &str) -> Option<String> {
    let call = tool_of(text);
    let detail = call.detail.trim();
    if detail.is_empty() {
        return None;
    }
    match family_of(call.tool)? {
        Family::Run => Some(format!("$ {}", detail.lines().next().unwrap_or(""))),
        Family::Read | Family::Edit | Family::Search => Some(detail.lines().next().unwrap_or("").to_string()),
        _ => None,
    }
}

/// The tools a run holds, each once, in the order first called, for its name.
pub fn names_of(texts: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for text in texts {
        let name = tool_of(text).tool.to_string();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_file_is_named_and_several_are_multiple_files() {
        assert_eq!(calls_line(&["Edit crates/store/src/audit.rs"], false), "Edited audit.rs");
        assert_eq!(calls_line(&["Edit a/one.rs", "Edit a/two.rs"], false), "Edited multiple files");
        assert_eq!(calls_line(&["Edit a/one.rs", "Edit b/one.rs"], true), "Editing one.rs");
    }

    #[test]
    fn the_last_tool_bridge_knows_names_the_run() {
        assert_eq!(calls_line(&["Edit a.rs", "Foo bar", "Read b.rs"], false), "Read b.rs");
        assert_eq!(calls_line(&["Foo bar"], false), "Foo");
        assert_eq!(calls_line(&["mcp__armada-fleet__get_health"], false), "get_health");
    }

    #[test]
    fn a_shell_line_reads_as_its_command_and_checks_keep_a_count() {
        assert_eq!(calls_line(&["Bash cd x && cargo check --all"], false), "Ran cargo check");
        assert_eq!(calls_line(&["Bash cargo check", "Bash pnpm test", "Bash cargo clippy"], true), "Running 3 checks");
        assert_eq!(calls_line(&["Bash ls -la", "Bash cargo check"], false), "Ran multiple commands");
        assert_eq!(calls_line(&["Bash FOO=1 ./scripts/preview --restart"], false), "Ran preview");
    }

    #[test]
    fn a_briefed_agent_is_named_by_its_description_and_a_long_phrase_is_clipped() {
        assert_eq!(calls_line(&["Agent Read the docs index"], false), "Read the docs index");
        let long = format!("Agent {}", "x".repeat(60));
        assert_eq!(calls_line(&[long.as_str()], false).chars().count(), 40);
        assert_eq!(calls_line(&["Agent"], false), "Briefed an agent");
    }

    #[test]
    fn a_tool_with_no_verb_says_its_whole_call_on_a_line_of_its_own() {
        assert_eq!(call_line("Foo --flag value", false), "Foo --flag value");
        assert_eq!(call_line("Read src/main.rs", true), "Reading main.rs");
        assert_eq!(call_line("AskUserQuestion", false), "You answered");
    }

    #[test]
    fn a_running_call_says_what_it_works_on_in_one_line() {
        assert_eq!(working_on("Bash cd x && cargo check\n--all").as_deref(), Some("$ cd x && cargo check"));
        assert_eq!(working_on("Read crates/api/src/router.rs").as_deref(), Some("crates/api/src/router.rs"));
        assert_eq!(working_on("Grep mergeLineTone").as_deref(), Some("mergeLineTone"));
        assert_eq!(working_on("Agent Read the docs index"), None);
        assert_eq!(working_on("Bash"), None);
    }

    #[test]
    fn names_are_each_once_in_first_call_order() {
        assert_eq!(names_of(&["Edit a", "Bash b", "Edit c", "mcp__x__show_file p"]), ["Edit", "Bash", "show_file"]);
    }
}
