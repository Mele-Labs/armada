//! ⌘F in a Job's thread: a bar at the thread's top that finds words anywhere in the thread, marks
//! every match, and steps through them with Enter, ⇧Enter, ⌘G and ⇧⌘G.
//!
//! **It reads what Fleet served, not what is drawn.** The list draws only the rows on screen, so the
//! matches are counted from each item's words: a message's and a reasoning's text, and the line each
//! tool call reads as. A row's marks are painted from its own drawn words once it is drawn: prose
//! through its text view's range highlights, a call's line through styled text. Markdown that reads
//! otherwise than its source, such as a link's address, can count a match its row does not mark.

use std::collections::HashMap;
use std::ops::Range;

use gpui::{div, prelude::*, AnyElement, Context, Entity, HighlightStyle, Hsla, KeyDownEvent, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::text::{RangeHighlight, RenderedText, TextViewState};
use ipc::{SessionRow, SessionVoice, ThreadItem};

use super::head::{act_button, icon_button};
use super::rows::{glyph, mono, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// Where a match's words are drawn.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Place {
    /// Words the thread draws as prose, by the key `prose` is given.
    Prose(String),
    /// One plain line: a tool call's, or a Fleet turn's notice or call.
    Line(String),
}

/// One stretch of the thread's words.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Words {
    pub item: usize,
    pub place: Place,
    pub text: String,
    /// The fold it is drawn in, and whether that fold starts open.
    pub fold: Option<(String, bool)>,
}

/// One match.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Hit {
    pub item: usize,
    pub place: Place,
    /// Which match it is in its place, from 0.
    pub nth: usize,
    pub fold: Option<(String, bool)>,
}

/// The words of every item a find reads, in the thread's order. `edits` are his saved new words, by row.
pub(super) fn words_of(items: &[ThreadItem], edits: &HashMap<String, String>) -> Vec<Words> {
    let mut all = Vec::new();
    for (item, one) in items.iter().enumerate() {
        match one {
            ThreadItem::Row { row: SessionRow::Message { id, from, text, .. } } => {
                let (key, text) = match from {
                    SessionVoice::You if super::rows::is_act(text) => continue,
                    SessionVoice::You => (format!("you:{id}"), super::rows::split_debug(edits.get(id).unwrap_or(text)).0.to_string()),
                    SessionVoice::Agent => (format!("agent:{id}"), text.clone()),
                    _ => (format!("from:{id}"), text.clone()),
                };
                all.push(Words { item, place: Place::Prose(key), text, fold: None });
            }
            ThreadItem::Row { row: SessionRow::Reasoning { id, text, .. } } => {
                let key = format!("reasoning:{id}");
                let open = super::lines::reasoning_reading(items, item).1;
                all.push(Words { item, place: Place::Prose(key.clone()), text: text.clone(), fold: Some((key, open)) });
            }
            ThreadItem::Calls { id, rows, .. } => {
                let fold = Some((format!("calls:{id}"), false));
                for (at, row) in rows.iter().enumerate() {
                    let line = match row {
                        SessionRow::Tool { message: Some(_), .. } => continue,
                        SessionRow::Tool { description: Some(said), .. } if !said.trim().is_empty() => said.clone(),
                        SessionRow::Tool { text, .. } => super::calls::call_line(text, false),
                        SessionRow::Ask { ask, .. } => super::calls::call_line(&format!("{} {}", ask.tool, ask.detail), false),
                        SessionRow::WindowAsk { question, .. } => question.clone(),
                        _ => continue,
                    };
                    all.push(Words { item, place: Place::Line(format!("call:{id}:{at}")), text: line, fold: fold.clone() });
                }
            }
            ThreadItem::FleetTurn { id, notice, rows, .. } => {
                let fold = Some((format!("fleet-turn-fold:{id}"), false));
                if let SessionRow::Message { text, .. } = notice {
                    all.push(Words { item, place: Place::Line(format!("fleet-notice:{id}")), text: text.clone(), fold: fold.clone() });
                }
                for (at, row) in rows.iter().enumerate() {
                    let (place, text) = match row {
                        SessionRow::Tool { text, .. } => (Place::Line(format!("fleet-call:{id}:{at}")), text.clone()),
                        SessionRow::Message { id: said, text, .. } => (Place::Prose(format!("fleet:{said}")), text.clone()),
                        _ => continue,
                    };
                    all.push(Words { item, place, text, fold: fold.clone() });
                }
            }
            _ => {}
        }
    }
    all
}

