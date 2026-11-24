//! The margin beside a page: his comments on its passages and the agent's answers, one thread a passage, as Fleet keeps
//! them (`ipc::MarginThread`). This file holds what the margin remembers and what its acts send; `reader_talk.rs`
//! draws it. **An answer changes nothing on the page**: only his Accept of a suggested edit writes a revision.
//!
//! The threads are read once a page's reader opens and again whenever the Job's thread says a round moved
//! (`ThreadItem::Margin`). What he types in a comment or a reply is kept as he types, so it is there when he comes back
//! and when a newer revision comes in under him.

use std::collections::HashMap;

use gpui::{AppContext as _, Context, Entity, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, TextareaState};
use ipc::{ActOnMargin, CommentMargin, MarginAct, MarginAnchor, MarginOn, MarginState, MarginThread, MarginThreads, SendHeldMargin, SessionId, ThreadItem};
use serde::{Deserialize, Serialize};

use super::row_tabs::RowKind;
use super::JobView;
use crate::web::reader as ask;

/// What one page's margin remembers.
#[derive(Default)]
pub(super) struct Margin {
    /// Oldest first, as Fleet answers them.
    pub threads: Vec<MarginThread>,
    /// Fleet has answered once, so the threads draw in place of the shapes.
    pub loaded: bool,
    /// The Settled tab is in front.
    pub settled: bool,
    /// The thread J and K are on.
    pub on: Option<String>,
    /// The comment he is writing on a passage, not sent yet.
    pub new: Option<NewComment>,
    /// The reply box of each thread still open.
    pub replies: HashMap<String, (Entity<TextareaState>, Subscription)>,
    /// A request is on its way, so the acts wait for it.
    pub busy: bool,
    /// What Fleet said when it refused the last request.
    pub said: Option<SharedString>,
    /// The rounds the threads were read at, so a moved round reads them again.
    pub stamp: Option<String>,
    /// The comment he left half written is back.
    pub restored: bool,
    /// The margin's own scroll, so a thread chosen from the Job's thread is brought into view.
    pub scroll: gpui::ScrollHandle,
    /// The thread to bring into view once the margin is drawn at the width it stands at.
    pub reach: std::cell::RefCell<Option<String>>,
}

/// A comment begun on a passage.
pub(super) struct NewComment {
    pub anchor: MarginAnchor,
    pub area: Entity<TextareaState>,
    pub _typing: Subscription,
}

/// What a typed box is.
#[derive(Clone)]
enum Typed {
    New,
    Reply(String),
}

/// What a request sent, so its box is emptied once Fleet took it.
#[derive(Clone)]
enum Sent {
    New,
    Reply(String),
    Nothing,
}

/// A comment begun, as it is kept while he types.
#[derive(Serialize, Deserialize)]
struct KeptNew {
    quote: String,
    #[serde(default)]
    block: Option<String>,
    #[serde(default)]
    section: Option<String>,
    text: String,
}

fn new_key(job: &str, page: &str) -> String {
    format!("margin-new:{job}:{page}")
}

fn reply_key(job: &str, page: &str, thread: &str) -> String {
    format!("margin-reply:{job}:{page}:{thread}")
}

