//! The rows a Job's lists share (`LedgerRow.tsx`, `SessionLayers.tsx`'s `Row`): the kind's glyph in
//! the kind's hue and a title that wraps to two lines, then a line of chips: the path or type in mono,
//! the kind's word, and the state as a filled chip. A row with somewhere to go is a button; one
//! without is read. Subagents and the Ledger draw them.

use std::rc::Rc;

use chrono::Local;
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{AttachmentState, JobState, JudgeArtifact, JudgeKind, JudgeState, SessionRow, ThreadItem, ToolSubagent, ToolSubagentState, TouchAct, WaitingActKind};

use super::rows::{age, clock, glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// Where a row's press goes.
pub(super) type Opens = Rc<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>;

/// A row's state, as the ledger's marks draw it (`SessionLedgerTone`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    Pending,
    Passed,
    Failed,
    Running,
    Done,
    Escalated,
    Piloted,
    Stopped,
}

impl Tone {
    /// Its glyph (`TONE`), the hue of its chip (`MARK_HUE`) and its word (`TONE_WORD`).
    fn look(self) -> (&'static str, Token, &'static str) {
        match self {
            Tone::Pending => ("shield-ellipsis", token::STATUS_AWAITING_REVIEW, "Waiting"),
            Tone::Passed => ("shield-check", token::STEP_ADVANCED, "Passed"),
            Tone::Failed => ("shield-x", token::STEP_FAILED, "Failed"),
            Tone::Running => ("circle-dot", token::STEP_RUNNING, "Running"),
            Tone::Done => ("check", token::STEP_ADVANCED, "Done"),
            Tone::Escalated => ("megaphone", token::STATUS_AWAITING_REVIEW, "Waiting"),
            Tone::Piloted => ("terminal", token::STATUS_AWAITING_REVIEW, "Piloted"),
            Tone::Stopped => ("power", token::STATUS_NOT_STARTED, "Stopped"),
        }
    }
}

/// A row's state: its tone, what its tooltip says, and a word that stands in for the tone's own.
pub(super) struct Mark {
    pub tone: Tone,
    pub said: String,
    pub word: Option<String>,
}

/// A row's one act, in a column of its own (`ReopenAct`). Where `run` is None the column stays empty.
pub(super) struct Act {
    pub name: String,
    pub run: Option<Opens>,
}

/// One row of a Job's lists (`ArtifactRow`).
#[derive(Default)]
pub(super) struct Line {
    pub key: String,
    /// The artifact kind, which gives the glyph its hue.
    pub kind: &'static str,
    pub glyph: &'static str,
    /// What the glyph is: its tooltip, and the kind's word on the second line.
    pub said: &'static str,
    pub title: String,
    pub sub: Option<String>,
    pub words: Vec<String>,
    /// When, as a chip in the kind's hue before the words (`Stamp`).
    pub stamp: Option<String>,
    pub mark: Option<Mark>,
    pub opens: Option<Opens>,
    pub act: Option<Act>,
    /// A pull request, which draws as its own row (`PullRow`).
    pub pull: Option<super::pulls::Pull>,
    /// A linked Job's pull request, drawn as its Merged or Open and Checks facts before the mark.
    pub linked: Option<super::pulls::Pull>,
}

/// One group of the Ledger: its heading and its rows.
struct Group {
    id: &'static str,
    label: &'static str,
    lines: Vec<Line>,
}

/// The Commands group lists this many before "View all".
const COMMANDS_SHOWN: usize = 10;

/// Each kind's own hue (`ledger-row.css`).
fn hue(kind: &str) -> Token {
    match kind {
        "pull_request" => token::STEP_RUNNING,
        "issue" | "link" | "session" => token::ACCENT,
        "commit" => token::STEP_ADVANCED,
        "page" => token::TOOL_LOOK,
        "file" => token::TOOL_CHANGE,
        "doc" => token::STATUS_HANDED_IN,
        "sketch" => token::STATUS_ESCALATED,
        "subagent" | "studio" => token::STATUS_REJECTED,
        "command" => token::TOOL_RUN,
        _ => token::FG_MUTED,
    }
}

