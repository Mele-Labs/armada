//! The message a report about a Job starts a new Job with (`report-brief.ts`): the owner's words as he typed them, then what Bridge holds about the reported Job as plain labelled lines, each left out where it is not known.
use chrono::{DateTime, SecondsFormat, Utc};
use ipc::{AttachmentState, SessionRecord, SessionRow, SessionState, SessionTurn, SessionVoice, ThreadItem};

const DEBUG_INFO_HEADING: &str = "## Debug info";
const MESSAGES: usize = 6;
const TOOLS: usize = 5;
const CHARS: usize = 300;

/// What the window knows that the record does not say. The timeline's entries are not here: the native Bridge holds none, so that section is never written.
pub(super) struct ReportFrom {
    pub at: DateTime<Utc>,
    /// Where the report was pressed: the screen and the control.
    pub screen: String,
    /// The repository's root directory.
    pub root: String,
    /// The repository's name, where it has one.
    pub repository: Option<String>,
    /// What Fleet last refused a Job act, in words.
    pub refused: Option<String>,
    /// The questions nobody answered, from the thread's head.
    pub lapsed: Option<ipc::Lapsed>,
}

/// One line, cut: whitespace folded and a cut marked with an ellipsis.
fn cut(text: &str, most: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= most {
        return flat;
    }
    let head: String = flat.chars().take(most - 1).collect();
    format!("{}…", head.trim_end())
}

/// A wire enum as the wire spells it, so a state reads as Fleet carries it.
fn wire_word<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(word)) => word,
        _ => String::new(),
    }
}

/// The harness as a person names it (`acme-code` as `Acme Code`), and the first word of it lowercased, which is the folder its own files sit in. Read off the record, so no vendor is spelled here.
fn harness_words(harness: &str) -> Option<(String, String)> {
    let words: Vec<&str> = harness.split(['-', '_', ' ']).filter(|word| !word.is_empty()).collect();
    let first = words.first()?;
    let name = words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map(|head| head.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    Some((name, first.to_lowercase()))
}

/// Where the harness keeps a session's transcript: the directory is the working directory with every character that is not a letter or digit as `-`, one for each UTF-16 unit as the web build writes it. `None` where the record names no harness.
pub(super) fn transcript_of(harness: &str, cwd: &str, id: &str) -> Option<String> {
    let (_, folder) = harness_words(harness)?;
    let dir: String = cwd.chars().flat_map(|c| std::iter::repeat_n(if c.is_ascii_alphanumeric() { c } else { '-' }, if c.is_ascii_alphanumeric() { 1 } else { c.len_utf16() })).collect();
    Some(format!("~/.{folder}/projects/{dir}/{id}.jsonl"))
}

/// `Math.round(tokens / 1000)` and a `k`.
fn kb(tokens: u64) -> String {
    format!("{}k", (tokens + 500) / 1000)
}

/// A number the wire carries as text, as JavaScript's `Number` prints it where it reads one.
fn number(target: &str) -> String {
    target.trim().parse::<i64>().map(|n| n.to_string()).unwrap_or_else(|_| target.to_string())
}

fn detail<'a>(one: &'a ipc::Attachment, key: &str) -> Option<&'a str> {
    one.detail.get(key).map(String::as_str)
}

/// Its state in the words the Jobs page uses for what it can be sent.
fn state_of(record: &SessionRecord) -> String {
    let ended = record.state == SessionState::Ended;
    let base = if ended {
        "ended"
    } else if record.hosted.is_none() && !record.terminal.as_ref().is_some_and(|terminal| terminal.listening) {
        "terminal not listening"
    } else if matches!(record.hosted.as_ref().map(|hosted| &hosted.turn), Some(SessionTurn::Working { .. })) {
        "working"
    } else {
        "idle"
    };
    let Some(standing) = record.standing.filter(|_| !ended) else { return base.to_string() };
    let standing = wire_word(&standing);
    if standing == base {
        return base.to_string();
    }
    if base == "idle" && standing == "working" {
        if let Some(running) = record.attachments.iter().find(|one| one.kind == "subagent" && one.state == AttachmentState::Standing) {
            let task = detail(running, "task").or_else(|| detail(running, "description")).unwrap_or("Subagent");
            return format!("{base}, a subagent working: {task}");
        }
    }
    if standing == "stopped" {
        if let Some(stopped) = &record.stopped {
            return format!("{base}, stopped: {}", wire_word(&stopped.reason));
        }
    }
    format!("{base}, read by Fleet as {standing}")
}

