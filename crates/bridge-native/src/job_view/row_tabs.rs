//! The tabs a Job opens beside its thread for one thing it put to him or ran: a draft, a document, a
//! walk or a subagent, placed as Armada Classic's `focus.ts` places them. An open tab of the same thing comes to the
//! front; else the tab joins the thread's pane, in front. A walk the Job owes him arrives behind the
//! tab in front and pulses until he has had it in front (`WalkTabsSync`); one he closed is not opened
//! again, and one settled takes its tab with it.
//!
//! **Departures (ruling (b), 11 Nov):** a walk the Job never showed gets no tab, where Classic draws
//! an empty heading and a live Open; a draft whose row has gone takes its tab away and the front moves
//! as a closed tab's does, where Classic leaves a blank frame. A press on such a draft opens no tab.
//! A subagent's tab goes the same way once the Job no longer holds the subagent. All follow "an
//! empty slot stays empty".
//!
//! A walk of a native screen (`armada://walk/`) is a name, a brief and Open. **A walk of a web page draws the
//! page in its tab with the walk's bar above it** (`web_walk.rs`), as Classic's live walk tab does, in a
//! WKWebView placed over the frame the tab reports (`crate::web`). It has a tab as a native walk has, and a
//! press on the walk he is owed brings it forward.

use std::collections::{HashMap, HashSet};

use gpui::{div, prelude::*, AnyElement, Context, Entity, FontWeight, Role, SharedString, Subscription, Window};
use ipc::{SessionRecord, SessionRow, WaitingActKind, WaitingSource};

use super::acts::{button, Tone, SHEET};
use super::web_walk::{WebWalkEvent, WebWalkTab};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The address a walk of a native screen is shown on (`NATIVE_WALK`).
const NATIVE_WALK: &str = "armada://walk/";

/// What a tab beside the thread holds.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum RowKind {
    Draft,
    Document,
    Walk,
    Subagent,
}

/// The word a kept tab names its kind by; a walk's tab is not kept.
pub(super) fn kind_id(kind: RowKind) -> Option<&'static str> {
    match kind {
        RowKind::Draft => Some("draft"),
        RowKind::Document => Some("document"),
        RowKind::Subagent => Some("subagent"),
        RowKind::Walk => None,
    }
}

impl RowKind {
    /// The kind's glyph and the word a tab with no name of its own reads (`KINDS`).
    pub(super) fn look(self) -> (&'static str, &'static str) {
        match self {
            RowKind::Draft => ("notebook-pen", "Draft"),
            RowKind::Document => ("file-text", "Document"),
            RowKind::Walk => ("eye", "Walk"),
            RowKind::Subagent => ("drone", "Subagent"),
        }
    }
}

/// One tab: its kind, the thing it holds (a draft's row, a document's page, a walk's address) and
/// its name.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RowTab {
    pub kind: RowKind,
    pub key: String,
    pub title: String,
    /// Whether it has been in front of him; a walk's tab pulses until it has.
    pub seen: bool,
}

/// What Open on a native walk answered.
#[derive(Clone, Debug, PartialEq)]
enum Opened {
    Pending,
    Open,
    Refused(String),
}

/// One Job's tabs beside its thread.
#[derive(Default)]
pub(super) struct RowTabs {
    tabs: Vec<RowTab>,
    /// The tab in front, by its place in `tabs`; none is the thread.
    front: Option<usize>,
    /// Walks that had a tab once: one closed with its × does not come back (`arrivedBefore`).
    arrived: HashSet<String>,
    opened: HashMap<String, Opened>,
    /// The web page of each walk of a web page that has a tab, by address: kept while the walk is owed, so its page, scroll and
    /// step stay while its tab is behind another or closed (Classic parks the view hidden).
    web: HashMap<String, (Entity<WebWalkTab>, Subscription)>,
}