/// Where `query` stands in `text`, case aside, as byte ranges of `text`, none overlapping.
pub(super) fn spans(text: &str, query: &str) -> Vec<Range<usize>> {
    let low = |c: char| c.to_lowercase().next().unwrap_or(c);
    let wanted: Vec<char> = query.chars().map(low).collect();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut found = Vec::new();
    if wanted.is_empty() {
        return found;
    }
    let mut at = 0;
    while at + wanted.len() <= chars.len() {
        if chars[at..at + wanted.len()].iter().zip(&wanted).all(|((_, one), want)| low(*one) == *want) {
            let end = chars.get(at + wanted.len()).map_or(text.len(), |(end, _)| *end);
            found.push(chars[at].0..end);
            at += wanted.len();
        } else {
            at += 1;
        }
    }
    found
}

/// Every match of `query` in `words`, in the thread's order.
pub(super) fn hits_of(words: &[Words], query: &str) -> Vec<Hit> {
    words
        .iter()
        .flat_map(|one| {
            spans(&one.text, query).into_iter().enumerate().map(|(nth, _)| Hit { item: one.item, place: one.place.clone(), nth, fold: one.fold.clone() })
        })
        .collect()
}

/// The match a find starts on: the first at or below `from`, else the first of all.
pub(super) fn first_from(hits: &[Hit], from: usize) -> Option<usize> {
    (!hits.is_empty()).then(|| hits.iter().position(|hit| hit.item >= from).unwrap_or(0))
}

/// The match `by` steps from `at`, round the ends.
pub(super) fn stepped(at: usize, by: isize, count: usize) -> usize {
    (at as isize + by).rem_euclid(count.max(1) as isize) as usize
}

/// What the count reads: nothing before a word is typed.
pub(super) fn count_said(query: &str, at: Option<usize>, count: usize) -> Option<String> {
    match (query.is_empty(), at) {
        (true, _) => None,
        (false, None) => Some("No matches".to_string()),
        (false, Some(at)) => Some(format!("{} of {count}", at + 1)),
    }
}

/// The fill behind every match, and behind the current one.
fn fills(look: &Look) -> (Hsla, Hsla) {
    (look.color(token::ACCENT).opacity(0.28), look.color(token::STATUS_AWAITING_REVIEW).opacity(0.55))
}

/// The bar while it is open on a Job.
pub(super) struct Find {
    job: SharedString,
    input: Entity<InputState>,
    query: String,
    hits: Vec<Hit>,
    at: Option<usize>,
    /// The item a new word starts from: the top of the screen when the bar opened, then the current match.
    anchor: usize,
    /// The prose the matches are in, each with its own text view so its marks can be painted.
    painted: HashMap<String, Painted>,
    /// The current match waits to be scrolled to inside its prose once those words are read.
    reveal: bool,
    _typed: Subscription,
}

struct Painted {
    state: Entity<TextViewState>,
    source: String,
    /// The words, the word looked for and the current match last painted.
    searched: Option<(RenderedText, String, Option<usize>)>,
    _read: Subscription,
}