/// One attachment as a labelled line, for the kinds the TS words.
fn attachment_line(one: &ipc::Attachment) -> Option<(&'static str, String)> {
    match one.kind.as_str() {
        "pr" => {
            let checks = match detail(one, "checks") {
                Some("passed") => "checks passed".to_string(),
                Some("failed") => format!("checks failed ({})", detail(one, "failing").unwrap_or("")),
                _ => "checks pending".to_string(),
            };
            let state = if one.state == AttachmentState::Spent || detail(one, "state") == Some("merged") {
                "merged"
            } else if detail(one, "state") == Some("draft") {
                "draft"
            } else {
                "open"
            };
            let landing = if detail(one, "queued") == Some("true") {
                ", in merge queue"
            } else if detail(one, "auto_merge") == Some("true") && state != "merged" {
                ", auto-merge on"
            } else {
                ""
            };
            Some(("Pull request", format!("#{} {}, {state}{landing}, {checks}, branch {}, {}", number(&one.target), detail(one, "title").unwrap_or(""), detail(one, "branch").unwrap_or(""), detail(one, "address").unwrap_or(""))))
        }
        "forked_from" => Some(("Forked from Job", one.target.clone())),
        "forked_to" => Some(("Forked to Job", one.target.clone())),
        "started_by" => Some(("Started by Job", one.target.clone())),
        "started_to" => Some(("Started Job", one.target.clone())),
        _ => None,
    }
}

/// What a Job's ledger shows a person: a row let go is out, and a Job that ended keeps only its links and its pull requests.
fn drawn(record: &SessionRecord) -> impl Iterator<Item = &ipc::Attachment> {
    let ended = record.state == SessionState::Ended;
    record.attachments.iter().filter(move |one| one.state != AttachmentState::GivenBack && (!ended || matches!(one.kind.as_str(), "forked_to" | "forked_from" | "started_to" | "started_by" | "pr")))
}

fn who(from: &SessionVoice) -> String {
    match from {
        SessionVoice::You => "Owner".to_string(),
        SessionVoice::Agent => "Agent".to_string(),
        SessionVoice::Session { id, title } => format!("Job {title} ({id})"),
    }
}

