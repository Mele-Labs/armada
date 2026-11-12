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
//! Only a walk of a native screen (`armada://walk/`) has a tab here: a web page's walk waits for the
//! embedded browser.

use std::collections::{HashMap, HashSet};

use gpui::{div, prelude::*, AnimationExt, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{SessionRecord, SessionRow, WaitingActKind, WaitingSource};

use super::acts::{button, Tone, SHEET};
use super::rows::{glyph, Size};
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

impl RowKind {
    /// The kind's glyph and the word a tab with no name of its own reads (`KINDS`).
    fn look(self) -> (&'static str, &'static str) {
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
}

/// A page with no name of its own is named by the end of its path, as `nameOfPage` names it.
fn name_of_page(url: &str) -> String {
    url.split(['?', '#']).next().unwrap_or(url).trim_end_matches('/').rsplit('/').next().filter(|end| !end.is_empty()).unwrap_or(url).to_string()
}

fn shown_walk(record: &SessionRecord, url: &str) -> Option<ShownWalk> {
    let one = record.attachments.iter().find(|one| one.kind == "artifact" && one.target == url && one.detail.get("form").map(String::as_str) == Some("window"))?;
    let title = one.detail.get("title").filter(|title| title.as_str() != url && !title.is_empty()).cloned().unwrap_or_else(|| name_of_page(url));
    let brief = one.detail.get("brief").map(|brief| brief.trim().to_string()).filter(|brief| !brief.is_empty());
    Some(ShownWalk { title, brief })
}

/// The native walks the Job owes him, each with its tab's name: a walk is owed while Fleet's waiting
/// item for it stands, and has a tab only where the Job showed it (`owedWalks`).
fn owed_walks(record: &SessionRecord) -> Vec<(String, String)> {
    record
        .waiting_for
        .iter()
        .filter(|one| one.source == WaitingSource::Walk)
        .filter_map(|one| one.act.as_ref().filter(|act| act.kind == WaitingActKind::Walk).map(|act| act.target.clone()))
        .filter(|url| url.starts_with(NATIVE_WALK))
        .filter_map(|url| shown_walk(record, &url).map(|walk| (url, walk.title)))
        .collect()
}

impl JobView {
    fn row_tabs_of(&mut self, id: &SharedString) -> &mut RowTabs {
        self.row_tabs.entry(id.clone()).or_default()
    }

    /// The tab in front of the thread's pane, where it is not the thread.
    pub(super) fn row_front(&self, id: &str) -> Option<&RowTab> {
        let tabs = self.row_tabs.get(id)?;
        tabs.front.and_then(|at| tabs.tabs.get(at))
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
        self.keys_to_front(window, cx);
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
        self.keys_to_front(window, cx);
    }

    /// A subagent's thread as a tab named for its task (`focusSubagent`), from a drone on its group.
    /// A subagent the Job no longer holds opens no tab.
    pub(super) fn open_subagent_tab(&mut self, subagent: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(one) = self.record(&id, cx).and_then(|record| record.attachments.iter().find(|one| one.kind == "subagent" && one.target == subagent)).map(super::ledger::subagent_of) else { return };
        self.watch_subagent(&one, cx);
        self.row_tabs_of(&id).focus(RowKind::Subagent, subagent, one.task);
        self.keys_to_front(window, cx);
    }

    /// A press on a walk he is owed (`openJobWalk`): a native walk starts its window at once, and its
    /// tab comes forward only where Fleet does not start it.
    pub(super) fn press_walk(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !url.starts_with(NATIVE_WALK) {
            return;
        }
        let url = url.to_string();
        self.open_walk(&url, false, window, cx);
    }

    fn bring_walk_forward(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(title) = self.record(&id, cx).and_then(|record| shown_walk(record, url)).map(|walk| walk.title) else { return };
        let tabs = self.row_tabs_of(&id);
        tabs.arrived.insert(url.to_string());
        tabs.focus(RowKind::Walk, url, title);
        self.keys_to_front(window, cx);
    }

    /// The keys go to what is in front of the thread's pane: the tab's body, else the thread's message box.
    fn keys_to_front(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let front = self.shown.as_ref().and_then(|id| self.row_tabs.get(id)).and_then(|tabs| tabs.front);
        if let Some(at) = front {
            // The thread's own tab stands first in the strip while it is open.
            self.row_strip.scroll_to_item(at + usize::from(!self.tab_closed("thread")));
        }
        if front.is_some() {
            window.focus(&self.row_focus, cx);
        } else {
            self.to_composer(window, cx);
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
    pub(super) fn sync_walk_tabs(&mut self, id: &SharedString, cx: &mut Context<Self>) {
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
        let Some(owed) = self.record(id, cx).map(owed_walks) else { return };
        if self.row_tabs_of(id).sync_walks(&owed) {
            self.reopen_thread(id);
        }
    }

    // ---- drawing -----------------------------------------------------------------------

    /// The strip's tabs after the thread's.
    pub(super) fn row_tab_strip(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(tabs) = self.row_tabs.get(id) else { return Vec::new() };
        tabs.tabs
            .iter()
            .enumerate()
            .map(|(at, tab)| {
                let front = tabs.front == Some(at);
                let (icon, _) = tab.kind.look();
                let group = SharedString::from(format!("row-tab:{at}"));
                let pulses = tab.kind == RowKind::Walk && !tab.seen;
                let ink = look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED });
                let tint = look.color(token::STATUS_AWAITING_REVIEW);
                let close = labelled(
                    super::tabs::frame(look, SharedString::from(format!("row-tab-close:{at}")), "x")
                        .mr(look.length(token::SPACE_1))
                        .when(!front, |close| close.opacity(0.).group_hover(group.clone(), |style| style.opacity(1.)))
                        .tooltip(super::head::tip("Close".into()))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            if let Some(id) = this.shown.clone() {
                                this.row_tabs_of(&id).close(at);
                            }
                            this.keys_to_front(window, cx);
                        })),
                    Role::Button,
                    format!("Close {}", tab.title),
                );
                labelled(
                    div()
                        .id(SharedString::from(format!("row-tab:{}:{}", tab.kind.look().1, tab.key)))
                        .group(group.clone())
                        .relative()
                        .flex_none()
                        .max_w(look.length(token::SPACE_12) * 5.)
                        .h_full()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .pl(look.length(token::SPACE_3))
                        .border_r(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_GLASS))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                        .text_color(ink)
                        .when(front, |one| one.bg(look.color(token::BG_RAISED)))
                        .when(!front, |one| one.cursor_pointer().hover(|style| style.text_color(look.color(token::FG_DEFAULT))))
                        .when(pulses, |one| {
                            one.child(div().absolute().inset_0().with_animation(
                                SharedString::from(format!("row-tab-owes:{}", tab.key)),
                                gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.06, 0.14)),
                                move |tinted, at| tinted.bg(tint.opacity(at)),
                            ))
                        })
                        .child(glyph(look, icon, Size::Small, ink))
                        .child(div().min_w_0().truncate().child(SharedString::from(tab.title.clone())))
                        .child(close)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(id) = this.shown.clone() {
                                this.row_tabs_of(&id).show(Some(at));
                            }
                            this.keys_to_front(window, cx);
                        })),
                    Role::Tab,
                    format!("Job tab: {}", tab.title),
                )
                .into_any_element()
            })
            .collect()
    }

    /// The thread comes to the front of its pane.
    pub(super) fn front_thread(&mut self) {
        if let Some(id) = self.shown.clone() {
            self.row_tabs_of(&id).show(None);
        }
    }

    /// The body of the tab in front, where it is not the thread. **What has gone draws nothing**: a
    /// draft's row or a document's card no longer in the thread, a walk the Job never showed, or a
    /// subagent not yet read.
    pub(super) fn row_tab_body(&self, id: &SharedString, tab: &RowTab, look: &Look, window: &mut Window, cx: &Context<Self>) -> AnyElement {
        let Some(page) = self.pages.get(id) else { return div().id("row-tab-body").track_focus(&self.row_focus).size_full().into_any_element() };
        let now = chrono::Local::now();
        let card = match tab.kind {
            RowKind::Draft => self.thread(id, cx).and_then(|held| super::review::draft_row(&held.items, &tab.key)).and_then(|row| match row {
                SessionRow::Draft { id: row, at, title, text, before, settled, change } => {
                    Some(self.draft(SHEET, row, at, title, text, before.as_deref(), settled.as_ref(), *change, false, page, look, now, cx))
                }
                _ => None,
            }),
            RowKind::Document => self.thread(id, cx).and_then(|held| super::review::document_card(&held.items, &tab.key)).map(|card| self.document(SHEET, card, page, look, now, cx)),
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
    fn a_page_with_no_name_is_named_by_the_end_of_its_path() {
        assert_eq!(name_of_page("armada://walk/mergeLine"), "mergeLine");
        assert_eq!(name_of_page("armada://walk/tabs/"), "tabs");
    }
}

impl JobView {
    /// Whether no tab stands beside a Job's thread.
    pub(super) fn row_tabs_empty(&self, id: &str) -> bool {
        self.row_tabs.get(id).is_none_or(|tabs| tabs.tabs.is_empty())
    }
}