impl RowTabs {
    fn at(&self, kind: RowKind, key: &str) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.kind == kind && tab.key == key)
    }

    /// The thing's tab comes to the front, or joins the strip in front (`focusDraft`).
    fn focus(&mut self, kind: RowKind, key: &str, title: String) {
        let at = self.at(kind, key).unwrap_or_else(|| {
            self.tabs.push(RowTab { kind, key: key.to_string(), title, seen: false });
            self.tabs.len() - 1
        });
        self.show(Some(at));
    }

    fn show(&mut self, front: Option<usize>) {
        self.front = front;
        if let Some(tab) = front.and_then(|at| self.tabs.get_mut(at)) {
            tab.seen = true;
        }
    }

    /// The tab goes; where it was in front, the one after it comes forward, else the one before
    /// (`afterLeaving`), and the thread before the first.
    fn close(&mut self, at: usize) {
        if at >= self.tabs.len() {
            return;
        }
        self.tabs.remove(at);
        self.front = match self.front {
            Some(front) if front == at => {
                if at < self.tabs.len() {
                    Some(at)
                } else {
                    at.checked_sub(1)
                }
            }
            Some(front) if front > at => Some(front - 1),
            other => other,
        };
        if let Some(front) = self.front {
            self.show(Some(front));
        }
    }

    /// Every tab `stays` refuses goes, as its × would take it.
    fn keep(&mut self, stays: impl Fn(&RowTab) -> bool) {
        let mut at = 0;
        while at < self.tabs.len() {
            if stays(&self.tabs[at]) {
                at += 1;
            } else {
                self.close(at);
            }
        }
    }

    /// The walks owed now, each with its name: a settled walk takes its tab with it, and one owed for
    /// the first time arrives behind the tab in front. True where a walk arrived.
    fn sync_walks(&mut self, owed: &[(String, String)]) -> bool {
        self.keep(|tab| tab.kind != RowKind::Walk || owed.iter().any(|(url, _)| *url == tab.key));
        let mut arrived = false;
        for (url, title) in owed {
            if self.arrived.insert(url.clone()) && self.at(RowKind::Walk, url).is_none() {
                self.tabs.push(RowTab { kind: RowKind::Walk, key: url.clone(), title: title.clone(), seen: false });
                arrived = true;
            }
        }
        arrived
    }
}

/// A walk the Job showed, as `walksOf` reads its `window` artifact: its name and brief.
struct ShownWalk {
    title: String,
    brief: Option<String>,
    /// The scripted steps of a web page's walk (`show_window`'s `steps`), as the ledger keeps them.
    steps: Vec<ipc::WalkStep>,
}

/// Whether a walk is of a native screen, which Fleet starts in a window of its own, and not of a web page.
fn is_native(url: &str) -> bool {
    url.starts_with(NATIVE_WALK)
}

/// A page with no name of its own is named by the end of its path, as `nameOfPage` names it.
fn name_of_page(url: &str) -> String {
    url.split(['?', '#']).next().unwrap_or(url).trim_end_matches('/').rsplit('/').next().filter(|end| !end.is_empty()).unwrap_or(url).to_string()
}

fn shown_walk(record: &SessionRecord, url: &str) -> Option<ShownWalk> {
    let one = record.attachments.iter().find(|one| one.kind == "artifact" && one.target == url && one.detail.get("form").map(String::as_str) == Some("window"))?;
    let title = one.detail.get("title").filter(|title| title.as_str() != url && !title.is_empty()).cloned().unwrap_or_else(|| name_of_page(url));
    let brief = one.detail.get("brief").map(|brief| brief.trim().to_string()).filter(|brief| !brief.is_empty());
    let steps = one.detail.get("steps").and_then(|steps| ipc::decode::<Vec<ipc::WalkStep>>("walk steps", steps.as_bytes()).ok()).unwrap_or_default();
    Some(ShownWalk { title, brief, steps })
}

/// The walks the Job owes him, each with its tab's name: a walk is owed while Fleet's waiting
/// item for it stands, and has a tab only where the Job showed it (`owedWalks`).
fn owed_walks(record: &SessionRecord) -> Vec<(String, String)> {
    record
        .waiting_for
        .iter()
        .filter(|one| one.source == WaitingSource::Walk)
        .filter_map(|one| one.act.as_ref().filter(|act| act.kind == WaitingActKind::Walk).map(|act| act.target.clone()))
        .filter_map(|url| shown_walk(record, &url).map(|walk| (url, walk.title)))
        .collect()
}