/// The labelled lines about `record`, one per fact, in the order they are read. A fact that is not known is not a line.
fn facts(record: &SessionRecord, from: &ReportFrom) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut add = |label: &str, value: Option<String>| {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            lines.push(format!("{label}: {value}"));
        }
    };
    let ended = record.state == SessionState::Ended;
    let id = record.id.as_str();
    let title = record.title.as_deref().map(|title| title.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|title| !title.is_empty());
    let harness = harness_words(&record.harness);
    add("Reported at", Some(from.at.to_rfc3339_opts(SecondsFormat::Millis, true)));
    add("Pressed from", Some(from.screen.clone()));
    add("Job", title.map(|title| format!("{title} ({id})")));
    add("Armada job id", Some(id.to_string()));
    if let Some((name, _)) = &harness {
        add(&format!("{name} session id"), Some(id.to_string()));
    }
    add("Kind", Some(if record.hosted.is_none() { "run in a terminal" } else { "hosted by Fleet" }.to_string()));
    add("Repository", Some(match &from.repository { Some(name) => format!("{name}, {}", from.root), None => from.root.clone() }));
    if !record.cwd.is_empty() {
        add("Started in", Some(record.cwd.clone()));
        add("Transcript", transcript_of(&record.harness, &record.cwd, id));
    }
    let held: Vec<&ipc::Attachment> = drawn(record).collect();
    let slots: Vec<&ipc::Attachment> = held.iter().copied().filter(|one| one.kind == "slot" && one.state == AttachmentState::Standing).collect();
    let bay_path = |slot: &str| format!("{}/.armada/slots/slot-{slot}", from.root);
    let mut branches = 0;
    for one in held.iter().filter(|one| one.kind == "branch" && one.state == AttachmentState::Standing) {
        let slot = slots.iter().find(|slot| slot.manifest_id == one.manifest_id).or(slots.first()).map_or_else(|| "0".to_string(), |slot| number(&slot.target));
        add("Branch", Some(format!("{}, slot {slot}, {}", one.target, bay_path(&slot))));
        branches += 1;
    }
    if branches == 0 {
        for one in &slots {
            let slot = number(&one.target);
            add("Slot", Some(format!("{slot}, {}", bay_path(&slot))));
        }
    }
    if branches == 0 && slots.is_empty() {
        let mut bays: Vec<&ipc::Attachment> = record.attachments.iter().filter(|one| one.kind == "slot").collect();
        bays.sort_by(|a, b| a.since.cmp(&b.since));
        add("Slot held earlier", Some(bays.iter().map(|one| format!("slot-{}", number(&one.target))).collect::<Vec<_>>().join(", ")));
    }
    add("State", Some(state_of(record)));
    add("Started", Some(record.started_at.as_str().to_string()));
    add("Last turn ended", record.last_turn_at.as_ref().map(|at| at.as_str().to_string()));
    add("Held on the plan limit until", record.held_until.as_ref().filter(|_| !ended).map(|at| at.as_str().to_string()));
    let hosted = record.hosted.as_ref();
    let terminal = record.terminal.as_ref();
    let model = [
        hosted.and_then(|hosted| hosted.model.clone()).or_else(|| terminal.and_then(|terminal| terminal.model.clone())),
        hosted.and_then(|hosted| hosted.resolved_model.as_ref()).map(|resolved| format!("resolved {resolved}")),
        hosted.and_then(|hosted| hosted.effort.clone()).or_else(|| terminal.and_then(|terminal| terminal.effort.clone())).map(|effort| format!("effort {effort}")),
        hosted.map(|hosted| hosted.mode).or_else(|| terminal.and_then(|terminal| terminal.mode)).map(|mode| format!("mode {}", wire_word(&mode))),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    add("Model", Some(model.join(", ")));
    add("Context", record.usage.context_tokens.zip(record.usage.context_window).map(|(tokens, window)| format!("{} of {} tokens", kb(tokens), kb(window))));
    for one in drawn(record) {
        if let Some((label, value)) = attachment_line(one) {
            add(label, Some(value));
        }
    }
    if !ended {
        for item in &record.waiting_for {
            add("Waiting on the owner", Some(format!("{} ({}, since {})", cut(&item.text, 200), wire_word(&item.source), item.since.as_str())));
        }
    }
    if let Some(asked) = hosted.and_then(|hosted| hosted.asked.as_ref()).or_else(|| terminal.and_then(|terminal| terminal.asked.as_ref())) {
        let command = asked.whole.clone().unwrap_or_else(|| if asked.detail.is_empty() { asked.tool.clone() } else { asked.detail.clone() });
        add("Held on a call", Some(cut(&command, 200)));
    }
    if let Some(lapsed) = &from.lapsed {
        add("Question nobody answered", Some(cut(&lapsed.questions.iter().map(|one| one.question.as_str()).collect::<Vec<_>>().join(" / "), 200)));
    }
    add("Last Job act Fleet refused", from.refused.as_deref().map(|refused| cut(refused, 200)));
    lines
}

/// The message and tool rows of the thread in the order they were written: the ones drawn as they came, those a run of calls folds, and a Fleet turn's notice and what the agent did under it.
fn rows_of(items: &[ThreadItem]) -> Vec<&SessionRow> {
    let mut out = Vec::new();
    for item in items {
        match item {
            ThreadItem::Row { row } => out.push(row),
            ThreadItem::Calls { rows, .. } => out.extend(rows),
            ThreadItem::FleetTurn { notice, rows, .. } => {
                out.push(notice);
                out.extend(rows);
            }
            _ => {}
        }
    }
    out
}