/// What a judge reads in a row, from its key and group (`artifactOfRow`). None for a row with nothing to
/// read: a linked Job, a closed item, a link, a studio.
fn judged_of(key: &str, title: &str, group: &str) -> Option<JudgeArtifact> {
    let (prefix, id) = key.split_once(':')?;
    let kind = match prefix {
        "pr" => JudgeKind::PullRequest,
        "commit" => JudgeKind::Commit,
        "artifact" => match group {
            "page" => JudgeKind::Page,
            "file" => JudgeKind::File,
            "doc" => JudgeKind::Doc,
            _ => return None,
        },
        "touched" => JudgeKind::File,
        "sketch" => JudgeKind::Sketch,
        "ran" | "bash" | "background" => JudgeKind::Command,
        "subagent" => JudgeKind::Subagent,
        "issue" => JudgeKind::Issue,
        "note" => JudgeKind::Note,
        _ => return None,
    };
    let title = if kind == JudgeKind::PullRequest { format!("#{id}") } else { title.to_string() };
    Some(JudgeArtifact { kind, id: id.to_string(), title })
}

/// The subagent a ledger row names, as the thread's Agent call would carry it, for its sheet.
pub(super) fn subagent_of(one: &ipc::Attachment) -> ToolSubagent {
    let standing = one.state == AttachmentState::Standing;
    ToolSubagent {
        id: one.target.clone(),
        agent_type: one.detail.get("type").filter(|kind| !kind.is_empty()).cloned(),
        task: one.detail.get("task").or_else(|| one.detail.get("description")).cloned().unwrap_or_else(|| "Subagent".to_string()),
        latest: None,
        state: if standing { ToolSubagentState::Running } else { ToolSubagentState::Finished },
        since: one.since.clone(),
        ended: (!standing).then(|| one.changed_at.clone()),
        // A ledger row carries no report; the sheet reads the subagent's own.
        report: None,
    }
}

/// What a Job holds on its ledger: its attachments, less those given back, which only a subagent, an artifact and a background task outlive.
fn held(record: &ipc::SessionRecord) -> impl Iterator<Item = &ipc::Attachment> {
    record.attachments.iter().filter(|one| one.state != AttachmentState::GivenBack || matches!(one.kind.as_str(), "subagent" | "artifact" | "background"))
}

fn detail<'a>(one: &'a ipc::Attachment, key: &str) -> Option<&'a str> {
    one.detail.get(key).map(String::as_str)
}

/// A page, file or doc the Job made, as its row reads it.
pub(super) fn artifact_line(one: &ipc::Attachment, kind: &'static str) -> Line {
    let (glyph, said) = match detail(one, "form") {
        Some("page") => ("globe", "Published page"),
        Some("file") => ("files", "File written"),
        Some("image") => ("image", "Looked at"),
        Some("doc") => ("notebook-text", "Doc"),
        Some("document") => ("file-text", "Document"),
        _ => ("app-window", "Shown in a window"),
    };
    // A file the agent showed and did not write.
    let shown_only = detail(one, "form") == Some("file") && detail(one, "by") == Some("agent") && detail(one, "shown").is_some() && detail(one, "written") != Some("true");
    Line {
        key: format!("artifact:{}", one.target),
        kind,
        glyph,
        said: if shown_only { "File shown" } else { said },
        title: detail(one, "title").map(str::to_string).unwrap_or_else(|| one.target.rsplit('/').next().unwrap_or_default().to_string()),
        sub: Some(one.target.clone()),
        ..Line::default()
    }
}

/// A Job's state as the ledger's marks say it (`TONE`).
fn tone_of(state: JobState) -> Tone {
    match state {
        JobState::Working | JobState::Running | JobState::Subagent => Tone::Running,
        JobState::Stuck | JobState::Failing | JobState::Unstarted => Tone::Failed,
        JobState::Waiting => Tone::Escalated,
        JobState::Replied | JobState::Done => Tone::Done,
        JobState::Ended | JobState::Quiet | JobState::Stopped | JobState::StoppedAsks => Tone::Stopped,
        JobState::Idle | JobState::Blank | JobState::Limited | JobState::After | JobState::Draft | JobState::Held | JobState::Paused => Tone::Pending,
    }
}

