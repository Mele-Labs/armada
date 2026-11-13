//! What typing in the message box offers (`SessionComposer.tsx`, `tokenAt`, section 4.4): `/` at the
//! start lists the agent's commands, `@` at the start of a word lists other Jobs and then files, and
//! `#` lists the repository's issues and pull requests.
//!
//! The list hangs over the box. ↑ and ↓ move, Enter and Tab choose, Esc closes. A Job or a number
//! chosen becomes one atomic chip in the text; a file or a command is plain text.

use std::time::Duration;

use gpui::{div, prelude::*, svg, Context, Role, SharedString, Window};
use gpui_component::input::InlineToken;
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
    pub pick: Pick,
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
            Row { group: None, glyph: "slash", name: format!("/{bare}"), says: String::new(), pick: Pick::Command(bare) }
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

/// A tag as the text holds it: `@<title>`, with the kind and id kept in the chip's own id.
pub(crate) fn chip_of(kind: TagKind, id: &str, title: &str, url: Option<&str>) -> InlineToken {
    let kind = match kind {
        TagKind::Session => "session",
        TagKind::PullRequest => "pull_request",
        TagKind::Branch => "branch",
        TagKind::Issue => "issue",
        TagKind::Report => "report",
        TagKind::Terminal => "terminal",
    };
    let one_line = title.replace(['\n', '\r'], " ");
    InlineToken::new(format!("{kind}\u{1f}{id}\u{1f}{}", url.unwrap_or_default()), format!("@{one_line}")).with_label(one_line)
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
        _ => return None,
    };
    let tag_id = parts.next()?.to_string();
    let url = parts.next().filter(|url| !url.is_empty()).map(str::to_string);
    Some(ipc::SessionTag { kind, id: tag_id, title: label.to_string(), url })
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
                let commands = self.record(&id, cx).and_then(|job| job.hosted.as_ref()).map(|hosted| hosted.commands.clone()).unwrap_or_default();
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
                ("Tag an issue or pull request", numbers_for(numbered, &query))
            }
            _ => {
                let lowered = query.to_lowercase();
                let mut rows: Vec<Row> = self
                    .list
                    .read(cx)
                    .records()
                    .iter()
                    .filter(|job| self.reporting.is_none() && job.id.as_str() != id.as_str() && job.state != ipc::SessionState::Ended)
                    .filter_map(|job| job.title.as_ref().map(|title| (job.id.as_str().to_string(), title.clone())))
                    .filter(|(_, title)| lowered.is_empty() || title.to_lowercase().contains(&lowered))
                    .take(5)
                    .map(|(job, title)| Row {
                        group: None,
                        glyph: "square-terminal",
                        name: title.clone(),
                        says: String::new(),
                        pick: Pick::Tag { kind: TagKind::Session, id: job, title, url: None },
                    })
                    .collect();
                if let Some(first) = rows.first_mut() {
                    first.group = Some("Jobs");
                }
                let files = self.composer.catalog.files.as_ref().filter(|(asked, _)| *asked == query).map(|(_, paths)| paths.clone());
                loading = files.is_none();
                let stale = files.or_else(|| self.composer.catalog.files.as_ref().map(|(_, paths)| paths.clone())).unwrap_or_default();
                let before = rows.len();
                rows.extend(stale.into_iter().take(5).map(|path| {
                    let (folder, name) = path.rsplit_once('/').map_or(("", path.as_str()), |(folder, name)| (folder, name));
                    Row { group: None, glyph: "file", name: name.to_string(), says: folder.to_string(), pick: Pick::File(path.clone()) }
                }));
                if let Some(first) = rows.get_mut(before) {
                    first.group = Some("Files");
                }
                self.want_files(&id, &query, window, cx);
                ("Tag a Job or a file", rows)
            }
        };
        if rows.is_empty() && !loading {
            self.composer.offer = None;
            return;
        }
        let current = self.composer.offer.as_ref().filter(|held| held.label == label).map_or(0, |held| held.current.min(rows.len().saturating_sub(1)));
        self.composer.offer = Some(Offer { label, rows, current, from, to, query, loading });
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

    /// A key while a list is open: it is the list's, and the box does not see it. True where taken.
    pub(super) fn offer_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(offer) = self.composer.offer.as_mut() else { return false };
        let last = offer.rows.len().saturating_sub(1);
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
            match row.pick {
                Pick::Command(name) => {
                    input.set_selected_range(range, cx);
                    input.replace(format!("/{name} "), window, cx);
                }
                Pick::File(path) => {
                    input.set_selected_range(range, cx);
                    input.replace(format!("@{path} "), window, cx);
                }
                Pick::Tag { kind, id, title, url } => {
                    if input.replace_range_with_token(range, chip_of(kind, &id, &title, url.as_deref()), window, cx).is_ok() {
                        input.insert(" ", window, cx);
                    }
                }
            }
        });
        self.typed(cx);
    }

    /// The list over the box (`.armada-mention`): rows with a glyph, a name and its words, in groups.
    pub(super) fn render_offer(&self, look: &Look, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let offer = self.composer.offer.as_ref()?;
        Some(offer_list(offer, look, |at| {
            let this = cx.entity().downgrade();
            Box::new(move |_, window, cx| {
                let _ = this.update(cx, |this, cx| this.choose(at, window, cx));
            })
        }))
    }
}

/// The list over a box (`.armada-mention`): rows with a glyph, a name and its words, in groups. `choose` makes
/// each row's press.
pub(crate) fn offer_list(offer: &Offer, look: &Look, choose: impl Fn(usize) -> Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App)>) -> gpui::AnyElement {
    let mut rows: Vec<gpui::AnyElement> = Vec::new();
    for (at, row) in offer.rows.iter().enumerate() {
        if let Some(group) = row.group {
            rows.push(
                div()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(group.to_uppercase())
                    .into_any_element(),
            );
        }
        let name: SharedString = if row.says.is_empty() { row.name.clone().into() } else { format!("{} {}", row.name, row.says).into() };
        let mono = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default();
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
                    .child(
                        div().flex_none().mr(look.length(token::SPACE_2)).child(
                            svg().path(icon_path(row.glyph)).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)),
                        ),
                    )
                    .child(div().flex_none().font_family(mono).text_color(look.color(token::FG_DEFAULT)).child(row.name.clone()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .ml(look.length(token::SPACE_3))
                            .text_color(look.color(token::FG_MUTED))
                            .child(row.says.clone()),
                    )
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
            .absolute()
            .bottom_full()
            .left_0()
            .mb(look.length(token::SPACE_1))
            .w(look.length(token::SPACE_12) * 8.)
            .max_w_full()
            .max_h(look.length(token::H_CONTROL) * 8.)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .p(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .occlude()
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

    #[test]
    fn a_chip_keeps_its_kind_and_id_and_reads_back_as_the_mention() {
        let chip = chip_of(TagKind::Issue, "3290", "#3290 Merge line", Some("https://forge/3290"));
        assert_eq!(chip.text().as_ref(), "@#3290 Merge line");
        let tag = mention_of(chip.id(), chip.label()).expect("a mention");
        assert_eq!((tag.kind, tag.id.as_str(), tag.url.as_deref()), (TagKind::Issue, "3290", Some("https://forge/3290")));
    }
}
