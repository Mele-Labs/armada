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
        window.focus(&self.row_focus, cx);
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
                    window.focus(&self.row_focus, cx);
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
        self.edited_here(id, &page);
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
        let Some(one) = held.margin.threads.iter().find(|one| one.id == thread).cloned() else { return };
        self.margin_marks(id, name, cx);
        if let Some(tab) = self.reader_page(id, name) {
            let script = ask::reach(&one.id, &one.anchor.quote, one.anchor.block.as_deref().unwrap_or_default());
            tab.update(cx, |tab, cx| tab.run(script, cx));
        }
        cx.notify();
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

    /// ⇧⌘↵ sends what he holds on the page whose reader has the keys. False where no page's reader holds them.
    pub(super) fn margin_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !act.is("queue_session") {
            return false;
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

    /// Open on a round of talk in the thread: the page in its tab, the margin's Open tab, and the round's first thread
    /// brought into view; a passage of a page sent back is shown in the page.
    pub(super) fn margin_open(&mut self, page: &str, thread: Option<&str>, passage: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        self.open_document_tab(page, window, cx);
        self.sync_page_tabs(&id, window, cx);
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