impl JobView {
    fn row_tabs_of(&mut self, id: &SharedString) -> &mut RowTabs {
        self.row_tabs.entry(id.clone()).or_default()
    }

    /// Whether the Job has subagent `subagent` open as a tab beside its thread.
    pub(super) fn subagent_tab_open(&self, job: &str, subagent: &str) -> bool {
        self.row_tabs.get(job).is_some_and(|tabs| tabs.at(RowKind::Subagent, subagent).is_some())
    }

    /// The tab Job `id` holds for this thing.
    pub(super) fn row_tab_of(&self, id: &str, kind: RowKind, key: &str) -> Option<&RowTab> {
        let tabs = self.row_tabs.get(id)?;
        tabs.at(kind, key).and_then(|at| tabs.tabs.get(at))
    }

    /// Every tab Job `id` holds beside its thread, in the order they were opened.
    pub(super) fn row_tab_list(&self, id: &str) -> Vec<RowTab> {
        self.row_tabs.get(id).map(|tabs| tabs.tabs.clone()).unwrap_or_default()
    }

    /// A tab has been in front of him: a walk's stops pulsing.
    pub(super) fn row_tab_seen(&mut self, id: &SharedString, kind: RowKind, key: &str) {
        let tabs = self.row_tabs_of(id);
        if let Some(tab) = tabs.at(kind, key).and_then(|at| tabs.tabs.get_mut(at)) {
            tab.seen = true;
        }
    }

    /// A tab that closed lets go of what it held, so it is not opened again of itself (a walk's stays shut).
    pub(super) fn row_tab_let_go(&mut self, id: &SharedString, kind: RowKind, key: &str) {
        let tabs = self.row_tabs_of(id);
        if let Some(at) = tabs.at(kind, key) {
            tabs.close(at);
        }
    }