/// The sections that are lists: why it did not start, where it did not, then its last messages and its last tool calls.
fn lists(record: &SessionRecord, items: &[ThreadItem]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(failed) = record.hosted.as_ref().filter(|_| record.state != SessionState::Ended).and_then(|hosted| hosted.start_failed.as_ref()) {
        out.push(String::new());
        out.push(format!("Did not start: {}", cut(&failed.cause, CHARS)));
        if !failed.log.is_empty() {
            // A fence one longer than any backtick run in the log, so a line of the agent's own cannot close it.
            let longest = failed.log.iter().map(|line| backtick_run(line)).max().unwrap_or(0);
            let fence = "`".repeat((longest + 1).max(3));
            out.push(String::new());
            out.push("The agent's last error lines, oldest first:".to_string());
            out.push(fence.clone());
            out.extend(failed.log.iter().cloned());
            out.push(fence);
        }
    }
    let rows = rows_of(items);
    let mut section = |head: &str, lines: Vec<String>| {
        if lines.is_empty() {
            return;
        }
        out.push(String::new());
        out.push(format!("{head}:"));
        out.extend(lines.into_iter().map(|line| format!("- {line}")));
    };
    let messages: Vec<String> = rows.iter().filter_map(|row| match row { SessionRow::Message { at, from, text, .. } => Some(format!("{} at {}: {}", who(from), at.as_str(), cut(text, CHARS))), _ => None }).collect();
    section("Last messages, oldest first", messages[messages.len().saturating_sub(MESSAGES)..].to_vec());
    let tools: Vec<String> = rows.iter().filter_map(|row| match row { SessionRow::Tool { at, text, .. } => Some(format!("{} {}", at.as_str(), cut(text, 160))), _ => None }).collect();
    section("Last tool calls, oldest first", tools[tools.len().saturating_sub(TOOLS)..].to_vec());
    out
}