/// The rounds of talk on page `page` the Job's thread holds: what changes when Fleet says its threads moved.
pub(super) fn stamp_of(items: &[ThreadItem], page: &str) -> String {
    items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::Margin(card) if card.page == page => Some(format!("{}\u{1f}{}\u{1f}{}", card.id, card.summary, card.waits_on_you)),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where the text of a picked passage is empty, it is none.
fn some(text: String) -> Option<String> {
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

impl Margin {
    /// He holds a comment on the page, to go with Send held.
    pub(super) fn holds(&self) -> bool {
        self.threads.iter().any(|one| one.messages.iter().any(|said| said.held))
    }

    /// The threads the Open tab lists.
    pub(super) fn open(&self) -> impl Iterator<Item = &MarginThread> {
        self.threads.iter().filter(|one| one.state != MarginState::Settled)
    }

    /// Where thread `thread` stands among the Open tab's children, as `margin_column` lays them out: the head, Fleet's
    /// refusal, the held bar and the comment being written come before the threads.
    pub(super) fn place_of(&self, thread: &str) -> Option<usize> {
        let before = 1 + usize::from(self.said.is_some()) + usize::from(self.holds()) + usize::from(self.new.is_some());
        self.open().position(|one| one.id == thread).map(|at| before + at)
    }

    /// Whether the caret is in a comment or a reply.
    pub(super) fn typing(&self, window: &Window, cx: &gpui::App) -> bool {
        use gpui::Focusable as _;
        let in_area = |area: &Entity<TextareaState>| area.read(cx).focus_handle(cx).contains_focused(window, cx);
        self.new.as_ref().is_some_and(|new| in_area(&new.area)) || self.replies.values().any(|(area, _)| in_area(area))
    }

    /// The marks the page draws: each open thread's passage in his colour, a suggested edit struck and added, and the
    /// passage of the comment he is writing. A thread whose passage is gone marks nothing.
    pub(super) fn marks(&self) -> Vec<ask::Mark> {
        let mut marks: Vec<ask::Mark> = self
            .open()
            .filter(|one| !one.gone)
            .map(|one| {
                let pending = one.suggestion.as_ref().filter(|edit| edit.decided.is_none() && one.state == MarginState::Suggested);
                ask::Mark {
                    thread: one.id.clone(),
                    quote: one.anchor.quote.clone(),
                    block: one.anchor.block.clone().unwrap_or_default(),
                    on: self.on.as_deref() == Some(one.id.as_str()),
                    struck: pending.map(|edit| edit.struck.clone()),
                    added: pending.map(|edit| edit.added.clone()),
                }
            })
            .collect();
        if let Some(new) = &self.new {
            marks.push(ask::Mark { thread: "new".into(), quote: new.anchor.quote.clone(), block: new.anchor.block.clone().unwrap_or_default(), on: true, struck: None, added: None });
        }
        marks
    }
}

impl JobView {
    /// A page's margin is read as its reader opens, and again when a round on the page moved; the comment he left half
    /// written comes back.
    pub(super) fn margin_sync(&mut self, id: &SharedString, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        if held.kind != RowKind::Document {
            return;
        }
        let page = held.key.clone();
        let restored = held.margin.restored;
        let stamp = self.thread(id, cx).map(|held| stamp_of(&held.items, &page)).unwrap_or_default();
        if !restored {
            let kept = serde_json::from_str::<KeptNew>(&self.draft_words(&new_key(id, &page))).ok().filter(|kept| !kept.quote.trim().is_empty());
            if let Some(kept) = kept {
                let (area, typing) = self.margin_area(id, name, Typed::New, &kept.text, window, cx);
                if let Some(held) = self.reader_mut(id, name) {
                    held.margin.new = Some(NewComment { anchor: MarginAnchor { quote: kept.quote, block: kept.block, section: kept.section }, area, _typing: typing });
                }
            }
            if let Some(held) = self.reader_mut(id, name) {
                held.margin.restored = true;
            }
        }
        if self.reader_of(id, name).is_some_and(|held| held.margin.stamp.as_deref() == Some(stamp.as_str())) {
            return;
        }
        if let Some(held) = self.reader_mut(id, name) {
            held.margin.stamp = Some(stamp);
        }
        self.margin_fetch(id, name, window, cx);
    }

    /// A box for a comment or a reply, which keeps what is typed in it.
    fn margin_area(&mut self, id: &SharedString, name: &str, typed: Typed, text: &str, window: &mut Window, cx: &mut Context<Self>) -> (Entity<TextareaState>, Subscription) {
        let placeholder = match typed {
            Typed::New => "Comment",
            Typed::Reply(_) => "Reply",
        };
        let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).placeholder(placeholder));
        area.update(cx, |area, cx| area.set_value(text.to_string(), window, cx));
        let (job, reader) = (id.clone(), name.to_string());
        let typing = cx.subscribe_in(&area, window, move |this, _, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                this.margin_typed(&job, &reader, &typed, cx);
            }
        });
        (area, typing)
    }

    /// What he typed is kept, under the page and the thread it is on.
    fn margin_typed(&mut self, id: &SharedString, name: &str, typed: &Typed, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let page = held.key.clone();
        let (key, words) = match typed {
            Typed::New => {
                let kept = held.margin.new.as_ref().map(|new| KeptNew {
                    quote: new.anchor.quote.clone(),
                    block: new.anchor.block.clone(),
                    section: new.anchor.section.clone(),
                    text: new.area.read(cx).value().to_string(),
                });
                (new_key(id, &page), kept.and_then(|kept| serde_json::to_string(&kept).ok()).unwrap_or_default())
            }
            Typed::Reply(thread) => (reply_key(id, &page, thread), held.margin.replies.get(thread).map(|(area, _)| area.read(cx).value().to_string()).unwrap_or_default()),
        };
        self.keep_words(&key, &words, cx);
        cx.notify();
    }

    /// The page's threads, read from Fleet (`get_margin_threads`).
    fn margin_fetch(&mut self, id: &SharedString, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.reader_of(id, name).map(|held| held.key.clone()) else { return };
        if self.fleet.is_none() {
            let kept = self.row_tabs.entry(id.clone()).or_default().mock_margin.entry(page.clone()).or_insert_with(|| crate::mock_margin::threads(id, &page)).clone();
            self.margin_took(id, name, Ok(kept), window, cx);
            return;
        }
        let path = format!("/sessions/margin?session_id={}&document={}", super::wire::encoded(id), super::wire::encoded(&page));
        let (job, reader) = (id.clone(), name.to_string());
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<MarginThreads>("MarginThreads", &bytes).map_err(|why| why.to_string()));
                this.margin_took(&job, &reader, read, window, cx);
            }),
        );
    }

    /// Fleet's threads, or its refusal: every thread still open has its reply box, with what he typed in it.
    fn margin_took(&mut self, id: &SharedString, name: &str, read: Result<MarginThreads, String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let page = held.key.clone();
        let first = !held.margin.loaded;
        match read {
            Ok(read) => {
                let open: Vec<String> = read.threads.iter().filter(|one| one.state != MarginState::Settled).map(|one| one.id.clone()).collect();
                let missing: Vec<String> = open.iter().filter(|one| !held.margin.replies.contains_key(*one)).cloned().collect();
                let mut made = Vec::new();
                for thread in missing {
                    let text = self.draft_words(&reply_key(id, &page, &thread));
                    let area = self.margin_area(id, name, Typed::Reply(thread.clone()), &text, window, cx);
                    made.push((thread, area));
                }
                let Some(held) = self.reader_mut(id, name) else { return };
                held.margin.replies.extend(made);
                held.margin.replies.retain(|thread, _| open.contains(thread));
                held.margin.threads = read.threads;
                held.margin.said = None;
                if held.margin.on.as_ref().is_some_and(|on| !open.contains(on)) {
                    held.margin.on = None;
                }
            }
            Err(why) => {
                let Some(held) = self.reader_mut(id, name) else { return };
                held.margin.said = Some(why.into());
            }
        }
        if let Some(held) = self.reader_mut(id, name) {
            held.margin.loaded = true;
            held.margin.busy = false;
        }
        self.margin_marks(id, name, cx);
        // A thread chosen before the threads came in is brought into view now that they have.
        if first {
            self.margin_reach(id, name, cx);
        }
        cx.notify();
    }

    /// The page draws the margin's marks.
    pub(super) fn margin_marks(&mut self, id: &SharedString, name: &str, cx: &mut Context<Self>) {
        let Some(marks) = self.reader_of(id, name).map(|held| held.margin.marks()) else { return };
        let Some(tab) = self.reader_page(id, name) else { return };
        tab.update(cx, |tab, cx| tab.set_marks(ask::mark(&marks), cx));
    }

    /// N on words selected in the page: a comment begins on them, with the caret in it. Words typed in one begun before
    /// go with it to the new passage.
    pub(super) fn margin_begin(&mut self, id: &SharedString, name: &str, picked: ask::Picked, window: &mut Window, cx: &mut Context<Self>) {
        let anchor = MarginAnchor { quote: picked.quote.trim().to_string(), block: some(picked.block), section: some(picked.section) };
        let text = self.reader_of(id, name).and_then(|held| held.margin.new.as_ref()).map(|new| new.area.read(cx).value().to_string()).unwrap_or_default();
        let (area, typing) = self.margin_area(id, name, Typed::New, &text, window, cx);
        let Some(held) = self.reader_mut(id, name) else { return };
        held.margin.new = Some(NewComment { anchor, area: area.clone(), _typing: typing });
        held.margin.settled = false;
        held.margin.said = None;
        held.notes_on = true;
        area.update(cx, |area, cx| area.focus(window, cx));
        self.margin_typed(id, name, &Typed::New, cx);
        self.margin_marks(id, name, cx);
    }

    /// Esc in an empty comment, or its ×: it goes, and so does what was kept of it.
    pub(super) fn margin_drop_new(&mut self, id: &SharedString, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, name) else { return };
        let page = held.key.clone();
        held.margin.new = None;
        self.keep_words(&new_key(id, &page), "", cx);
        self.margin_leave(id, name, window, cx);
        self.margin_marks(id, name, cx);
        cx.notify();
    }

    /// Hold or Send now on the comment he is writing.
    pub(super) fn margin_send_new(&mut self, id: &SharedString, name: &str, hold: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((anchor, area)) = self.reader_of(id, name).and_then(|held| held.margin.new.as_ref()).map(|new| (new.anchor.clone(), new.area.clone())) else { return };
        let text = area.read(cx).value().trim().to_string();
        if text.is_empty() {
            area.update(cx, |area, cx| area.focus(window, cx));
            return;
        }
        self.margin_comment(id, name, MarginOn::Passage { anchor }, text, hold, Sent::New, window, cx);
    }

    /// Hold or Send now on a reply.
    pub(super) fn margin_send_reply(&mut self, id: &SharedString, name: &str, thread: &str, hold: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(area) = self.reader_of(id, name).and_then(|held| held.margin.replies.get(thread)).map(|(area, _)| area.clone()) else { return };
        let text = area.read(cx).value().trim().to_string();
        if text.is_empty() {
            area.update(cx, |area, cx| area.focus(window, cx));
            return;
        }
        self.margin_comment(id, name, MarginOn::Thread { thread: thread.to_string() }, text, hold, Sent::Reply(thread.to_string()), window, cx);
    }

    #[allow(clippy::too_many_arguments)]
    fn margin_comment(&mut self, id: &SharedString, name: &str, on: MarginOn, text: String, hold: bool, sent: Sent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.reader_of(id, name).map(|held| held.key.clone()) else { return };
        let body = ipc::encode(&CommentMargin { session_id: SessionId::carried(id.to_string()), document: page, on: on.clone(), text: text.clone(), hold });
        let mock = move |kept: &mut MarginThreads| crate::mock_margin::comment(kept, &on, &text, hold).map(|()| None);
        self.margin_post(id, name, "/sessions/margin/comment", body, Box::new(mock), sent, window, cx);
    }

    /// Send held: every comment he holds on the page goes to the agent together.
    pub(super) fn margin_send_held(&mut self, id: &SharedString, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.reader_of(id, name).filter(|held| held.margin.holds()).map(|held| held.key.clone()) else { return };
        let body = ipc::encode(&SendHeldMargin { session_id: SessionId::carried(id.to_string()), document: page });
        let mock = |kept: &mut MarginThreads| crate::mock_margin::send_held(kept).map(|()| None);
        self.margin_post(id, name, "/sessions/margin/send_held", body, Box::new(mock), Sent::Nothing, window, cx);
    }

    /// Make this change, Accept, Reject or Settle on a thread. Settle takes the words in its reply box as the outcome.
    pub(super) fn margin_do(&mut self, id: &SharedString, name: &str, thread: &str, act: MarginAct, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let (act, sent) = match act {
            MarginAct::Settle { .. } => {
                let typed = held.margin.replies.get(thread).map(|(area, _)| area.read(cx).value().trim().to_string()).filter(|said| !said.is_empty());
                let sent = if typed.is_some() { Sent::Reply(thread.to_string()) } else { Sent::Nothing };
                (MarginAct::Settle { outcome: typed }, sent)
            }
            other => (other, Sent::Nothing),
        };
        let newest = self.reader_page(id, name).map_or(1, |tab| tab.read(cx).shown());
        let body = ipc::encode(&ActOnMargin { session_id: SessionId::carried(id.to_string()), thread: thread.to_string(), act: act.clone() });
        let named = thread.to_string();
        let mock = move |kept: &mut MarginThreads| crate::mock_margin::act(kept, &named, &act, newest);
        self.margin_post(id, name, "/sessions/margin/act", body, Box::new(mock), sent, window, cx);
    }

    /// A margin request, answered with the page's threads. On the mock its answer is worked out here, and an Accept
    /// writes the page's next revision as Fleet's does.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn margin_post(
        &mut self,
        id: &SharedString,
        name: &str,
        path: &str,
        body: Result<String, ipc::Unencodable>,
        mock: Box<dyn FnOnce(&mut MarginThreads) -> Result<Option<(String, String)>, String>>,
        sent: Sent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(held) = self.reader_mut(id, name) else { return };
        if held.margin.busy {
            return;
        }
        held.margin.busy = true;
        let page = held.key.clone();
        cx.notify();
        if self.fleet.is_none() {
            let kept = self.row_tabs.entry(id.clone()).or_default().mock_margin.entry(page.clone()).or_insert_with(|| crate::mock_margin::threads(id, &page));
            let answer = mock(kept).map(|edit| (edit, kept.clone()));
            let read = match answer {
                Ok((edit, read)) => {
                    if let Some(edit) = edit {
                        self.margin_mock_accepted(id, name, &edit, window, cx);
                    }
                    Ok(read)
                }
                Err(why) => Err(why),
            };
            self.margin_sent(id, name, sent, read, window, cx);
            return;
        }
        let (job, reader) = (id.clone(), name.to_string());
        self.post(
            path.to_string(),
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<MarginThreads>("MarginThreads", &bytes).map_err(|why| why.to_string()));
                this.margin_sent(&job, &reader, sent, read, window, cx);
            }),
        );
    }

    /// Fleet took the request: the box it sent from empties. A refusal leaves his words where they are.
    fn margin_sent(&mut self, id: &SharedString, name: &str, sent: Sent, read: Result<MarginThreads, String>, window: &mut Window, cx: &mut Context<Self>) {
        if read.is_ok() {
            let page = self.reader_of(id, name).map(|held| held.key.clone()).unwrap_or_default();
            match &sent {
                Sent::New => {
                    if let Some(held) = self.reader_mut(id, name) {
                        held.margin.new = None;
                    }
                    self.keep_words(&new_key(id, &page), "", cx);
                    self.margin_leave(id, name, window, cx);
                }
                Sent::Reply(thread) => {
                    let area = self.reader_of(id, name).and_then(|held| held.margin.replies.get(thread)).map(|(area, _)| area.clone());
                    if let Some(area) = area {
                        area.update(cx, |area, cx| area.set_value(String::new(), window, cx));
                    }
                    self.keep_words(&reply_key(id, &page, thread), "", cx);
                }
                Sent::Nothing => {}
            }
        }
        self.margin_took(id, name, read, window, cx);
    }

    /// The mock Fleet's Accept: the page's next revision is the one shown with the edit in it, and the tab moves to it
    /// where he was reading.
    fn margin_mock_accepted(&mut self, id: &SharedString, name: &str, edit: &(String, String), window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.reader_page(id, name) else { return };
        let (page, shown) = {
            let held = tab.read(cx);
            (held.page().to_string(), held.shown())
        };
        let his = self.row_tabs.get(id).and_then(|tabs| tabs.mock_kept.get(&page)).map(|kept| kept.his.clone()).unwrap_or_default();
        let Ok(html) = crate::mock::page_source(&page, shown, &his).or_else(|_| crate::mock::sealed_revision(&page, shown, &his, &[], &[])) else { return };
        self.row_tabs.entry(id.clone()).or_default().mock_kept.entry(page.clone()).or_default().his.push(html.replacen(&edit.0, &edit.1, 1));
        self.revised_here(id, &page, ipc::ArtifactBy::Agent);
        tab.update(cx, |tab, _| tab.keep_at(shown + 1));
        self.fetch_page(tab, window, cx);
    }

    /// J and K: the next thread or the one before, its passage brought into view and marked.
    pub(super) fn margin_move(&mut self, id: &SharedString, name: &str, down: bool, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let ids: Vec<String> = held.margin.open().map(|one| one.id.clone()).collect();
        if ids.is_empty() || held.margin.settled {
            return;
        }
        let at = held.margin.on.as_ref().and_then(|on| ids.iter().position(|one| one == on));
        let last = ids.len() - 1;
        let at = match (at, down) {
            (None, true) => 0,
            (None, false) => last,
            (Some(at), true) => (at + 1).min(last),
            (Some(at), false) => at.saturating_sub(1),
        };
        if at != held.margin.on.as_ref().and_then(|on| ids.iter().position(|one| one == on)).unwrap_or(usize::MAX) {
            crate::ui_sounds::moved();
        }
        self.margin_on(id, name, &ids[at], cx);
    }

    /// A thread chosen: its passage is brought into view in the page and marked as the one he is on.
    pub(super) fn margin_on(&mut self, id: &SharedString, name: &str, thread: &str, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, name) else { return };
        held.margin.on = Some(thread.to_string());
        held.margin.settled = false;
        held.notes_on = true;
        self.margin_marks(id, name, cx);
        self.margin_reach(id, name, cx);
        cx.notify();
    }

    /// The page the margin stands beside came: its marks are drawn, and the thread he is on is brought into view.
    pub(super) fn margin_page_came(&mut self, id: &SharedString, name: &str, cx: &mut Context<Self>) {
        self.margin_marks(id, name, cx);
        self.margin_reach(id, name, cx);
    }

    /// The keys leave a comment or a reply for what holds the margin: the reader in a tab, the sheet's list in the sheet.
    pub(super) fn margin_leave(&mut self, id: &SharedString, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.sheet_reader(cx).is_some_and(|(job, reader)| job == *id && reader == name) {
            window.focus(&self.review_focus.list, cx);
        } else {
            window.focus(&self.row_focus, cx);
        }
    }

    /// The thread he is on, brought into view: its passage in the page, and the thread in the margin.
    fn margin_reach(&mut self, id: &SharedString, name: &str, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let Some(one) = held.margin.on.as_ref().and_then(|on| held.margin.threads.iter().find(|one| &one.id == on)).cloned() else { return };
        held.margin.reach.replace(Some(one.id.clone()));
        if let Some(tab) = self.reader_page(id, name) {
            let script = ask::reach(&one.id, &one.anchor.quote, one.anchor.block.as_deref().unwrap_or_default());
            tab.update(cx, |tab, cx| tab.run(script, cx));
        }
    }

    /// Enter on the thread J and K are on: the caret goes into its reply.
    pub(super) fn margin_into(&mut self, id: &SharedString, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let area = self.reader_of(id, name).and_then(|held| held.margin.on.as_ref().and_then(|on| held.margin.replies.get(on))).map(|(area, _)| area.clone());
        if let Some(area) = area {
            area.update(cx, |area, cx| area.focus(window, cx));
        }
    }

    /// The Open or the Settled tab in front.
    pub(super) fn margin_tab(&mut self, id: &SharedString, name: &str, settled: bool, cx: &mut Context<Self>) {
        if let Some(held) = self.reader_mut(id, name) {
            held.margin.settled = settled;
            cx.notify();
        }
    }

    /// ⇧⌘↵ sends what he holds on the page whose reader has the keys, or on the page the review sheet shows. False where no
    /// page's reader holds them.
    pub(super) fn margin_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !act.is("margin_send_held") {
            return false;
        }
        // The sheet over the window holds the keys, and the page it shows is the one they send for.
        if let Some((job, name)) = self.sheet_reader(cx).filter(|(job, name)| self.reader_of(job, name).is_some_and(|held| held.margin.holds())) {
            self.margin_send_held(&job, &name, window, cx);
            return true;
        }
        let Some(id) = self.shown.clone() else { return false };
        let in_reader = self.row_focus.contains_focused(window, cx);
        let name = self.row_tabs.get(&id).and_then(|tabs| {
            tabs.readers.iter().find(|(_, held)| held.kind == RowKind::Document && held.margin.holds() && (in_reader || held.margin.typing(window, cx))).map(|(name, _)| name.clone())
        });
        let Some(name) = name else { return false };
        self.margin_send_held(&id, &name, window, cx);
        true
    }

    /// Open on a round of talk in the thread: the page in its tab, or in the review sheet while it asks him to approve it, the
    /// margin's Open tab, and the round's first thread brought into view; a passage of a page sent back is shown in the page.
    pub(super) fn margin_open(&mut self, page: &str, thread: Option<&str>, passage: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        match self.sheet_item(&id, &format!("document:{page}"), cx).filter(|item| item.kind == super::review::Kind::Document) {
            Some(item) => {
                self.open_ask(&item.id, window, cx);
                // The sheet's page is made now, so what the margin runs in it waits there until it has loaded.
                self.sync_panel_pages(window, cx);
            }
            None => {
                self.open_document_tab(page, window, cx);
                self.sync_page_tabs(&id, window, cx);
            }
        }
        self.sync_readers(&id, window, cx);
        let name = super::reader::reader_id(RowKind::Document, page);
        if let Some(thread) = thread {
            self.margin_on(&id, &name, thread, cx);
            return;
        }
        if let (Some(passage), Some(tab)) = (passage.filter(|one| !one.trim().is_empty()), self.reader_page(&id, &name)) {
            let script = ask::show(passage, "");
            tab.update(cx, |tab, cx| tab.run(script, cx));
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{px, size, Bounds, Modifiers, Pixels, Role};

    use super::super::parts_fold::{drawn, redraw, rooted_job};

    /// Every place a `role` named `name` is drawn, the latest last.
    fn all_of(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> Vec<Bounds<Pixels>> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|one| one.role == role && one.name == name).map(|one| one.bounds).collect())
    }

    /// The scenario's page asks him to approve it, as Fleet lists one: here it does not, so it reads in its tab.
    fn no_ask(jobs: &mut Vec<ipc::SessionRecord>) {
        jobs.iter_mut().for_each(|job| job.waiting_for.retain(|item| item.id != "document:quiet"));
    }

    fn reader_name() -> String {
        super::super::reader::reader_id(super::RowKind::Document, "quiet")
    }

    #[gpui::test]
    fn the_sheet_draws_the_margin_beside_a_page_that_asks_and_folds_it_under_a_narrow_one(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "planning-margin", "plan-job", size(px(1728.), px(1080.)), |_| {});
        view.update_in(cx, |view, window, cx| view.margin_open("quiet", None, None, window, cx));
        redraw(cx);
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "a page that asks opens in the review sheet");
        assert!(view.read_with(cx, |view, _| !view.row_tab_list("plan-job").iter().any(|tab| tab.key == "quiet")), "and has no tab");
        let sheet = all_of(cx, Role::Dialog, "Review").pop().expect("the sheet is drawn");
        let wide = all_of(cx, Role::Group, "Margin").pop().expect("the margin is drawn in the sheet");
        assert!(sheet.contains(&wide.origin), "the margin stands in the sheet: {wide:?} in {sheet:?}");
        let width = f32::from(super::super::reader_talk::margin_width(&crate::look::Look::dark()));
        assert!((f32::from(wide.size.width) - width).abs() < 1., "beside the page the margin is {width} wide, not {:?}", wide.size.width);
        cx.simulate_resize(size(px(900.), px(1080.)));
        redraw(cx);
        redraw(cx);
        let narrow = all_of(cx, Role::Group, "Margin").pop().expect("the margin is still drawn");
        assert!(f32::from(narrow.size.width) > width + 1., "under the page the margin takes the page's width, {:?}", narrow.size.width);
        assert!(narrow.top() > wide.top(), "and stands below the page");
    }

    #[gpui::test]
    fn open_on_a_round_of_a_page_that_asks_opens_the_sheet_on_its_first_thread(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "planning-margin", "plan-job", size(px(1728.), px(560.)), |_| {});
        view.update_in(cx, |view, window, cx| view.margin_open("quiet", Some("m5"), None, window, cx));
        redraw(cx);
        redraw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.review_chosen().map(|item| item.id)), Some("document:quiet".to_string()), "the sheet is open on the page");
        let on = view.read_with(cx, |view, _| view.reader_of("plan-job", &reader_name()).map(|held| held.margin.on.clone()));
        assert_eq!(on, Some(Some("m5".to_string())), "the round's thread is the one he is on");
        let margin = all_of(cx, Role::Group, "Margin").pop().expect("the margin is drawn");
        let thread = all_of(cx, Role::Group, "Thread on \u{201c}A Job asking a question waits until the next workday.\u{201d}").pop().expect("m5 is drawn");
        assert!(margin.contains(&thread.origin) && thread.top() < margin.bottom(), "m5 stands in the margin's view: {thread:?} in {margin:?}");
    }

    #[gpui::test]
    fn in_the_sheet_hold_keeps_a_comment_and_send_held_sends_them_together(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "planning-margin", "plan-job", size(px(1728.), px(1080.)), |_| {});
        view.update_in(cx, |view, window, cx| view.margin_open("quiet", None, None, window, cx));
        redraw(cx);
        let (id, name) = (gpui::SharedString::from("plan-job"), reader_name());
        let picked = crate::web::reader::Picked { quote: "Whether a merged pull request is worth a notice at all.".into(), ..Default::default() };
        view.update_in(cx, |view, window, cx| view.margin_begin(&id, &name, picked, window, cx));
        redraw(cx);
        assert!(drawn(cx, Role::TextInput, "Comment"), "a comment begins in the sheet's margin");
        let area = view.read_with(cx, |view, _| view.reader_of(&id, &name).and_then(|held| held.margin.new.as_ref()).map(|new| new.area.clone())).expect("the comment's box");
        view.update_in(cx, |_, window, cx| area.update(cx, |area, cx| area.set_value("Is a merged pull request ever urgent?".to_string(), window, cx)));
        redraw(cx);
        let hold = all_of(cx, Role::Button, "Hold").pop().expect("Hold stands under the comment");
        cx.simulate_click(hold.center(), Modifiers::none());
        redraw(cx);
        let held = |view: &super::JobView| view.reader_of("plan-job", &reader_name()).map(|held| held.margin.threads.iter().filter(|one| one.messages.iter().any(|said| said.held)).count());
        assert_eq!(view.read_with(cx, |view, _| held(view)), Some(2), "Hold keeps it with his other held comment");
        // The app's own keys, as main.rs installs them.
        cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::default(), cx));
        cx.simulate_keystrokes("cmd-shift-enter");
        redraw(cx);
        assert_eq!(view.read_with(cx, |view, _| held(view)), Some(0), "\u{21e7}\u{2318}\u{21b5} sends what he holds on the sheet's page");
        assert!(view.read_with(cx, |view, _| view.review_open()), "and the sheet stays up");
    }

    #[gpui::test]
    fn the_margin_stands_beside_a_wide_page_at_its_width_and_folds_under_a_narrow_one(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "planning-margin", "plan-job", size(px(1728.), px(1080.)), no_ask);
        view.update_in(cx, |view, window, cx| view.margin_open("quiet", None, None, window, cx));
        redraw(cx);
        redraw(cx);
        let wide = all_of(cx, Role::Group, "Margin").pop().expect("the margin is drawn");
        let width = f32::from(super::super::reader_talk::margin_width(&crate::look::Look::dark()));
        assert!((f32::from(wide.size.width) - width).abs() < 1., "beside the page the margin is {width} wide, not {:?}", wide.size.width);
        cx.simulate_resize(size(px(760.), px(1080.)));
        redraw(cx);
        redraw(cx);
        let narrow = all_of(cx, Role::Group, "Margin").pop().expect("the margin is still drawn");
        assert!(f32::from(narrow.size.width) > width + 1., "under the page the margin takes the page's width, {:?}", narrow.size.width);
        assert!(narrow.top() > wide.top(), "and stands below the page");
    }

    #[gpui::test]
    fn settle_sends_the_reply_as_the_outcome_or_none_and_takes_the_passage_mark_away(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "planning-margin", "plan-job", size(px(1728.), px(1080.)), |_| {});
        view.update_in(cx, |view, window, cx| view.margin_open("quiet", Some("m1"), None, window, cx));
        redraw(cx);
        let (id, name) = (gpui::SharedString::from("plan-job"), super::super::reader::reader_id(super::RowKind::Document, "quiet"));
        let marked = |view: &super::JobView, thread: &str| view.reader_of(&id, &name).is_some_and(|held| held.margin.marks().iter().any(|mark| mark.thread == thread));
        assert!(view.read_with(cx, |view, _| marked(view, "m1") && marked(view, "m2")), "the open threads mark their passages");
        view.update_in(cx, |view, window, cx| view.margin_do(&id, &name, "m1", ipc::MarginAct::Settle { outcome: None }, window, cx));
        redraw(cx);
        let area = view.read_with(cx, |view, _| view.reader_of(&id, &name).and_then(|held| held.margin.replies.get("m2")).map(|(area, _)| area.clone())).expect("m2 has its reply box");
        view.update_in(cx, |_, window, cx| area.update(cx, |area, cx| area.set_value("Quiet until my work start".to_string(), window, cx)));
        view.update_in(cx, |view, window, cx| view.margin_do(&id, &name, "m2", ipc::MarginAct::Settle { outcome: None }, window, cx));
        redraw(cx);
        view.read_with(cx, |view, _| {
            let held = view.reader_of(&id, &name).expect("the page's reader");
            let outcome = |thread: &str| held.margin.threads.iter().find(|one| one.id == thread).map(|one| (one.state, one.outcome.clone()));
            assert_eq!(outcome("m1"), Some((ipc::MarginState::Settled, None)), "an empty box sends no outcome, and Fleet asks the agent");
            assert_eq!(outcome("m2"), Some((ipc::MarginState::Settled, Some("Quiet until my work start".to_string()))), "the box's words are the outcome");
            assert!(!marked(view, "m1") && !marked(view, "m2"), "a settled thread marks nothing in the page");
        });
    }

    #[gpui::test]
    fn open_on_a_round_brings_its_first_thread_into_the_margins_view(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "planning-margin", "plan-job", size(px(1728.), px(560.)), no_ask);
        view.update_in(cx, |view, window, cx| view.margin_open("quiet", Some("m5"), None, window, cx));
        redraw(cx);
        redraw(cx);
        let on = view.read_with(cx, |view, _| view.reader_of("plan-job", &super::super::reader::reader_id(super::RowKind::Document, "quiet")).map(|held| (held.margin.on.clone(), held.margin.place_of("m5"))));
        assert_eq!(on, Some((Some("m5".to_string()), Some(5))), "the round's thread is the one he is on");
        let margin = all_of(cx, Role::Group, "Margin").pop().expect("the margin is drawn");
        let thread = all_of(cx, Role::Group, "Thread on \u{201c}A Job asking a question waits until the next workday.\u{201d}").pop().expect("m5 is drawn");
        assert!(margin.contains(&thread.origin) && thread.top() < margin.bottom(), "m5 stands in the margin's view: {thread:?} in {margin:?}");
    }
}
