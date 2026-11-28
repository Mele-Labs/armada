//! What typing in the message box offers (`SessionComposer.tsx`, `tokenAt`, section 4.4): `/` at the
//! start lists the agent's commands, `@` at the start of a word lists other Jobs and then files, and
//! `#` lists the repository's issues and pull requests.
//!
//! The list hangs over the box. ↑ and ↓ move, Enter and Tab choose, Esc closes. A Job or a number
//! chosen becomes one atomic chip in the text; a file or a command is plain text.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{div, prelude::*, svg, Context, Role, ScrollHandle, SharedString, Window};
use gpui_component::input::{InlineToken, TextareaState};
use ipc::TagKind;

use super::JobView;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What choosing a row does.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Pick {
    Command(String),
    File(String),
    Tag { kind: TagKind, id: String, title: String, url: Option<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    pub group: Option<&'static str>,
    pub glyph: &'static str,
    pub name: String,
    pub says: String,
    /// The project an issue is from, where the list reads several projects' trackers: drawn at the row's right.
    pub project: Option<String>,
    pub pick: Pick,
}

/// What the list holds between frames: where it is scrolled, and whether a press outside it is waiting to put it away.
#[derive(Clone, Default)]
pub(crate) struct Held {
    pub scroll: ScrollHandle,
    pub away: Rc<Cell<bool>>,
}

pub(crate) struct Offer {
    pub label: &'static str,
    pub rows: Vec<Row>,
    pub current: usize,
    /// The word being typed, as a byte range of the text.
    pub from: usize,
    pub to: usize,
    pub query: String,
    /// The files have not been answered for yet.
    pub loading: bool,
    /// The tools list one level in, on API Client's saved requests.
    pub within: Option<super::ask_tools::Within>,
    pub held: Held,
}

impl Offer {
    /// Scroll the list so the marked row is in view. The headings and the search are children of the list too, so the row's
    /// place among them is counted; the first row brings the top with it.
    pub fn keep_marked_in_view(&self) {
        if self.current == 0 {
            self.held.scroll.scroll_to_top_of_item(0);
            return;
        }
        let headings = self.rows.iter().take(self.current + 1).filter(|row| row.group.is_some()).count();
        let search = if self.within.is_some() { 2 } else { 0 };
        self.held.scroll.scroll_to_item(self.current + headings + search);
    }
}

/// The word under the caret and its trigger (`tokenAt`): `/` only at the very start, `@` and `#` at
/// the start of a word. Its byte range and the text after the trigger.
pub(crate) fn token_at(text: &str, cursor: usize) -> Option<(char, usize, String)> {
    let cursor = cursor.min(text.len());
    if !text.is_char_boundary(cursor) {
        return None;
    }
    let start = text[..cursor].rfind(char::is_whitespace).map_or(0, |at| at + text[at..].chars().next().map_or(1, char::len_utf8));
    let word = &text[start..cursor];
    let first = word.chars().next()?;
    match first {
        '/' if start == 0 && !word.contains(char::is_whitespace) => Some(('/', start, word[1..].to_string())),
        '@' => Some(('@', start, word[1..].to_string())),
        '#' if word[1..].chars().all(|ch| ch.is_alphanumeric() || ch == '-') => Some(('#', start, word[1..].to_string())),
        _ => None,
    }
}

/// Commands whose names start with the query, then those that contain it.
pub(super) fn commands_for(commands: &[String], query: &str) -> Vec<Row> {
    let query = query.to_lowercase();
    let named = |name: &String| name.trim_start_matches('/').to_lowercase();
    let starts = commands.iter().filter(|name| named(name).starts_with(&query));
    let contains = commands.iter().filter(|name| !named(name).starts_with(&query) && named(name).contains(&query));
    starts
        .chain(contains)
        .map(|name| {
            let bare = name.trim_start_matches('/').to_string();
            Row { group: None, glyph: "slash", name: format!("/{bare}"), says: String::new(), project: None, pick: Pick::Command(bare) }
        })
        .collect()
}

/// Issues and pull requests the query names, newest number first, at most six (`NUMBERED_ROWS`).
pub(crate) fn numbers_for(mut numbered: Vec<(bool, String, String, Option<String>)>, query: &str) -> Vec<Row> {
    let query = query.to_lowercase();
    numbered.retain(|(_, reference, title, _)| {
        query.is_empty() || reference.trim_start_matches('#').starts_with(&query) || reference.to_lowercase().contains(&query) || title.to_lowercase().contains(&query)
    });
    numbered.sort_by_key(|(_, reference, ..)| std::cmp::Reverse(reference.trim_start_matches('#').parse::<u64>().unwrap_or(0)));
    numbered.truncate(6);
    numbered
        .into_iter()
        .map(|(pull, reference, title, url)| {
            let id = reference.trim_start_matches('#').to_string();
            Row {
                group: None,
                glyph: if pull { "git-pull-request" } else { "ticket" },
                name: reference.clone(),
                says: title.clone(),
                project: None,
                pick: Pick::Tag {
                    kind: if pull { TagKind::PullRequest } else { TagKind::Issue },
                    id,
                    title: if title.is_empty() { reference } else { format!("{reference} {title}") },
                    url,
                },
            }
        })
        .collect()
}

/// The issues of every project's tracker the query names, newest first, at most six, each row naming its project.
pub(crate) fn project_numbers(held: &[(&str, &[ipc::TrackedIssue])], query: &str) -> Vec<Row> {
    let query = query.to_lowercase();
    let mut found: Vec<(&str, &ipc::TrackedIssue)> = held
        .iter()
        .flat_map(|(project, issues)| issues.iter().map(move |issue| (*project, issue)))
        .filter(|(_, issue)| {
            query.is_empty()
                || issue.reference.trim_start_matches('#').starts_with(&query)
                || issue.reference.to_lowercase().contains(&query)
                || issue.title.to_lowercase().contains(&query)
        })
        .collect();
    found.sort_by(|(_, a), (_, b)| b.updated_at.cmp(&a.updated_at));
    found.truncate(6);
    found
        .into_iter()
        .map(|(project, issue)| {
            let id = issue.reference.trim_start_matches('#').to_string();
            Row {
                group: None,
                glyph: "ticket",
                name: issue.reference.clone(),
                says: issue.title.clone(),
                project: Some(project.to_string()),
                pick: Pick::Tag { kind: TagKind::Issue, id, title: format!("{} {}", issue.reference, issue.title), url: Some(issue.url.clone()) },
            }
        })
        .collect()
}

/// The key a project's issues are held under in the catalog: no manifest id holds a unit separator, so it clashes with none.
fn project_key(project: &str) -> String {
    format!("\u{1f}project\u{1f}{project}")
}

/// A tag as the text holds it: `@<title>`, with the kind and id kept in the chip's own id.
pub(crate) fn chip_of(kind: TagKind, id: &str, title: &str, url: Option<&str>) -> InlineToken {
    let kind = match kind {
        TagKind::Session => "session",
        TagKind::PullRequest => "pull_request",
        TagKind::Branch => "branch",
        TagKind::Issue => "issue",
        TagKind::Report => "report",
        TagKind::Terminal => "terminal",
        TagKind::Tool => "tool",
        TagKind::Request => "request",
    };
    let one_line = title.replace(['\n', '\r'], " ");
    // A request's chip reads its tool and its name; the words it stands for are `@<name>`, which the thread finds it by.
    let label = if kind == "request" { super::ask_tools::request_words(&one_line) } else { one_line.clone() };
    InlineToken::new(format!("{kind}\u{1f}{id}\u{1f}{}", url.unwrap_or_default()), format!("@{one_line}")).with_label(label)
}

/// The mention a chip stands for, read back from its id and label.
pub(crate) fn mention_of(id: &str, label: &str) -> Option<ipc::SessionTag> {
    let mut parts = id.split('\u{1f}');
    let kind = match parts.next()? {
        "session" => TagKind::Session,
        "pull_request" => TagKind::PullRequest,
        "branch" => TagKind::Branch,
        "issue" => TagKind::Issue,
        "report" => TagKind::Report,
        "terminal" => TagKind::Terminal,
        "tool" => TagKind::Tool,
        "request" => TagKind::Request,
        _ => return None,
    };
    let tag_id = parts.next()?.to_string();
    let url = parts.next().filter(|url| !url.is_empty()).map(str::to_string);
    let title = if kind == TagKind::Request { super::ask_tools::request_title(label) } else { label };
    Some(ipc::SessionTag { kind, id: tag_id, title: title.to_string(), url })
}

/// What `@` offers: the Jobs whose titles hold the query, at most five, then the files `found` for it, at most five.
pub(crate) fn at_rows(jobs: &[(String, String)], found: Vec<String>, query: &str) -> Vec<Row> {
    let lowered = query.to_lowercase();
    let mut rows: Vec<Row> = jobs
        .iter()
        .filter(|(_, title)| lowered.is_empty() || title.to_lowercase().contains(&lowered))
        .take(5)
        .map(|(job, title)| Row {
            group: None,
            glyph: "square-terminal",
            name: title.clone(),
            says: String::new(),
            project: None,
            pick: Pick::Tag { kind: TagKind::Session, id: job.clone(), title: title.clone(), url: None },
        })
        .collect();
    if let Some(first) = rows.first_mut() {
        first.group = Some("Jobs");
    }
    let before = rows.len();
    rows.extend(found.into_iter().take(5).map(|path| {
        let (folder, name) = path.rsplit_once('/').map_or(("", path.as_str()), |(folder, name)| (folder, name));
        Row { group: None, glyph: "file", name: name.to_string(), says: folder.to_string(), project: None, pick: Pick::File(path.clone()) }
    }));
    if let Some(first) = rows.get_mut(before) {
        first.group = Some("Files");
    }
    rows
}

/// A row chosen puts its words in the box over `range`: a command or a file as plain text, a Job or a number as one chip.
pub(crate) fn put_pick(input: &mut TextareaState, pick: Pick, range: std::ops::Range<usize>, window: &mut Window, cx: &mut Context<TextareaState>) {
    match pick {
        Pick::Command(name) => {
            input.set_selected_range(range, cx);
            input.replace(format!("/{name} "), window, cx);
        }
        Pick::File(path) => {
            input.set_selected_range(range, cx);
            input.replace(format!("@{path} "), window, cx);
        }
        Pick::Tag { kind, id, title, url } => {
            // A tool's list replaces no word: its chip goes at the caret, a space before it where a word ends there.
            let range = if range.is_empty() && input.value()[..range.start].chars().next_back().is_some_and(|ch| !ch.is_whitespace()) {
                input.set_selected_range(range.clone(), cx);
                input.insert(" ", window, cx);
                range.start + 1..range.start + 1
            } else {
                range
            };
            if input.replace_range_with_token(range, chip_of(kind, &id, &title, url.as_deref()), window, cx).is_ok() {
                input.insert(" ", window, cx);
            }
        }
    }
}

impl JobView {
    /// The offer for what is under the caret now, or none.
    pub(super) fn refresh_offer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let (text, cursor) = {
            let input = self.composer.input.read(cx);
            (input.value().to_string(), input.cursor())
        };
        let Some((trigger, from, query)) = token_at(&text, cursor) else {
            self.composer.offer = None;
            return;
        };
        let to = cursor;
        let mut loading = false;
        let (label, rows): (&'static str, Vec<Row>) = match trigger {
            // A report's box offers no commands and no Jobs to tag, as `ReportBox` passes none.
            '/' if self.reporting.is_some() => ("Skills and commands", Vec::new()),
            '/' => {
                // The Operator's commands are its own record's alone: it has no Loadout for `/mcp` to open.
                let beside = self.is_beside(&id);
                let commands = self.record(&id, cx).and_then(|job| job.hosted.as_ref()).map(|hosted| if beside { hosted.commands.clone() } else { super::loadout_tab::with_mcp(&hosted.commands) }).unwrap_or_default();
                ("Skills and commands", commands_for(&commands, &query))
            }
            '#' => {
                let mut numbered = Vec::new();
                if let Some(manifest) = self.record(&id, cx).and_then(|job| job.manifest_id.as_ref()) {
                    if let Some(issues) = self.composer.catalog.issues.get(manifest.as_str()) {
                        numbered.extend(issues.iter().map(|issue| (false, issue.reference.clone(), issue.title.clone(), Some(issue.url.clone()))));
                    }
                }
                for job in self.list.read(cx).records() {
                    for one in job.attachments.iter().filter(|one| matches!(one.kind.as_str(), "pr" | "pull_request") && one.state == ipc::AttachmentState::Standing) {
                        let reference = format!("#{}", one.target);
                        if !numbered.iter().any(|(pull, held, ..)| *pull && *held == reference) {
                            numbered.push((true, reference, String::new(), None));
                        }
                    }
                }
                self.want_issues(&id, cx);
                let mut rows = numbers_for(numbered, &query);
                // The Operator has no repository: it reads the issues of every project's tracker, each row naming its project.
                if self.is_beside(&id) {
                    let projects = self.own_projects(cx);
                    self.want_project_issues(&projects, window, cx);
                    let kept: Vec<_> = projects.iter().filter_map(|(project, name)| Some((name.as_str(), self.composer.catalog.issues.get(&project_key(project))?.as_slice()))).collect();
                    let mut issues = project_numbers(&kept, &query);
                    issues.append(&mut rows);
                    rows = issues;
                }
                ("Tag an issue or pull request", rows)
            }
            _ => {
                let jobs: Vec<(String, String)> = self
                    .list
                    .read(cx)
                    .records()
                    .iter()
                    .filter(|job| self.reporting.is_none() && job.id.as_str() != id.as_str() && job.state != ipc::SessionState::Ended)
                    .filter_map(|job| job.title.as_ref().map(|title| (job.id.as_str().to_string(), title.clone())))
                    .collect();
                let files = self.composer.catalog.files.as_ref().filter(|(asked, _)| *asked == query).map(|(_, paths)| paths.clone());
                loading = files.is_none();
                let stale = files.or_else(|| self.composer.catalog.files.as_ref().map(|(_, paths)| paths.clone())).unwrap_or_default();
                let rows = at_rows(&jobs, stale, &query);
                self.want_files(&id, &query, window, cx);
                ("Tag a Job or a file", rows)
            }
        };
        if rows.is_empty() && !loading {
            self.composer.offer = None;
            return;
        }
        let current = self.composer.offer.as_ref().filter(|held| held.label == label).map_or(0, |held| held.current.min(rows.len().saturating_sub(1)));
        self.composer.offer = Some(Offer { label, rows, current, from, to, query, loading, within: None, held: Held::default() });
    }

    /// Ask Fleet for the files a query names, 150 ms after the last key (`LOOKUP_MS`).
    fn want_files(&mut self, id: &SharedString, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.composer.catalog.files.as_ref().is_some_and(|(asked, _)| asked == query) {
            return;
        }
        if self.fleet.is_none() {
            // Mock data: the repository's files are the fixture's, narrowed as Fleet narrows them.
            let lowered = query.to_lowercase();
            let paths = self.composer.catalog.mock_files.iter().filter(|path| path.to_lowercase().contains(&lowered)).take(50).cloned().collect();
            self.composer.catalog.files = Some((query.to_string(), paths));
            return;
        }
        let Some(manifest) = self.record(id, cx).and_then(|job| job.manifest_id.as_ref().map(|id| id.as_str().to_string())) else { return };
        let path = format!("/manifest/files?q={}&manifest_id={}", super::wire::encoded(query), super::wire::encoded(&manifest));
        let asked = query.to_string();
        self.composer.lookup = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(150)).await;
            let _ = this.update_in(cx, |this, window, cx| {
                let done: super::wire::Done = Box::new(move |this, answer, window, cx| {
                    if let Ok(bytes) = answer {
                        if let Ok(found) = ipc::decode::<ipc::FilesFound>("FilesFound", &bytes) {
                            this.composer.catalog.files = Some((asked, found.paths));
                            this.refresh_offer(window, cx);
                            cx.notify();
                        }
                    }
                });
                this.get(path, window, cx, done);
            });
        }));
    }

    /// Read the repository's issues once, for `#`.
    fn want_issues(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let Some(manifest) = self.record(id, cx).and_then(|job| job.manifest_id.as_ref().map(|id| id.as_str().to_string())) else { return };
        if self.fleet.is_none() || !self.composer.catalog.issues_asked.insert(manifest.clone()) {
            return;
        }
        let Some(port) = self.port(cx) else { return };
        let path = format!("/issues/{}", super::wire::encoded(&manifest));
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<ipc::IssueList>("IssueList", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(list) = answer {
                    this.composer.catalog.issues.insert(manifest, list.issues);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Read each project's issues once, for the Operator's `#`: the tracker the project names, as `list_project_issues` answers.
    fn want_project_issues(&mut self, projects: &[(String, String)], window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            return;
        }
        let Some(port) = self.port(cx) else { return };
        for (project, _) in projects {
            let key = project_key(project);
            if !self.composer.catalog.issues_asked.insert(key.clone()) {
                continue;
            }
            let path = format!("/projects/project/{}/issues", super::wire::encoded(project));
            cx.spawn_in(window, async move |this, cx| {
                let answer = cx
                    .background_executor()
                    .spawn(async move {
                        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                        let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                        if !(200..300).contains(&status) {
                            return Err(super::wire::reason(status, &bytes));
                        }
                        ipc::decode::<ipc::ProjectIssueList>("ProjectIssueList", &bytes).map_err(|why| why.to_string())
                    })
                    .await;
                let _ = this.update_in(cx, |this, window, cx| {
                    if let Ok(list) = answer {
                        this.composer.catalog.issues.insert(key, list.issues);
                        // The list the person is looking at takes the rows that came in.
                        if this.beside_has_keys(window, cx) {
                            this.on_beside(|this| this.refresh_offer(window, cx));
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }

    /// A key while a list is open: it is the list's, and the box does not see it. True where taken.
    pub(super) fn offer_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.tools_key(key, cx) {
            return true;
        }
        let Some(offer) = self.composer.offer.as_mut() else { return false };
        let last = offer.rows.len().saturating_sub(1);
        let was = offer.current;
        match key {
            "down" => offer.current = (offer.current + 1).min(last),
            "up" => offer.current = offer.current.saturating_sub(1),
            "escape" => self.composer.offer = None,
            "enter" | "tab" => {
                let current = offer.current;
                // A command typed whole goes as it is on Enter.
                let whole = key == "enter" && offer.rows.get(current).is_some_and(|row| matches!(&row.pick, Pick::Command(name) if *name == offer.query));
                if whole {
                    self.composer.offer = None;
                    return false;
                }
                self.choose(current, window, cx);
            }
            _ => return false,
        }
        if self.composer.offer.as_ref().is_some_and(|offer| offer.current != was) && matches!(key, "up" | "down") {
            crate::ui_sounds::moved();
        }
        if let Some(offer) = self.composer.offer.as_ref().filter(|_| matches!(key, "up" | "down")) {
            offer.keep_marked_in_view();
        }
        cx.notify();
        true
    }

    pub(super) fn choose(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(offer) = self.composer.offer.take() else { return };
        let Some(row) = offer.rows.get(at).cloned() else { return };
        let range = offer.from..offer.to;
        let input = self.composer.input.clone();
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            put_pick(input, row.pick, range, window, cx);
        });
        self.typed(cx);
    }

    /// The list over the box (`.armada-mention`): rows with a glyph, a name and its words, in groups.
    pub(super) fn render_offer(&self, look: &Look, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let offer = self.composer.offer.as_ref()?;
        let away = offer.held.away.clone();
        let this = cx.entity().downgrade();
        // A press outside the list puts it away once the press has been through every element, unless the + took it as its own.
        let outside: Box<dyn Fn(&gpui::MouseDownEvent, &mut Window, &mut gpui::App)> = Box::new(move |_, _, cx| {
            away.set(true);
            let this = this.clone();
            cx.defer(move |cx| {
                let _ = this.update(cx, |this, cx| this.leave_offer(cx));
            });
        });
        Some(offer_list(
            offer,
            look,
            |at| {
                let this = cx.entity().downgrade();
                Box::new(move |_, window, cx| {
                    let _ = this.update(cx, |this, cx| this.pressed(at, window, cx));
                })
            },
            Some(outside),
        ))
    }

    /// A press on a row: API Client's row in the tools list opens its items, as → does, and any other row is picked, as Enter does.
    fn pressed(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open_api_client_items(at, cx) {
            self.choose(at, window, cx);
        }
    }

    /// Put the list away where a press outside it is still waiting.
    fn leave_offer(&mut self, cx: &mut Context<Self>) {
        if self.composer.offer.as_ref().is_some_and(|offer| offer.held.away.get()) {
            self.composer.offer = None;
            cx.notify();
        }
    }
}

/// The list over a box (`.armada-mention`): rows with a glyph, a name and its words, in groups. `choose` makes
/// each row's press, and `outside` a press anywhere else.
pub(crate) fn offer_list(
    offer: &Offer,
    look: &Look,
    choose: impl Fn(usize) -> Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App)>,
    outside: Option<Box<dyn Fn(&gpui::MouseDownEvent, &mut Window, &mut gpui::App)>>,
) -> gpui::AnyElement {
    offer_list_at(offer, look, choose, outside, true)
}

/// The same list, floating over the box or, where `floating` is false, standing in the flow above its words: a card
/// that cuts what runs past its edge would cut a floating one.
pub(crate) fn offer_list_at(
    offer: &Offer,
    look: &Look,
    choose: impl Fn(usize) -> Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App)>,
    outside: Option<Box<dyn Fn(&gpui::MouseDownEvent, &mut Window, &mut gpui::App)>>,
    floating: bool,
) -> gpui::AnyElement {
    let mut rows: Vec<gpui::AnyElement> = Vec::new();
    let heading = |text: &str| {
        div()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_2XS))
            .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_SUBTLE))
            .child(text.to_uppercase())
            .into_any_element()
    };
    // API Client's items: the heading over a search that takes the keys, and each row's method in front of its name.
    if offer.within.is_some() {
        rows.push(heading(offer.label));
        rows.push(super::ask_tools::search(look, &offer.query));
    }
    for (at, row) in offer.rows.iter().enumerate() {
        if let Some(group) = row.group {
            rows.push(heading(group));
        }
        let method = offer.within.as_ref().and_then(|within| within.method_of(&row.pick));
        let name: SharedString = match (method, row.says.is_empty()) {
            (Some(method), _) => format!("{method} {} {}", row.name, row.says).into(),
            (None, true) => row.name.clone().into(),
            (None, false) => format!("{} {}", row.name, row.says).into(),
        };
        // An issue read from a project's tracker names the project.
        let name: SharedString = match &row.project {
            Some(project) => format!("{name} {project}").into(),
            None => name,
        };
        // A tool's glyph takes its hue and its name the box's own face; a command's or a file's name is code.
        let hue = match &row.pick {
            Pick::Tag { kind: TagKind::Tool, id, .. } => super::ask_tools::hue(id),
            Pick::Tag { kind: TagKind::Request, .. } => super::ask_tools::hue(super::ask_tools::API_CLIENT),
            _ => None,
        };
        // The sign that → opens API Client's items, on its row alone.
        let opens = offer.within.is_none() && matches!(&row.pick, Pick::Tag { kind: TagKind::Tool, id, .. } if id == super::ask_tools::API_CLIENT);
        let mono = if hue.is_some() { None } else { look.font_stack(token::FONT_MONO).into_iter().next() };
        rows.push(
            labelled(
                div()
                    .id(SharedString::from(format!("offer-{at}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .when(offer.current == at, |row| row.bg(look.color(token::BG_HOVER)))
                    .cursor_pointer()
                    .child(if offer.within.is_some() {
                        super::ask_tools::lead(look, method, row.glyph)
                    } else {
                        div()
                            .flex_none()
                            .mr(look.length(token::SPACE_2))
                            .child(svg().path(icon_path(row.glyph)).size(look.length(token::SPACE_3)).text_color(look.color(hue.unwrap_or(token::FG_MUTED))))
                            .into_any_element()
                    })
                    .child(div().flex_none().when_some(mono, |name, mono| name.font_family(mono)).text_color(look.color(token::FG_DEFAULT)).child(row.name.clone()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .ml(look.length(token::SPACE_3))
                            .when(offer.within.is_some(), |says| says.text_right().text_color(look.color(token::FG_SUBTLE)))
                            .when(offer.within.is_none(), |says| says.text_color(look.color(token::FG_MUTED)))
                            .child(row.says.clone()),
                    )
                    .children(row.project.clone().map(|project| {
                        div().flex_none().max_w(gpui::relative(0.4)).truncate().ml(look.length(token::SPACE_3)).text_color(look.color(token::FG_SUBTLE)).child(project)
                    }))
                    .when(opens, |row| {
                        row.child(
                            div()
                                .flex_none()
                                .ml(look.length(token::SPACE_2))
                                .child(svg().path(icon_path("chevron-right")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED))),
                        )
                    })
                    .on_click(choose(at)),
                Role::ListItem,
                name,
            )
            .into_any_element(),
        );
    }
    if offer.loading {
        for at in 0..2 {
            rows.push(
                div()
                    .h(look.length(token::H_CONTROL))
                    .flex()
                    .items_center()
                    .px(look.length(token::SPACE_3))
                    .child(
                        div()
                            .h(look.length(token::SPACE_2))
                            .w(gpui::relative(0.4 + 0.2 * at as f32))
                            .rounded(look.length(token::RADIUS_SM))
                            .bg(look.color(token::FG_DEFAULT).opacity(0.07)),
                    )
                    .into_any_element(),
            );
        }
    }
    labelled(
        div()
            .id("offer")
            .when(floating, |list| list.absolute().bottom_full().left_0().mb(look.length(token::SPACE_1)))
            .w(look.length(token::SPACE_12) * 8.)
            .max_w_full()
            .max_h(look.length(token::H_CONTROL) * 8.)
            .overflow_y_scroll()
            .track_scroll(&offer.held.scroll)
            .flex()
            .flex_col()
            .p(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .occlude()
            .when_some(outside, |list, outside| list.on_mouse_down_out(outside))
            .children(rows),
        Role::List,
        offer.label,
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slash_offers_commands_only_at_the_very_start() {
        assert_eq!(token_at("/upd", 4), Some(('/', 0, "upd".into())));
        assert_eq!(token_at("run /upd", 8), None);
        assert_eq!(token_at("/update now", 11), None);
    }

    #[test]
    fn an_at_and_a_hash_offer_at_the_start_of_a_word() {
        assert_eq!(token_at("look at @sess", 13), Some(('@', 8, "sess".into())));
        assert_eq!(token_at("see #33", 7), Some(('#', 4, "33".into())));
        assert_eq!(token_at("mail a@b", 8), None);
    }

    #[test]
    fn commands_that_start_with_the_query_come_before_those_that_contain_it() {
        let commands = ["compact".to_string(), "recompile".to_string(), "model".to_string()];
        let names: Vec<_> = commands_for(&commands, "comp").into_iter().map(|row| row.name).collect();
        assert_eq!(names, ["/compact", "/recompile"]);
    }

    #[test]
    fn numbers_come_newest_first_and_stop_at_six() {
        let numbered: Vec<_> = (1..=9).map(|n| (false, format!("#{}", 3300 + n), format!("Issue {n}"), None)).collect();
        let rows = numbers_for(numbered, "");
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].name, "#3309");
    }

    fn tracked(reference: &str, title: &str, updated: &str) -> ipc::TrackedIssue {
        ipc::TrackedIssue {
            reference: reference.into(),
            title: title.into(),
            url: format!("https://forge/{}", reference.trim_start_matches('#')),
            labels: Vec::new(),
            assignees: Vec::new(),
            updated_at: ipc::Instant::carried(updated),
            excerpt: String::new(),
            linked_pull_request: false,
            author: None,
            state: ipc::IssueState::Open,
            fields: Vec::new(),
            holders: Vec::new(),
        }
    }

    #[test]
    fn project_numbers_are_newest_first_at_most_six_and_name_their_project() {
        let armada = [tracked("#1", "One", "2026-11-01T09:00:00Z"), tracked("#2", "Two", "2026-11-05T09:00:00Z"), tracked("#3", "Three", "2026-11-09T09:00:00Z")];
        let pocket = [
            tracked("#4", "Four", "2026-11-02T09:00:00Z"),
            tracked("#5", "Five", "2026-11-06T09:00:00Z"),
            tracked("#6", "Six", "2026-11-10T09:00:00Z"),
            tracked("#7", "Seven", "2026-11-03T09:00:00Z"),
            tracked("#8", "Eight", "2026-11-07T09:00:00Z"),
        ];
        let rows = project_numbers(&[("Armada", armada.as_slice()), ("Pocket", pocket.as_slice())], "");
        let named: Vec<(&str, Option<&str>)> = rows.iter().map(|row| (row.name.as_str(), row.project.as_deref())).collect();
        assert_eq!(
            named,
            [("#6", Some("Pocket")), ("#3", Some("Armada")), ("#8", Some("Pocket")), ("#5", Some("Pocket")), ("#2", Some("Armada")), ("#7", Some("Pocket"))],
            "the six newest of both trackers, newest first, each naming where it is from"
        );
        assert!(matches!(&rows[0].pick, Pick::Tag { kind: TagKind::Issue, id, title, url } if id == "6" && title == "#6 Six" && url.as_deref() == Some("https://forge/6")));
        let narrowed: Vec<_> = project_numbers(&[("Armada", armada.as_slice()), ("Pocket", pocket.as_slice())], "thr").into_iter().map(|row| row.name).collect();
        assert_eq!(narrowed, ["#3"], "the query narrows by title");
    }

    #[test]
    fn a_chip_keeps_its_kind_and_id_and_reads_back_as_the_mention() {
        let chip = chip_of(TagKind::Issue, "3290", "#3290 Merge line", Some("https://forge/3290"));
        assert_eq!(chip.text().as_ref(), "@#3290 Merge line");
        let tag = mention_of(chip.id(), chip.label()).expect("a mention");
        assert_eq!((tag.kind, tag.id.as_str(), tag.url.as_deref()), (TagKind::Issue, "3290", Some("https://forge/3290")));
    }

    #[test]
    fn a_tool_chip_reads_back_as_the_tool_it_asks_for() {
        let chip = chip_of(TagKind::Tool, "takes", "Takes", None);
        assert_eq!((chip.text().as_ref(), chip.label().as_ref()), ("@Takes", "Takes"));
        let tag = mention_of(chip.id(), chip.label()).expect("a mention");
        assert_eq!(tag, ipc::SessionTag { kind: TagKind::Tool, id: "takes".into(), title: "Takes".into(), url: None });
    }

    #[test]
    fn a_request_chip_reads_api_client_and_its_name_and_reads_back_as_the_request() {
        let chip = chip_of(TagKind::Request, "service/health", "Health", None);
        assert_eq!(chip.label().as_ref(), "API Client · Health", "the chip names its tool and the request");
        assert_eq!(chip.text().as_ref(), "@Health", "the words it stands for are the request's name");
        let tag = mention_of(chip.id(), chip.label()).expect("a mention");
        assert_eq!(tag, ipc::SessionTag { kind: TagKind::Request, id: "service/health".into(), title: "Health".into(), url: None }, "the wire title is the name alone");
        let new = chip_of(TagKind::Request, "new", "New request", None);
        assert_eq!(new.label().as_ref(), "API Client · New request");
        assert_eq!(mention_of(new.id(), new.label()).map(|tag| (tag.id, tag.title)), Some(("new".to_string(), "New request".to_string())));
    }
}