    /// A tab Job `id` held comes to the front of its pane, which takes the keys.
    fn row_tab_front(&mut self, id: &SharedString, kind: RowKind, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_row_tabs_into_panes(id, cx);
        let (kind_name, key) = (super::panes_place::row_kind_name(kind), key.to_string());
        let arrangement = self.arrangement_of(id).cloned();
        let found = arrangement.as_ref().and_then(|arrangement| super::pane_tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == kind_name && tab.row.as_deref() == Some(key.as_str())).map(|tab| tab.id.clone()));
        if let Some(tab) = found {
            self.look_in(id, move |arrangement| super::pane_tree::activate(arrangement, &tab), cx);
        }
        self.keys_to_front(window, cx);
    }

    /// A draft's card as a tab named for the draft (`openDraftCard`). A draft whose row has gone
    /// opens no tab.
    pub(super) fn open_draft_tab(&mut self, row: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(title) = self.thread(&id, cx).and_then(|held| super::review::draft_row(&held.items, row)).and_then(|found| match found {
            SessionRow::Draft { title, .. } => Some(title.clone()),
            _ => None,
        }) else {
            return;
        };
        self.row_tabs_of(&id).focus(RowKind::Draft, row, title);
        self.row_tab_front(&id, RowKind::Draft, row, window, cx);
    }

    /// The page a Job owes him alone, read in its own tab (`soleDocument`).
    pub(super) fn sole_document(&self, id: &str, cx: &gpui::App) -> Option<String> {
        match self.record(id, cx)?.waiting_for.as_slice() {
            [only] if only.source == WaitingSource::Document => Some(only.id.strip_prefix("document:").unwrap_or(&only.id).to_string()),
            _ => None,
        }
    }

    /// A page as a tab named for it (`focusDocument`). A page whose card has gone opens no tab.
    pub(super) fn open_document_tab(&mut self, page: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(title) = self.thread(&id, cx).and_then(|held| super::review::document_card(&held.items, page)).map(|card| card.title.clone()) else { return };
        self.row_tabs_of(&id).focus(RowKind::Document, page, title);
        self.row_tab_front(&id, RowKind::Document, page, window, cx);
    }

    /// A subagent's thread as a tab named for its task (`focusSubagent`), from a drone on its group.
    /// A subagent the Job no longer holds opens no tab.
    pub(super) fn open_subagent_tab(&mut self, subagent: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(one) = self.record(&id, cx).and_then(|record| record.attachments.iter().find(|one| one.kind == "subagent" && one.target == subagent)).map(super::ledger::subagent_of) else { return };
        self.watch_subagent(&one, cx);
        self.row_tabs_of(&id).focus(RowKind::Subagent, subagent, one.task);
        self.row_tab_front(&id, RowKind::Subagent, subagent, window, cx);
    }

    /// A press on a walk he is owed (`openJobWalk`): a native walk starts its window at once, and its
    /// tab comes forward only where Fleet does not start it.
    pub(super) fn press_walk(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let url = url.to_string();
        if is_native(&url) {
            self.open_walk(&url, false, window, cx);
        } else {
            // A web page's walk is drawn in its own tab, so a press brings the tab forward (`focusWalk`).
            self.bring_walk_forward(&url, window, cx);
        }
    }

    fn bring_walk_forward(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(title) = self.record(&id, cx).and_then(|record| shown_walk(record, url)).map(|walk| walk.title) else { return };
        let tabs = self.row_tabs_of(&id);
        tabs.arrived.insert(url.to_string());
        tabs.focus(RowKind::Walk, url, title);
        self.row_tab_front(&id, RowKind::Walk, url, window, cx);
    }

    /// The keys go to what is in front of the pane that has the focus: a row's tab body, the thread's message box.
    fn keys_to_front(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let arrangement = self.arrangement().into_owned();
        let front = super::pane_tree::pane_by_id(&arrangement, &arrangement.focus).and_then(super::panes_place::front_of).map(|tab| tab.kind.clone());
        match front.as_deref() {
            Some("thread") => self.to_composer(window, cx),
            Some(kind) if super::panes_place::row_kind_of(kind).is_some() => window.focus(&self.row_focus, cx),
            _ => {}
        }
        cx.notify();
    }

    /// Open on a native walk: Fleet starts the Job's own build of the native app on it, in a window
    /// of its own (`POST /sessions/window/open`). `in_tab`: pressed in the walk's tab, which says
    /// what Fleet answered.
    fn open_walk(&mut self, url: &str, in_tab: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if in_tab {
            self.row_tabs_of(&id).opened.insert(url.to_string(), Opened::Pending);
            cx.notify();
        }
        let key = url.to_string();
        let done: super::wire::Done = Box::new(move |this, answer, window, cx| {
            let opened = match answer {
                Ok(_) => Opened::Open,
                Err(why) => Opened::Refused(why),
            };
            if !in_tab {
                if matches!(opened, Opened::Refused(_)) {
                    this.bring_walk_forward(&key, window, cx);
                }
                return;
            }
            this.row_tabs_of(&id).opened.insert(key, opened);
            cx.notify();
        });
        if self.fleet.is_none() {
            let answer = crate::mock::walk_refused(url).map_or(Ok(Vec::new()), Err);
            done(self, answer, window, cx);
            return;
        }
        let body = ipc::encode(&ipc::OpenNativeWalk { session_id: ipc::SessionId::carried(self.shown.clone().unwrap_or_default().to_string()), url: url.to_string() });
        self.post("/sessions/window/open".into(), body, window, cx, done);
    }

    /// The walks the shown Job owes him get their tabs; a closed thread comes back when one arrives
    /// (`reopenThread`). A draft or page whose row has gone from the thread takes its tab away, as a
    /// settled walk does.
    pub(super) fn sync_walk_tabs(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let subagents: HashSet<String> = self.record(id, cx).map_or_else(HashSet::new, |record| record.attachments.iter().filter(|one| one.kind == "subagent").map(|one| one.target.clone()).collect());
        if let Some(held) = self.thread(id, cx) {
            let rows: HashSet<(RowKind, String)> = self.row_tabs.get(id).map_or_else(HashSet::new, |tabs| {
                tabs.tabs
                    .iter()
                    .filter(|tab| match tab.kind {
                        RowKind::Draft => super::review::draft_row(&held.items, &tab.key).is_some(),
                        RowKind::Document => super::review::document_card(&held.items, &tab.key).is_some(),
                        RowKind::Walk => true,
                        RowKind::Subagent => subagents.contains(&tab.key),
                    })
                    .map(|tab| (tab.kind, tab.key.clone()))
                    .collect()
            });
            self.row_tabs_of(id).keep(|tab| rows.contains(&(tab.kind, tab.key.clone())));
        }
        self.restore_row_front(id, cx);
        let Some(owed) = self.record(id, cx).map(owed_walks) else { return };
        self.sync_web_walks(id, &owed, window, cx);
        if self.row_tabs_of(id).sync_walks(&owed) {
            self.reopen_thread(id, cx);
        }
        self.sync_row_tabs_into_panes(id, cx);
    }

    /// The panes hold a tab for each thing Job `id` holds one for, and none for what it let go of. A tab that arrives joins
    /// the thread's pane behind the one in front; one that goes is closed as its × would close it. Neither is a step back.
    pub(super) fn sync_row_tabs_into_panes(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        self.ensure_placed(id, cx);
        let held = self.row_tab_list(id);
        let Some(arrangement) = self.arrangement_of(id) else { return };
        let in_panes: Vec<(String, String, String)> = super::pane_tree::panes_of(&arrangement.root)
            .into_iter()
            .flat_map(|pane| pane.tabs.iter())
            .filter(|tab| super::panes_place::row_kind_of(&tab.kind).is_some())
            .filter_map(|tab| Some((tab.id.clone(), tab.kind.clone(), tab.row.clone()?)))
            .collect();
        let missing: Vec<(&'static str, String, String)> = held
            .iter()
            .filter(|one| !in_panes.iter().any(|(_, kind, key)| *kind == super::panes_place::row_kind_name(one.kind) && *key == one.key))
            .map(|one| (super::panes_place::row_kind_name(one.kind), one.key.clone(), one.title.clone()))
            .collect();
        let gone: Vec<String> = in_panes.iter().filter(|(_, kind, key)| !held.iter().any(|one| super::panes_place::row_kind_name(one.kind) == kind && one.key == *key)).map(|(tab, _, _)| tab.clone()).collect();
        if missing.is_empty() && gone.is_empty() {
            return;
        }
        self.look_in(
            id,
            move |was| {
                let mut next = was.clone();
                for (kind, key, title) in &missing {
                    let extras = super::pane_tree::Extras { label: Some(title.clone()), row: Some(key.clone()), ..Default::default() };
                    let focus = next.focus.clone();
                    let holder = super::pane_tree::holder_of(&next, "thread").map(|pane| (pane.id.clone(), pane.active.clone()));
                    let pane = holder.as_ref().map_or_else(|| focus.clone(), |(pane, _)| pane.clone());
                    next = super::pane_tree::add_tab(&next, &pane, kind, extras).into_owned();
                    if let Some((_, Some(active))) = &holder {
                        next = super::pane_tree::activate(&next, active).into_owned();
                    }
                    next.focus = focus;
                }
                for tab in &gone {
                    next = super::pane_tree::close_tab(&next, tab).into_owned();
                }
                std::borrow::Cow::Owned(next)
            },
            cx,
        );
    }

    /// Each walk of a web page the Job owes gets its page and bar once, where its tab first arrives; a settled one takes
    /// its page with it (Classic disposes the view as the walk leaves).
    fn sync_web_walks(&mut self, id: &SharedString, owed: &[(String, String)], window: &mut Window, cx: &mut Context<Self>) {
        self.row_tabs_of(id).web.retain(|url, _| owed.iter().any(|(owed, _)| owed == url));
        let manifest = self.record(id, cx).and_then(|record| record.manifest_id.as_ref().map(|manifest| manifest.as_str().to_string()));
        for (url, title) in owed.iter().filter(|(url, _)| !is_native(url)) {
            if self.row_tabs_of(id).web.contains_key(url) {
                continue;
            }
            let Some(walk) = self.record(id, cx).and_then(|record| shown_walk(record, url)) else { continue };
            let (job, address, title) = (id.clone(), url.clone(), title.clone());
            let keys = self.row_focus.clone();
            let tab = cx.new(|cx| WebWalkTab::new(job, address, title, walk.steps, manifest.as_deref(), keys, cx));
            let approving = cx.subscribe_in(&tab, window, |this, tab, _: &WebWalkEvent, window, cx| {
                let (job, url) = (tab.read(cx).job().clone(), tab.read(cx).url().to_string());
                this.approve_web_walk(&job, &url, window, cx);
            });
            self.row_tabs_of(id).web.insert(url.clone(), (tab, approving));
        }
    }

    /// Approve on a web page's walk, as the walk bar sends it: the line Fleet's `walk_approval` reads, to the Job. A Job
    /// with no Fleet behind it (the mock) is sent nothing.
    fn approve_web_walk(&mut self, job: &SharedString, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            return;
        }
        let body = ipc::SendSessionMessageBody {
            message: ipc::SendSessionMessage {
                session_id: ipc::SessionId::carried(job.to_string()),
                text: crate::walk::fleet::approved(url, None),
                attachments: Vec::new(),
                mentions: Vec::new(),
                sketches: Vec::new(),
            },
            aim: None,
        };
        let done: super::wire::Done = Box::new(|_, _, _, _| {});
        self.post("/sessions/message".into(), ipc::encode(&body), window, cx, done);
    }

    /// The tab that was in front of the thread's pane when he left, once the thread is held: it opens again
    /// where its draft, page or subagent is still the Job's, and is let go where it is not.
    fn restore_row_front(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        if self.thread(id, cx).is_none() {
            return;
        }
        let Some((kind, key)) = self.place.wants_row.remove(id) else { return };
        let held = self.thread(id, cx).map(|held| &held.items);
        let (kind, title) = match kind.as_str() {
            "draft" => match held.and_then(|items| super::review::draft_row(items, &key)) {
                Some(SessionRow::Draft { title, .. }) => (RowKind::Draft, title.clone()),
                _ => return,
            },
            "document" => match held.and_then(|items| super::review::document_card(items, &key)) {
                Some(card) => (RowKind::Document, card.title.clone()),
                None => return,
            },
            "subagent" => {
                let Some(one) = self.record(id, cx).and_then(|record| record.attachments.iter().find(|one| one.kind == "subagent" && one.target == key)).map(super::ledger::subagent_of) else { return };
                self.watch_subagent(&one, cx);
                (RowKind::Subagent, one.task)
            }
            _ => return,
        };
        self.row_tabs_of(id).focus(kind, &key, title);
        // The tab joins the thread's pane as any other does and comes to the front of the pane that holds it.
        self.sync_row_tabs_into_panes(id, cx);
        let (kind_name, key) = (super::panes_place::row_kind_name(kind), key.to_string());
        self.restore_in(id, move |was| {
            let found = super::pane_tree::panes_of(&was.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == kind_name && tab.row.as_deref() == Some(key.as_str()));
            match found {
                Some(tab) => super::pane_tree::activate(was, &tab.id),
                None => std::borrow::Cow::Borrowed(was),
            }
        });
    }

    // ---- drawing -----------------------------------------------------------------------

    /// The body of the tab in front, where it is not the thread. **What has gone draws nothing**: a
    /// draft's row or a document's card no longer in the thread, a walk the Job never showed, or a
    /// subagent not yet read.
    pub(super) fn row_tab_body(&self, id: &SharedString, tab: &RowTab, look: &Look, window: &mut Window, cx: &Context<Self>) -> AnyElement {
        let Some(page) = self.pages.get(id) else { return div().id("row-tab-body").track_focus(&self.row_focus).size_full().into_any_element() };
        let now = chrono::Local::now();
        let card = match tab.kind {
            RowKind::Draft => self.thread(id, cx).and_then(|held| super::review::draft_row(&held.items, &tab.key)).and_then(|row| match row {
                SessionRow::Draft { id: row, at, title, text, before, settled, change, .. } => {
                    Some(self.draft(SHEET, row, at, title, text, before.as_deref(), settled.as_ref(), *change, false, page, look, now, cx))
                }
                _ => None,
            }),
            RowKind::Document => self.thread(id, cx).and_then(|held| super::review::document_card(&held.items, &tab.key)).map(|card| self.document(SHEET, card, page, look, now, cx)),
            // A web page's walk is its bar over its page, edge to edge; a native walk's is its name, brief and Open.
            RowKind::Walk if !is_native(&tab.key) => {
                let web = self.row_tabs.get(id).and_then(|tabs| tabs.web.get(&tab.key)).map(|(web, _)| web.clone());
                return div().id("row-tab-body").size_full().children(web).into_any_element();
            }
            RowKind::Walk => self.record(id, cx).and_then(|record| shown_walk(record, &tab.key)).map(|walk| self.walk_body(id, &tab.key, walk, look, cx)),
            // The subagent's thread draws its own padding, as the reading sheet's does.
            RowKind::Subagent => {
                let thread = self.subagent_thread(id, &tab.key, window, cx);
                return div().id("row-tab-body").track_focus(&self.row_focus).size_full().overflow_y_scroll().children(thread).into_any_element();
            }
        };
        match card {
            Some(card) => div().id("row-tab-body").track_focus(&self.row_focus).size_full().overflow_y_scroll().p(look.length(token::SPACE_4)).child(card).into_any_element(),
            None => div().id("row-tab-body").track_focus(&self.row_focus).size_full().into_any_element(),
        }
    }

    /// A walk of a native screen (`NativeWalk`): its name, its brief, Open, and what Fleet answered.
    fn walk_body(&self, id: &SharedString, url: &str, walk: ShownWalk, look: &Look, cx: &Context<Self>) -> AnyElement {
        let opened = self.row_tabs.get(id).and_then(|tabs| tabs.opened.get(url)).cloned();
        let pending = opened == Some(Opened::Pending);
        let key = url.to_string();
        let said = match opened {
            Some(Opened::Open) => Some(labelled(div().id("walk-said").text_color(look.color(token::FG_MUTED)).child("Open in its own window"), Role::Status, "Open in its own window").into_any_element()),
            // Classic's `job-walk__native-said` draws a refusal in the same muted words as an open.
            Some(Opened::Refused(why)) => Some(labelled(div().id("walk-said").flex_1().min_w_0().text_color(look.color(token::FG_MUTED)).child(SharedString::from(why.clone())), Role::Alert, why).into_any_element()),
            _ => None,
        };
        let open = labelled(
            button(look, "walk-open", "Open", Tone::Primary, false, None, !pending).when(!pending, |button| {
                button.on_click(cx.listener(move |this, _, window, cx| this.open_walk(&key, true, window, cx)))
            }),
            Role::Button,
            "Open",
        );
        labelled(
            div()
                .id("walk-body")
                .flex()
                .flex_col()
                .p(look.length(token::SPACE_2))
                .max_w(look.length(token::SPACE_8) * 20.)
                .gap(look.length(token::SPACE_3))
                .child(
                    div()
                        .text_size(look.length(token::TEXT_LG))
                        .line_height(look.length(token::LEADING_LG))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(SharedString::from(walk.title.clone())),
                )
                .children(walk.brief.map(|brief| {
                    div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(brief))
                }))
                .child(div().flex().items_center().gap(look.length(token::SPACE_3)).text_size(look.length(token::TEXT_SM)).child(open).children(said)),
            Role::Region,
            walk.title,
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(kind: RowKind, key: &str) -> RowTab {
        RowTab { kind, key: key.into(), title: key.into(), seen: false }
    }

    #[test]
    fn a_thing_with_a_tab_brings_it_forward_and_does_not_open_another() {
        let mut tabs = RowTabs::default();
        tabs.focus(RowKind::Draft, "dra-1", "Comment".into());
        tabs.focus(RowKind::Document, "design", "Design".into());
        tabs.focus(RowKind::Draft, "dra-1", "Comment".into());
        assert_eq!(tabs.tabs.len(), 2);
        assert_eq!(tabs.front, Some(0));
    }

    #[test]
    fn closing_the_tab_in_front_brings_the_next_forward_else_the_one_before_else_the_thread() {
        let mut tabs = RowTabs { tabs: vec![tab(RowKind::Draft, "a"), tab(RowKind::Draft, "b"), tab(RowKind::Draft, "c")], front: Some(1), ..Default::default() };
        tabs.close(1);
        assert_eq!(tabs.front, Some(1), "c comes forward");
        tabs.close(1);
        assert_eq!(tabs.front, Some(0), "a, the one before");
        tabs.close(0);
        assert_eq!(tabs.front, None, "the thread");
    }

    #[test]
    fn closing_a_tab_behind_keeps_the_one_in_front() {
        let mut tabs = RowTabs { tabs: vec![tab(RowKind::Draft, "a"), tab(RowKind::Draft, "b")], front: Some(1), ..Default::default() };
        tabs.close(0);
        assert_eq!(tabs.front, Some(0));
        assert_eq!(tabs.tabs[0].key, "b");
    }

    #[test]
    fn an_owed_walk_arrives_behind_once_and_a_settled_one_takes_its_tab() {
        let mut tabs = RowTabs::default();
        let owed = vec![("armada://walk/a".to_string(), "A".to_string())];
        assert!(tabs.sync_walks(&owed));
        assert_eq!((tabs.tabs.len(), tabs.front), (1, None), "behind the thread");
        tabs.close(0);
        assert!(!tabs.sync_walks(&owed), "closed with its ×, it stays closed");
        assert!(tabs.tabs.is_empty());
        let mut tabs = RowTabs::default();
        tabs.sync_walks(&owed);
        tabs.sync_walks(&[]);
        assert!(tabs.tabs.is_empty(), "settled");
    }

    #[test]
    fn a_draft_whose_row_has_gone_takes_its_tab_and_the_next_comes_forward() {
        let mut tabs = RowTabs { tabs: vec![tab(RowKind::Draft, "a"), tab(RowKind::Draft, "gone"), tab(RowKind::Walk, "w")], front: Some(1), ..Default::default() };
        tabs.keep(|tab| tab.key != "gone");
        assert_eq!(tabs.tabs.iter().map(|tab| tab.key.as_str()).collect::<Vec<_>>(), ["a", "w"]);
        assert_eq!(tabs.front, Some(1));
    }

    #[test]
    fn a_walk_of_a_web_page_is_owed_a_tab_as_a_walk_of_a_native_screen_is() {
        let jobs = crate::mock::jobs("job-web-walk").expect("decodes");
        let record = jobs.iter().find(|one| one.id.as_str() == "mock-01").expect("the first Job");
        let owed = owed_walks(record);
        assert_eq!(owed.len(), 1, "the native walks are gone from this Job, the web walk is owed");
        let (url, title) = &owed[0];
        assert!(url.starts_with("file://") && url.ends_with("/fixtures/web-walk-page.html"), "{url}");
        assert!(!is_native(url));
        assert_eq!(title, "Orders page");
        let shown = shown_walk(record, url).expect("the Job showed it");
        assert_eq!(shown.steps.len(), 6);
        assert_eq!(shown.steps[4].press.as_ref().map(|target| target.name.as_deref()), Some(Some("Cancel")), "the form is shut before Export");
        assert_eq!(shown.steps[2].press.as_ref().map(|target| target.role.as_str()), Some("button"));
        let mut tabs = RowTabs::default();
        assert!(tabs.sync_walks(&owed), "it arrives behind the thread");
        assert_eq!(tabs.tabs.len(), 1);
        assert_eq!(tabs.tabs[0].kind, RowKind::Walk);
    }

    #[test]
    fn a_native_walk_is_told_from_a_web_walk_by_its_address() {
        assert!(is_native("armada://walk/mergeLine"));
        assert!(!is_native("https://example.com/orders"));
        assert!(!is_native("file:///tmp/page.html"));
    }

    #[test]
    fn a_page_with_no_name_is_named_by_the_end_of_its_path() {
        assert_eq!(name_of_page("armada://walk/mergeLine"), "mergeLine");
        assert_eq!(name_of_page("armada://walk/tabs/"), "tabs");
    }
}