impl JobView {
    /// ⌘F: the bar opens over the shown Job's thread with the keys in it.
    pub(super) fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if let Some(find) = self.find.as_ref().filter(|find| find.job == id) {
            find.input.update(cx, |input, cx| input.focus(window, cx));
            return;
        }
        let anchor = self.pages.get(&id).map_or(0, |page| page.list.logical_scroll_top().item_ix.saturating_sub(page.lead + page.before));
        let input = cx.new(|cx| InputState::new(window, cx));
        let typed = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.find_typed(cx);
            }
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        self.find = Some(Find { job: id, input, query: String::new(), hits: Vec::new(), at: None, anchor, painted: HashMap::new(), reveal: false, _typed: typed });
        cx.notify();
    }

    /// Esc: the bar shuts, its marks go, and the keys go back to the message box.
    pub(super) fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.take().is_none() {
            return;
        }
        match self.shown.clone() {
            Some(id) if !self.closed(&id, cx) => self.to_composer(window, cx),
            _ => window.focus(&self.focus, cx),
        }
        cx.notify();
    }

    /// Whether the bar is open on the Job shown.
    pub(super) fn finding(&self) -> bool {
        self.find.as_ref().is_some_and(|find| Some(&find.job) == self.shown.as_ref())
    }

    fn find_typed(&mut self, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_mut() else { return };
        find.query = find.input.read(cx).value().to_string();
        self.count_matches(cx);
        let Some(find) = self.find.as_mut() else { return };
        find.at = first_from(&find.hits, find.anchor);
        self.go_to_match(cx);
    }

    /// ⌘G, Enter and their shifts: the next or the previous match, round the ends.
    pub(super) fn find_step(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_mut() else { return };
        let Some(at) = find.at else { return };
        find.at = Some(stepped(at, by, find.hits.len()));
        self.go_to_match(cx);
    }

    /// The thread changed under an open bar: the matches are counted again and the current one kept.
    pub(super) fn find_thread_changed(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref().filter(|find| &find.job == id) else { return };
        let was = find.at.and_then(|at| find.hits.get(at)).cloned();
        self.count_matches(cx);
        let Some(find) = self.find.as_mut() else { return };
        find.at = was.and_then(|was| find.hits.iter().position(|hit| *hit == was)).or_else(|| first_from(&find.hits, find.anchor));
        let keys: Vec<String> = find.painted.keys().cloned().collect();
        keys.iter().for_each(|key| self.paint_matches(key, cx));
    }

    /// Counts the matches in the whole thread, and keeps a text view for each prose that holds one.
    fn count_matches(&mut self, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else { return };
        let job = find.job.clone();
        let query = find.query.clone();
        let edits = self.pages.get(&job).map(|page| page.edits.clone()).unwrap_or_default();
        let words = self.thread(&job, cx).map(|held| words_of(&held.items, &edits)).unwrap_or_default();
        let hits = hits_of(&words, &query);
        let mut wanted: HashMap<String, String> = HashMap::new();
        for one in &words {
            if let Place::Prose(key) = &one.place {
                if hits.iter().any(|hit| hit.place == one.place) {
                    wanted.insert(key.clone(), self.prose_source(&one.text, cx));
                }
            }
        }
        let Some(find) = self.find.as_mut() else { return };
        find.hits = hits;
        find.painted.retain(|key, painted| wanted.get(key) == Some(&painted.source));
        let new: Vec<(String, String)> = wanted.into_iter().filter(|(key, _)| !find.painted.contains_key(key)).collect();
        for (key, source) in new {
            let state = cx.new(|cx| TextViewState::markdown(&source, cx));
            let read = cx.observe(&state, {
                let key = key.clone();
                move |this, _, cx| this.paint_matches(&key, cx)
            });
            if let Some(find) = self.find.as_mut() {
                find.painted.insert(key, Painted { state, source, searched: None, _read: read });
            }
        }
    }

    /// The current match: its fold opens, its row scrolls into the thread, and every mark is painted again.
    fn go_to_match(&mut self, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_mut() else { return };
        let hit = find.at.and_then(|at| find.hits.get(at)).cloned();
        if let Some(hit) = &hit {
            find.anchor = hit.item;
            find.reveal = matches!(hit.place, Place::Prose(_));
        }
        let job = find.job.clone();
        let keys: Vec<String> = find.painted.keys().cloned().collect();
        if let (Some(hit), Some(page)) = (hit, self.pages.get_mut(&job)) {
            if let Some((key, first_open)) = &hit.fold {
                if *first_open == page.opened.contains(key) {
                    if !page.opened.remove(key) {
                        page.opened.insert(key.clone());
                    }
                    page.remeasure(hit.item);
                }
            }
            // A thread that follows its tail would snap back to it: it stays where the match is, and follows
            // again once he is back at the bottom.
            page.list.pause_following_tail();
            page.list.scroll_to_reveal_item(page.entry(hit.item));
        }
        keys.iter().for_each(|key| self.paint_matches(key, cx));
        cx.notify();
    }

    /// Paints the marks on one prose's words, once they are read, and scrolls the current match into view.
    fn paint_matches(&mut self, key: &str, cx: &mut Context<Self>) {
        let (all, now) = fills(cx.look());
        let Some(find) = self.find.as_mut() else { return };
        let current = find.at.and_then(|at| find.hits.get(at)).filter(|hit| hit.place == Place::Prose(key.to_string())).map(|hit| hit.nth);
        let query = find.query.clone();
        let reveal = find.reveal && current.is_some();
        let Some(painted) = find.painted.get_mut(key) else { return };
        let text = painted.state.read(cx).rendered_text();
        if text.is_empty() {
            return;
        }
        let stamp = (text.clone(), query.clone(), current);
        if painted.searched.as_ref() == Some(&stamp) && !reveal {
            return;
        }
        let found = spans(text.as_str(), &query);
        let current = current.filter(|_| !found.is_empty()).map(|nth| nth.min(found.len() - 1));
        let marks: Vec<RangeHighlight> = found.iter().enumerate().map(|(nth, range)| RangeHighlight::new(range.clone(), if Some(nth) == current { now } else { all })).collect();
        let shown = current.map(|nth| found[nth].clone());
        painted.state.update(cx, |state, cx| {
            state.set_range_highlights(marks, cx).ok();
            if let Some(range) = shown.filter(|_| reveal) {
                state.reveal_range(range, cx).ok();
            }
        });
        painted.searched = Some(stamp);
        if reveal {
            find.reveal = false;
        }
    }

    /// The text view a prose's words are drawn through while the bar has a match in them: `None` draws them as ever.
    pub(super) fn found_prose(&self, key: &str, source: &str) -> Option<Entity<TextViewState>> {
        let find = self.find.as_ref().filter(|_| self.finding())?;
        find.painted.get(key).filter(|painted| painted.source == source).map(|painted| painted.state.clone())
    }

    /// The marks on one plain line of the thread, by its key: empty where the bar has no match in it.
    pub(super) fn line_marks(&self, key: &str, text: &str, look: &Look) -> Vec<(Range<usize>, HighlightStyle)> {
        let Some(find) = self.find.as_ref().filter(|_| self.finding()) else { return Vec::new() };
        let place = Place::Line(key.to_string());
        if !find.hits.iter().any(|hit| hit.place == place) {
            return Vec::new();
        }
        let current = find.at.and_then(|at| find.hits.get(at)).filter(|hit| hit.place == place).map(|hit| hit.nth);
        let (all, now) = fills(look);
        spans(text, &find.query)
            .into_iter()
            .enumerate()
            .map(|(nth, range)| (range, HighlightStyle { background_color: Some(if Some(nth) == current { now } else { all }), ..Default::default() }))
            .collect()
    }

    /// The bar over the thread, while it is open on this Job.
    pub(super) fn render_find(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let find = self.find.as_ref().filter(|find| &find.job == id)?;
        let look = cx.look();
        let said = count_said(&find.query, find.at, find.hits.len());
        let none = said.is_some() && find.at.is_none();
        let edge = look.color(if none { token::STATUS_COMPLETED_FAILED } else { token::ACCENT });
        let count = said.map(|said| {
            labelled(
                div()
                    .id("thread-find-count")
                    .flex_none()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(if none { token::STATUS_COMPLETED_FAILED } else { token::FG_MUTED }))
                    .child(said.clone()),
                Role::Status,
                said,
            )
        });
        let field = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_BASE))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .child(glyph(look, "search", Size::Small, edge))
            .child(labelled(
                div()
                    .id("thread-find-input")
                    .key_context(crate::keymap::TEXT_BOX)
                    .flex_1()
                    .min_w_0()
                    .text_size(look.length(token::TEXT_SM))
                    .child(Input::new(&find.input).appearance(false)),
                Role::SearchInput,
                "Find in thread",
            ))
            .children(count);
        let off = find.at.is_none();
        let previous = labelled(
            act_button(look, "thread-find-previous", "chevron-up", "thread_find_previous", "Previous match".into(), false, off)
                .on_click(cx.listener(|this, _, _, cx| this.find_step(-1, cx))),
            Role::Button,
            "Previous match",
        );
        let next = labelled(
            act_button(look, "thread-find-next", "chevron-down", "thread_find_next", "Next match".into(), false, off).on_click(cx.listener(|this, _, _, cx| this.find_step(1, cx))),
            Role::Button,
            "Next match",
        );
        let close = labelled(
            icon_button(look, "thread-find-close", "x", "Close".into(), false, false).on_click(cx.listener(|this, _, window, cx| this.close_find(window, cx))),
            Role::Button,
            "Close find",
        );
        let row = div()
            .w_full()
            .min_w_0()
            .max_w(self.column(look))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .child(field)
            .child(previous)
            .child(next)
            .child(close);
        Some(
            labelled(
                div()
                    .id("thread-find")
                    .flex_none()
                    .flex()
                    .justify_center()
                    // Narrower than the rows' sides, so the box keeps its room in a narrow pane.
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .bg(look.color(token::BG_RAISED))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    // The bar's own keys, before the box under them reads them as typing.
                    .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        let key = &event.keystroke;
                        if key.modifiers.platform || key.modifiers.control || key.modifiers.alt {
                            return;
                        }
                        match key.key.as_str() {
                            "escape" => this.close_find(window, cx),
                            "enter" => this.find_step(if key.modifiers.shift { -1 } else { 1 }, cx),
                            _ => return,
                        }
                        cx.stop_propagation();
                    }))
                    .child(row),
                Role::Toolbar,
                "Find",
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(json: &str) -> Vec<ThreadItem> {
        ipc::decode::<Vec<ThreadItem>>("items", json.as_bytes()).expect("the items decode")
    }

    const THREAD: &str = r#"[
        {"kind":"row","row":{"kind":"message","id":"y1","at":"2026-11-11T02:00:00Z","from":{"kind":"you"},"text":"Why do Webhook retries pile up?"}},
        {"kind":"calls","id":"t1","names":["Bash","Read"],"rows":[
            {"kind":"tool","id":"t1","at":"2026-11-11T02:00:05Z","text":"Bash git grep -n webhook -- crates"},
            {"kind":"tool","id":"t1b","at":"2026-11-11T02:00:06Z","text":"Read crates/fleet/src/queue.rs"}]},
        {"kind":"row","row":{"kind":"reasoning","id":"r1","at":"2026-11-11T02:00:08Z","text":"The webhook backoff doubles; webhook calls wait."}},
        {"kind":"calls","id":"t2","names":["Bash"],"rows":[{"kind":"tool","id":"t2","at":"2026-11-11T02:00:09Z","text":"Bash ls"}]},
        {"kind":"row","row":{"kind":"message","id":"a1","at":"2026-11-11T02:00:10Z","from":{"kind":"agent"},"text":"The retries drain now."}}
    ]"#;

    #[test]
    fn a_find_counts_every_match_in_the_thread_with_the_fold_each_sits_in() {
        let items = items(THREAD);
        let hits = hits_of(&words_of(&items, &HashMap::new()), "webhook");
        let places: Vec<(usize, &Place, usize)> = hits.iter().map(|hit| (hit.item, &hit.place, hit.nth)).collect();
        assert_eq!(
            places,
            vec![
                (0, &Place::Prose("you:y1".into()), 0),
                (1, &Place::Line("call:t1:0".into()), 0),
                (2, &Place::Prose("reasoning:r1".into()), 0),
                (2, &Place::Prose("reasoning:r1".into()), 1),
            ]
        );
        assert_eq!(hits[0].fold, None);
        assert_eq!(hits[1].fold, Some(("calls:t1".to_string(), false)));
        assert_eq!(hits[2].fold, Some(("reasoning:r1".to_string(), false)));
    }

    #[test]
    fn his_saved_words_are_found_in_place_of_what_he_first_sent() {
        let items = items(THREAD);
        let edits = HashMap::from([("y1".to_string(), "Why is the queue slow?".to_string())]);
        let hits = hits_of(&words_of(&items, &edits), "queue");
        assert_eq!(hits.iter().map(|hit| hit.item).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn a_match_ignores_case_and_never_overlaps() {
        assert_eq!(spans("Webhook WEBHOOK webhook", "webhook"), vec![0..7, 8..15, 16..23]);
        assert_eq!(spans("aaaa", "aa"), vec![0..2, 2..4]);
        assert_eq!(spans("Ärger ärger", "ärger"), vec![0..6, 7..13]);
        assert!(spans("anything", "").is_empty());
    }

    #[test]
    fn steps_go_round_the_ends_and_a_find_starts_at_the_screen() {
        assert_eq!(stepped(0, -1, 5), 4);
        assert_eq!(stepped(4, 1, 5), 0);
        assert_eq!(stepped(2, 1, 5), 3);
        let hits = hits_of(&words_of(&items(THREAD), &HashMap::new()), "webhook");
        assert_eq!(first_from(&hits, 2), Some(2));
        assert_eq!(first_from(&hits, 4), Some(0));
        assert_eq!(first_from(&[], 0), None);
    }

    #[test]
    fn the_count_reads_as_a_place_among_the_matches_or_none() {
        assert_eq!(count_said("", None, 0), None);
        assert_eq!(count_said("webhook", None, 0).as_deref(), Some("No matches"));
        assert_eq!(count_said("webhook", Some(2), 12).as_deref(), Some("3 of 12"));
    }
}