/// The message of a `git commit -m "..."` call (`commitOf`): the first quoted words after `-m` or `--message` on the command's first line.
pub(super) fn commit_of(text: &str) -> Option<String> {
    let rest = text.strip_prefix("Bash").filter(|rest| rest.starts_with(char::is_whitespace))?.trim_start();
    let tail = rest.strip_prefix("git commit").filter(|tail| !tail.starts_with(|next: char| next.is_alphanumeric() || next == '_'))?;
    for (at, _) in tail.char_indices() {
        if tail[..at].contains('\n') {
            break;
        }
        let here = &tail[at..];
        let Some(after) = here.strip_prefix("--message").or_else(|| here.strip_prefix("-m")).filter(|after| after.starts_with(char::is_whitespace)) else { continue };
        let after = after.trim_start();
        for quote in ['"', '\''] {
            if let Some(end) = after.strip_prefix(quote).and_then(|body| body.find(quote).filter(|end| *end > 0).map(|end| &body[..end])) {
                return Some(end.to_string());
            }
        }
    }
    None
}

/// A web address as its row titles it: the host and the path, with no scheme, query or trailing slash (`linkTitle`).
fn link_title(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://").filter(|(scheme, _)| !scheme.is_empty() && scheme.chars().all(|one| one.is_ascii_alphanumeric() || matches!(one, '+' | '-' | '.'))) else { return url.to_string() };
    let (authority, tail) = rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
    let host = authority.rsplit('@').next().unwrap_or(authority).to_lowercase();
    if host.is_empty() {
        return url.to_string();
    }
    let _ = scheme;
    let path = tail.split(['?', '#']).next().unwrap_or_default();
    let path = if path == "/" { "" } else { path.strip_suffix('/').unwrap_or(path) };
    format!("{host}{path}")
}