/// The longest run of backticks in `line`.
fn backtick_run(line: &str) -> usize {
    line.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

/// The owner's words first and untouched, then the facts, then what to do with them.
pub(super) fn report_brief(words: &str, record: &SessionRecord, items: &[ThreadItem], from: &ReportFrom) -> String {
    let mut lines: Vec<String> = vec![
        words.trim().to_string(),
        String::new(),
        "The owner reported the Job below from Bridge. What Bridge held about it when he pressed Report is under Debug info.".to_string(),
        String::new(),
        "Read its transcript and its Fleet record (list_sessions, get_events_since), work out what happened, and say what you found. If the fix is small, make it on a branch. If it is not small, file an issue.".to_string(),
        String::new(),
        // The thread folds what follows, so the owner's words and what to do stay in view.
        DEBUG_INFO_HEADING.to_string(),
    ];
    lines.extend(facts(record, from));
    lines.extend(lists(record, items));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::Instant;

    fn record() -> SessionRecord {
        let mut jobs = crate::mock::jobs("jobs-screens").expect("the fixture decodes");
        let mut job = jobs.iter().position(|job| job.hosted.is_some() && job.state == SessionState::Live).map(|at| jobs.swap_remove(at)).expect("a live hosted Job");
        job.title = Some("Fix the  list".into());
        job.cwd = "/work/my repo".into();
        job
    }

    fn from() -> ReportFrom {
        ReportFrom { at: "2026-11-10T09:08:07.123Z".parse().unwrap(), screen: "Jobs, Report".into(), root: "/work/my repo".into(), repository: Some("my repo".into()), refused: None, lapsed: None }
    }

    fn message(id: &str, from: SessionVoice, text: &str) -> ThreadItem {
        ThreadItem::Row { row: SessionRow::Message { id: id.into(), at: Instant::carried("2026-11-10T09:00:00.000Z"), from, text: text.into(), files: vec![], tags: vec![], notice: None, queued: false } }
    }

    #[test]
    fn the_words_come_first_and_untouched_then_the_heading() {
        let brief = report_brief("  It froze\non the third row  ", &record(), &[], &from());
        assert!(brief.starts_with("It froze\non the third row\n\nThe owner reported the Job below"));
        let at = brief.find("\n\n## Debug info\nReported at: 2026-11-10T09:08:07.123Z\nPressed from: Jobs, Report\n").expect("the facts follow the heading");
        assert!(brief[..at].ends_with("make it on a branch. If it is not small, file an issue."));
    }

    #[test]
    fn a_fact_the_record_lacks_is_no_line() {
        let mut job = record();
        job.last_turn_at = None;
        job.held_until = None;
        job.usage.context_tokens = None;
        let brief = report_brief("Words", &job, &[], &from());
        assert!(!brief.contains("Last turn ended:"));
        assert!(!brief.contains("Held on the plan limit until:"));
        assert!(!brief.contains("Context:"));
        assert!(!brief.contains("Last Job act Fleet refused:"));
        assert!(brief.contains(&format!("Job: Fix the list ({})", job.id.as_str())));
        assert!(brief.contains("Repository: my repo, /work/my repo\nStarted in: /work/my repo\nTranscript: ~/."));
        assert!(brief.contains("/projects/-work-my-repo/"));
        job.last_turn_at = Some(Instant::carried("2026-11-10T08:00:00.000Z"));
        assert!(report_brief("Words", &job, &[], &from()).contains("Last turn ended: 2026-11-10T08:00:00.000Z"));
    }

    #[test]
    fn a_cut_folds_whitespace_and_ends_with_an_ellipsis() {
        assert_eq!(cut("  one \n\t two   three ", 300), "one two three");
        assert_eq!(cut("abc def", 7), "abc def");
        assert_eq!(cut("abc def ghi", 6), "abc d…");
        assert_eq!(cut("abc  def ghi", 5), "abc…");
    }

    #[test]
    fn a_transcript_path_turns_every_other_character_into_a_dash() {
        assert_eq!(transcript_of("acme-code", "/a/b c.d", "x").as_deref(), Some("~/.acme/projects/-a-b-c-d/x.jsonl"));
        assert_eq!(harness_words("acme-code"), Some(("Acme Code".to_string(), "acme".to_string())));
        assert_eq!(transcript_of("", "/a", "x"), None);
    }

    #[test]
    fn the_fence_is_longer_than_any_run_in_the_log_and_at_least_three() {
        let mut job = record();
        let hosted = job.hosted.as_mut().unwrap();
        hosted.start_failed = Some(ipc::StartFailed { cause: "no   binary".into(), log: vec!["plain".into(), "a ```` b `` c".into()] });
        let brief = report_brief("Words", &job, &[], &from());
        assert!(brief.contains("\nDid not start: no binary\n\nThe agent's last error lines, oldest first:\n`````\nplain\na ```` b `` c\n`````"));
        job.hosted.as_mut().unwrap().start_failed = Some(ipc::StartFailed { cause: "x".into(), log: vec!["plain".into()] });
        assert!(report_brief("Words", &job, &[], &from()).contains("oldest first:\n```\nplain\n```"));
    }

    #[test]
    fn the_last_messages_are_six_and_the_last_tool_calls_five_oldest_first() {
        let mut items: Vec<ThreadItem> = (0..8).map(|n| message(&format!("m{n}"), if n % 2 == 0 { SessionVoice::You } else { SessionVoice::Agent }, &format!("say {n}"))).collect();
        let tool = |n: u32| SessionRow::Tool { id: format!("t{n}"), at: Instant::carried("2026-11-10T09:01:00.000Z"), text: format!("Read file {n}"), subagent: None };
        items.push(ThreadItem::Calls { id: "c".into(), names: vec!["Read".into()], rows: (0..7).map(tool).collect() });
        let brief = report_brief("Words", &record(), &items, &from());
        let messages = brief.split("Last messages, oldest first:\n").nth(1).unwrap().split("\n\n").next().unwrap();
        assert_eq!(messages.lines().count(), 6);
        assert!(messages.starts_with("- Owner at 2026-11-10T09:00:00.000Z: say 2") && messages.ends_with("say 7"));
        let tools = brief.split("Last tool calls, oldest first:\n").nth(1).unwrap();
        assert_eq!(tools.lines().count(), 5);
        assert!(tools.starts_with("- 2026-11-10T09:01:00.000Z Read file 2"));
        assert!(!brief.contains("Last timeline events"));
    }
}