impl JobView {
    /// A Job's subagents as rows, running ones first, then in the ledger's order (`artifactsOf`).
    pub(super) fn subagent_lines(&self, id: &str, cx: &gpui::App) -> Vec<Line> {
        let Some(record) = self.record(id, cx) else { return Vec::new() };
        let mut held: Vec<&ipc::Attachment> = record.attachments.iter().filter(|one| one.kind == "subagent").collect();
        held.sort_by_key(|one| one.state != AttachmentState::Standing);
        held.into_iter()
            .map(|one| {
                let subagent = subagent_of(one);
                let running = subagent.state == ToolSubagentState::Running;
                let (tone, said) = if running { (Tone::Running, "Running") } else { (Tone::Done, "Done") };
                let title = subagent.task.clone();
                let sub = subagent.agent_type.clone();
                let opens: Opens = Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.read_subagent(subagent.clone(), window, cx));
                Line {
                    key: format!("subagent:{}", one.target),
                    kind: "subagent",
                    glyph: "split",
                    said: "Subagent",
                    title,
                    sub,
                    words: Vec::new(),
                    stamp: None,
                    mark: Some(Mark { tone, said: said.to_string(), word: None }),
                    opens: Some(opens),
                    act: None,
                    pull: None,
                    linked: None,
                }
            })
            .collect()
    }

    /// Subagents: every subagent the Job started, as one list with no heading (`SubagentsLayer`).
    /// None, and nothing is drawn.
    pub(super) fn pane_subagents(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let lines = self.subagent_lines(id, cx);
        if lines.is_empty() {
            return None;
        }
        Some(labelled(div().id("pane-subagents").flex().flex_col().children(self.ledger_rows(lines, "subagent", look, cx)), Role::List, "Subagents").into_any_element())
    }

    /// The Ledger: everything attached to the Job, by kind (`LedgerBody`). A kind with nothing draws no heading;
    /// a Job with nothing draws no body.
    pub(super) fn pane_ledger(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let groups = self.ledger_groups(id, cx);
        if groups.is_empty() {
            return None;
        }
        let groups = groups.into_iter().map(|group| self.ledger_group(id, group, look, cx));
        Some(div().id("pane-ledger").flex().flex_col().gap(look.length(token::SPACE_4)).children(groups).into_any_element())
    }

    /// One group: its heading over its rows, and for Commands the cap with its "View all" (`Group`).
    fn ledger_group(&self, id: &SharedString, group: Group, look: &Look, cx: &Context<Self>) -> AnyElement {
        let total = group.lines.len();
        let capped = group.id == "command" && total > COMMANDS_SHOWN;
        let all = self.panes.commands_all.contains(id);
        let mut lines = group.lines;
        if capped && !all {
            // A row a judge left notes under is drawn past the cap, so the notes stand where the reading leads.
            let record = self.record(id, cx);
            let noted = |line: &Line| {
                judged_of(&line.key, &line.title, group.id)
                    .and_then(|artifact| record.and_then(|record| super::judge::reading_of(record, artifact.kind, &artifact.id)))
                    .is_some_and(|reading| reading.state == JudgeState::Done && !reading.notes.is_empty())
            };
            let mut at = 0;
            lines.retain(|line| {
                at += 1;
                at <= COMMANDS_SHOWN || noted(line)
            });
        }
        let toggle = capped.then(|| {
            let job = id.clone();
            let said = if all { "Show fewer".to_string() } else { format!("View all {total}") };
            labelled(
                div()
                    .id("ledger-commands-all")
                    .self_start()
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .child(SharedString::from(said.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.panes.commands_all.remove(&job) {
                            this.panes.commands_all.insert(job.clone());
                        }
                        cx.notify();
                    })),
                Role::Button,
                said,
            )
        });
        labelled(
            div()
                .id(SharedString::from(format!("ledger-group:{}", group.id)))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px(look.length(token::SPACE_2))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(group.label),
                )
                .child(div().flex().flex_col().children(self.ledger_rows(lines, group.id, look, cx)))
                .children(toggle),
            Role::Group,
            group.label,
        )
        .into_any_element()
    }

    /// A time as the thread's stamps read it: the clock, or its age once a press has flipped them.
    fn when(&self, at: &str) -> String {
        let now = Local::now();
        let (on_clock, ago) = (clock(at, now), age(at, now));
        (if self.aged { ago } else { on_clock }).unwrap_or_else(|| at.to_string())
    }

    /// The rows of the thread a Job holds, oldest first: the ones drawn as they came, the calls and a Fleet turn's.
    pub(super) fn thread_rows(&self, id: &str, cx: &gpui::App) -> Vec<SessionRow> {
        let Some(held) = self.thread(id, cx) else { return Vec::new() };
        let mut rows = Vec::new();
        for item in &held.items {
            match item {
                ThreadItem::Row { row } => rows.push(row.clone()),
                ThreadItem::Calls { rows: calls, .. } => rows.extend(calls.iter().cloned()),
                ThreadItem::FleetTurn { rows: turn, .. } => rows.extend(turn.iter().cloned()),
                _ => {}
            }
        }
        rows
    }

    /// The card a sketch of the ledger is drawn as in the thread, newest first, for its press.
    pub(super) fn sketch_card(&self, id: &str, target: &str, cx: &gpui::App) -> Option<String> {
        let held = self.thread(id, cx)?;
        let from_items = held.items.iter().rev().find_map(|item| match item {
            ThreadItem::Sketch(card) if card.sketch == target => Some(card.id.clone()),
            _ => None,
        });
        from_items.or_else(|| held.head.sketch.as_ref().filter(|card| card.sketch == target).map(|card| card.id.clone()))
    }

    /// Everything a Job holds, as Armada Classic's `artifactsOf` groups it, in its order. A group with no row is left out.
    fn ledger_groups(&self, id: &SharedString, cx: &gpui::App) -> Vec<Group> {
        let Some(record) = self.record(id, cx) else { return Vec::new() };
        let mut groups: Vec<Group> = Vec::new();
        let mut add = |id: &'static str, label: &'static str, lines: Vec<Line>| {
            if !lines.is_empty() {
                groups.push(Group { id, label, lines });
            }
        };
        let rows = self.thread_rows(id, cx);
        let of = |kind: &'static str| held(record).filter(move |one| one.kind == kind);
        let made: Vec<&ipc::Attachment> = of("artifact").collect();
        let form = |one: &&ipc::Attachment| detail(one, "form").unwrap_or_default().to_string();
        let newest = |forms: &[&str]| -> Vec<&ipc::Attachment> { made.iter().rev().filter(|one| forms.contains(&form(one).as_str())).copied().collect() };

        // The Jobs this one started, with the plan group each was handed under it.
        let jobs = of("started_to")
            .filter_map(|one| {
                let child = self.record(&one.target, cx)?;
                let sub = detail(one, "group").and_then(|group| record.plan.as_ref()?.groups.iter().find(|one| one.id == group)).map(|group| group.title.clone());
                let mark = child.mark.map(|mark| Mark { tone: tone_of(mark.state), said: crate::jobs::glyph_of(mark.state, None).1.to_string(), word: None });
                let child_id = child.id.as_str().to_string();
                let opens: Opens = Rc::new(move |this: &mut JobView, _: &mut Window, cx: &mut Context<JobView>| this.open_job(child_id.clone(), cx));
                Some(Line {
                    key: format!("session:{}", child.id.as_str()),
                    kind: "session",
                    glyph: "link",
                    said: "Linked Job",
                    title: child.title.clone().unwrap_or_else(|| child.id.as_str().to_string()),
                    sub,
                    mark,
                    opens: Some(opens),
                    linked: super::pulls::pull_of_child(child),
                    ..Line::default()
                })
            })
            .collect();
        add("session", "Jobs", jobs);

        let pulls = record.attachments.iter().filter_map(|one| super::pulls::pull_of(record, one, None)).map(super::pulls::pull_line).collect::<Vec<_>>();
        add("pull_request", "Pull requests", pulls.into_iter().rev().collect());

        // What he closed from Needs you, newest first, and Reopen while its thing still stands.
        let mut closed: Vec<&ipc::ClosedWaiting> = record.closed.iter().filter(|one| !self.panes.reopened.contains(&format!("{id}\0{}", one.item.id))).collect();
        closed.sort_by(|a, b| b.at.as_str().cmp(a.at.as_str()));
        let closed = closed
            .into_iter()
            .map(|one| {
                let (glyph, said) = match one.item.act.as_ref().map(|act| act.kind) {
                    Some(WaitingActKind::Walk) => ("app-window", "Walk"),
                    Some(WaitingActKind::Answer) => ("message-circle-question-mark", "Question"),
                    Some(WaitingActKind::ApprovePr) => ("git-pull-request-arrow", "Pull request"),
                    Some(WaitingActKind::Run) => ("terminal", "Command"),
                    Some(WaitingActKind::Message) => ("square-pen", "Message"),
                    None => ("bell-ring", "Call"),
                };
                let item_id = one.item.id.clone();
                let run: Option<Opens> = (!one.ended).then(|| -> Opens { Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.reopen_closed(item_id.clone(), window, cx)) });
                Line {
                    key: format!("closed{}", one.item.id),
                    kind: "closed",
                    glyph,
                    said,
                    title: one.item.text.clone(),
                    stamp: Some(self.when(one.at.as_str())),
                    act: Some(Act { name: one.item.text.clone(), run }),
                    ..Line::default()
                }
            })
            .collect();
        add("closed", "Closed", closed);

        let issues = of("issue")
            .map(|one| {
                let label = if one.target.bytes().all(|byte| byte.is_ascii_digit()) && !one.target.is_empty() { format!("#{}", one.target) } else { one.target.clone() };
                let item = super::issues::issue_of(record, one);
                let opens: Opens = Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.open_issue(item.clone(), window, cx));
                Line {
                    key: format!("issue:{}", one.target),
                    kind: "issue",
                    glyph: "ticket",
                    said: "Issue",
                    title: format!("{label} {}", detail(one, "title").unwrap_or_default()).trim_end().to_string(),
                    mark: (detail(one, "state") == Some("closed")).then(|| Mark { tone: Tone::Done, said: "Closed".to_string(), word: None }),
                    opens: Some(opens),
                    ..Line::default()
                }
            })
            .collect();
        add("issue", "Issues", issues);

        // The ledger holds no commit of its own: they are the thread's `git commit` lines.
        let commits: Vec<Line> = rows
            .iter()
            .filter_map(|row| {
                let SessionRow::Tool { id, at, text, .. } = row else { return None };
                Some(Line { key: format!("commit:{id}"), kind: "commit", glyph: "git-commit-horizontal", said: "Commit", title: commit_of(text)?, sub: Some(self.when(at.as_str())), ..Line::default() })
            })
            .collect();
        add("commit", "Commits", commits.into_iter().rev().collect());

        add("page", "Pages", newest(&["page", "window"]).into_iter().map(|one| artifact_line(one, "page")).collect());

        let written = newest(&["file", "image"]);
        let named: Vec<&str> = written.iter().map(|one| one.target.as_str()).collect();
        let mut files: Vec<Line> = written.iter().map(|one| artifact_line(one, "file")).collect();
        // What its drones touched: each file with what was last done to it.
        let mut touches: Vec<&ipc::FileTouch> = record.touches.iter().collect();
        touches.sort_by(|a, b| a.at.as_str().cmp(b.at.as_str()));
        let mut touched: Vec<(&str, TouchAct)> = Vec::new();
        for touch in touches {
            match touched.iter_mut().find(|(path, _)| *path == touch.path) {
                Some(one) => one.1 = touch.act,
                None => touched.push((touch.path.as_str(), touch.act)),
            }
        }
        files.extend(touched.into_iter().filter(|(path, _)| !named.contains(path)).map(|(path, act)| Line {
            key: format!("touched:{path}"),
            kind: "file",
            glyph: "files",
            said: "File touched",
            title: path.to_string(),
            sub: Some(match act {
                TouchAct::Write => "Written",
                TouchAct::Edit => "Edited",
                TouchAct::Remove => "Removed",
            }
            .to_string()),
            ..Line::default()
        }));
        add("file", "Files", files);
        add("doc", "Docs", newest(&["doc", "document"]).into_iter().map(|one| artifact_line(one, "doc")).collect());

        // A sketch opens its drawing where the thread holds its card.
        let sketches = of("sketch")
            .chain(made.iter().copied().filter(|one| detail(one, "form") == Some("sketch")))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|one| {
                let card = self.sketch_card(id, &one.target, cx);
                let opens: Option<Opens> = card.map(|card| -> Opens { Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.read_sketch(&card, &card, window, cx)) });
                Line {
                    key: format!("sketch:{}", one.target),
                    kind: "sketch",
                    glyph: "pencil-ruler",
                    said: "Sketch",
                    title: detail(one, "title").filter(|title| !title.is_empty()).map(str::to_string).unwrap_or_else(|| one.target.trim_start_matches("sketch:").to_string()),
                    sub: Some("Drawn by the agent".to_string()),
                    opens,
                    ..Line::default()
                }
            })
            .collect();
        add("sketch", "Sketches", sketches);

        // A note he sent: the words, the part of the screen it was on and where it stands.
        let notes = newest(&["note"])
            .into_iter()
            .filter_map(|one| {
                let (note, text) = (detail(one, "note")?, detail(one, "text")?);
                detail(one, "tag")?;
                detail(one, "sent")?;
                let (tone, word) = match detail(one, "status")? {
                    "open" => (Tone::Pending, "Open"),
                    "done" => (Tone::Done, "Done"),
                    "returned" => (Tone::Stopped, "Returned"),
                    _ => return None,
                };
                Some(Line {
                    key: format!("note:{note}"),
                    kind: "note",
                    glyph: "message-square-text",
                    said: "Note",
                    title: text.to_string(),
                    sub: detail(one, "component").map(str::to_string),
                    words: detail(one, "screen").map(str::to_string).into_iter().collect(),
                    mark: Some(Mark { tone, said: word.to_string(), word: Some(word.to_string()) }),
                    ..Line::default()
                })
            })
            .collect();
        add("note", "Notes", notes);

        let links = of("link")
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|one| {
                let url = one.target.clone();
                let opens: Opens = Rc::new(move |_: &mut JobView, _: &mut Window, cx: &mut Context<JobView>| cx.open_url(&url));
                Line {
                    key: format!("link:{}", one.target),
                    kind: "link",
                    glyph: "external-link",
                    said: "Link",
                    title: link_title(&one.target),
                    sub: Some(if detail(one, "by") == Some("you") { "You" } else { "Agent" }.to_string()),
                    opens: Some(opens),
                    ..Line::default()
                }
            })
            .collect();
        add("link", "Links", links);

        add("subagent", "Subagents", self.subagent_lines(id, cx));

        // Background tasks first, then what the agent ran in the thread, newest first.
        let mut commands: Vec<Line> = of("background")
            .filter_map(|one| {
                let form = detail(one, "form").filter(|form| matches!(*form, "command" | "monitor"))?;
                let exit: Option<i64> = detail(one, "exit").and_then(|code| code.parse().ok());
                let (tone, said) = if one.state == AttachmentState::Standing {
                    (Tone::Running, "Running".to_string())
                } else if one.state == AttachmentState::GivenBack || detail(one, "status") == Some("killed") {
                    (Tone::Stopped, "Stopped".to_string())
                } else if detail(one, "status") == Some("failed") {
                    (Tone::Failed, exit.map_or_else(|| "Failed".to_string(), |code| format!("Exit {code}")))
                } else {
                    (Tone::Done, format!("Exit {}", exit.unwrap_or(0)))
                };
                let (glyph, kind) = if form == "monitor" { ("radar", "Monitor") } else { ("square-terminal", "Background command") };
                Some(Line {
                    key: format!("background:{}", one.target),
                    kind: "command",
                    glyph,
                    said: kind,
                    title: detail(one, "task").or_else(|| detail(one, "command")).unwrap_or("Background task").to_string(),
                    sub: detail(one, "command").filter(|command| !command.is_empty()).map(str::to_string),
                    mark: Some(Mark { tone, said, word: None }),
                    ..Line::default()
                })
            })
            .collect();
        let mut ran: Vec<Line> = rows
            .iter()
            .filter_map(|row| match row {
                SessionRow::Ran { id, command, code, .. } => Some(Line {
                    key: format!("ran:{id}"),
                    kind: "command",
                    glyph: "square-terminal",
                    said: "Command run",
                    title: command.clone(),
                    sub: match code {
                        None => Some("Stopped".to_string()),
                        Some(0) => None,
                        Some(code) => Some(format!("Exit {code}")),
                    },
                    ..Line::default()
                }),
                SessionRow::Tool { id, text, .. } if text.starts_with("Bash ") && commit_of(text).is_none() => {
                    Some(Line { key: format!("bash:{id}"), kind: "command", glyph: "square-terminal", said: "Command run", title: text[5..].to_string(), ..Line::default() })
                }
                _ => None,
            })
            .collect();
        ran.reverse();
        commands.extend(ran);
        add("command", "Commands", commands);

        let studios = of("studio")
            .map(|one| Line { key: format!("studio:{}", one.target), kind: "studio", glyph: "presentation", said: "Studio", title: detail(one, "title").unwrap_or("Studio").to_string(), ..Line::default() })
            .collect();
        add("studio", "Studios", studios);
        groups
    }

    /// Let an item he closed wait again. It leaves the list at once; Fleet's answer brings it back where it refused.
    fn reopen_closed(&mut self, item_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let mark = format!("{job}\0{item_id}");
        self.panes.reopened.insert(mark.clone());
        cx.notify();
        let body = ipc::encode(&ipc::ReopenWaiting { session_id: ipc::SessionId::carried(job.to_string()), item_id });
        self.post(
            "/sessions/waiting/reopen".to_string(),
            body,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if answer.is_err() {
                    this.panes.reopened.remove(&mark);
                    cx.notify();
                }
            }),
        );
    }

    /// Rows one under another, a hairline between them. `group` says what a judge would read in each.
    pub(super) fn ledger_rows(&self, lines: Vec<Line>, group: &str, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        lines.into_iter().enumerate().map(|(at, line)| self.ledger_row(line, group, at == 0, look, cx)).collect()
    }

    /// A row's Judge, and what the last reading of it found, drawn under the row (`JudgeRowNotes`), with
    /// what Fleet said where it refused.
    pub(super) fn row_judge(&self, line: &Line, group: &str, look: &Look, cx: &Context<Self>) -> Option<(AnyElement, Option<AnyElement>)> {
        let artifact = judged_of(&line.key, &line.title, group)?;
        let job = self.shown.clone()?;
        let page = self.pages.get(&job)?;
        let reading = self.record(&job, cx).and_then(|record| super::judge::reading_of(record, artifact.kind, &artifact.id));
        let key = format!("ledger:{}", line.key);
        // A pull request's notes stand in its panel's Files tab, by the lines they name.
        let under_row = artifact.kind != JudgeKind::PullRequest;
        let press = self.judge_press(super::acts::SHEET, &key, artifact, reading, page, look, cx);
        let notes = reading
            .filter(|_| under_row)
            .and_then(|reading| super::judge::Pinned::whole_job(super::acts::SHEET, &key, reading))
            .and_then(|at| self.notes_list(&at, at.whole(), "Notes from the judge".to_string(), page, look, cx));
        let refused = page.cards.said.get(&key).map(|why| super::acts::refusal(look, &key, why.clone()));
        let under = (notes.is_some() || refused.is_some())
            .then(|| div().flex().flex_col().gap(look.length(token::SPACE_2)).pl(look.length(token::SPACE_2) * 2. + look.length(token::SPACE_3)).pr(look.length(token::SPACE_2)).pb(look.length(token::SPACE_2)).children(notes).children(refused).into_any_element());
        Some((press, under))
    }

    fn ledger_row(&self, mut line: Line, group: &str, first: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        if let Some(pull) = line.pull.take() {
            return self.pull_row(line, pull, group, look, cx);
        }
        let judged = self.row_judge(&line, group, look, cx);
        let ink = look.color(hue(line.kind));
        let chip = |words: SharedString| div().flex_none().px(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_SM)).child(words);
        let sub = line.sub.map(|sub| {
            div()
                .min_w_0()
                .truncate()
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                .font_family(mono(look))
                .child(SharedString::from(sub))
        });
        let stamp = line.stamp.map(|when| chip(when.into()).bg(ink.opacity(0.14)).text_color(ink).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))));
        let words = line.words.into_iter().map(|word| div().min_w_0().truncate().child(SharedString::from(word)));
        let kind_word = chip(line.said.into()).bg(ink.opacity(0.14)).text_color(ink).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)));
        let linked = line.linked.as_ref().map(|pull| super::pulls::linked_facts(look, &line.key, pull)).into_iter().flatten();
        let fact = line.mark.map(|mark| {
            let (icon, tone, word) = mark.tone.look();
            let fact = look.color(tone);
            div()
                .id(SharedString::from(format!("ledger-fact:{}", line.key)))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .pl(look.length(token::SPACE_1))
                .pr(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .bg(fact.opacity(0.14))
                .text_color(fact)
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .tooltip(tip(mark.said.clone()))
                .child(glyph(look, icon, Size::Small, fact))
                .child(SharedString::from(mark.word.unwrap_or_else(|| word.to_string())))
        });
        let lead = div()
            .id(SharedString::from(format!("ledger-glyph:{}", line.key)))
            .flex_none()
            .w(look.length(token::SPACE_3))
            .h(look.length(token::LEADING_XS))
            .flex()
            .items_center()
            .justify_center()
            .tooltip(tip(line.said))
            .child(glyph(look, line.glyph, Size::Small, ink));
        let main = div()
            .id(SharedString::from(format!("ledger-row:{}", line.key)))
            .flex_1()
            .min_w_0()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .child(lead)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().min_w_0().line_clamp(2).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(line.title.clone())))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .min_w_0()
                            .overflow_hidden()
                            .text_size(look.length(token::TEXT_2XS))
                            .line_height(look.length(token::LEADING_2XS))
                            .text_color(look.color(token::FG_MUTED))
                            .whitespace_nowrap()
                            .children(sub)
                            .children(stamp)
                            .children(words)
                            .child(kind_word)
                            .children(linked)
                            .children(fact),
                    ),
            );
        let main = match line.opens {
            Some(opens) => labelled(
                main.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| opens(this, window, cx))),
                Role::Button,
                line.title,
            )
            .into_any_element(),
            None => main.into_any_element(),
        };
        let act = line.act.map(|act| {
            let place = div().flex_none().size(look.length(token::H_CONTROL_SM));
            match act.run {
                Some(run) => labelled(
                    place
                        .id(SharedString::from(format!("ledger-act:{}", line.key)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(look.length(token::RADIUS_MD))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .tooltip(tip("Reopen"))
                        .child(glyph(look, "undo-2", Size::Small, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, window, cx| run(this, window, cx))),
                    Role::Button,
                    format!("Reopen {}", act.name),
                )
                .into_any_element(),
                None => place.into_any_element(),
            }
        });
        // Every row keeps its act column: Reopen on a closed item, else the Judge where there is something to read, else empty.
        let (press, under) = judged.map_or((None, None), |(press, under)| (Some(press), under));
        let act = act.or(press).unwrap_or_else(|| div().flex_none().size(look.length(token::H_CONTROL_SM)).into_any_element());
        div()
            .flex()
            .flex_col()
            .when(!first, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
            .child(div().flex().items_center().gap(look.length(token::SPACE_1)).child(main).child(act))
            .children(under)
            .into_any_element()
    }
}
