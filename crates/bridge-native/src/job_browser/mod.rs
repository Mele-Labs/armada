//! The Job browser: a page in each Job's Browser tab that the owner uses at any time, and that the Job's agent drives through
//! Fleet's seven browser acts (`docs/practices/protocol/the-job-browser.md`). Classic's `job-browsers.ts`,
//! `job-browser-acts.ts` and `JobBrowserPane.tsx`, ported.
//!
//! **This module is a child of the Job view** (`job_view/mod.rs` names the file), so it reads the Job view's panes and sends its
//! messages as the Job view's own code does, and the only edits to the Job view's files are the places that call it.
//!
//! | File | What it holds |
//! |---|---|
//! | `acts.rs` | Each act and its answer, against a `Page` a test fakes |
//! | `scripts.rs` | What runs inside the page, in two content worlds of Bridge's own |
//! | `live.rs` | The real page, Fleet's answer and the act's mark |
//! | `tab.rs` | The Browser tab: the head over the page |
//! | `reminder.rs` | The thread's line for a browser the agent reached for outside Bridge |

mod acts;
mod asks;
mod choices;
mod live;
mod reminder;
mod rules;
mod scripts;
mod tab;

use std::borrow::Cow;
use std::collections::HashMap;

use gpui::{AnyElement, App, AppContext, Context, Entity, SharedString, Subscription, Task, Window};

use self::tab::{BrowserTab, BrowserTabEvent, LastShown};
pub use self::tab::bind;
use super::pane_tree::{self as tree, Extras, Tab};
use super::JobView;

/// The tab of a page an agent opened before the Job had a Browser tab to hold it, which the first Browser tab takes over
/// (Classic's `UNATTACHED`).
pub const UNATTACHED: &str = "unattached";

/// Each Job's pages, one for each Browser tab it has, and the acts running on them.
#[derive(Default)]
pub struct Browsers {
    by_job: HashMap<String, Pages>,
    last: LastShown,
    running: HashMap<String, Vec<Mark>>,
    next: u64,
    /// Reminder rows whose press has gone to the Job, until the thread says so itself.
    told: std::collections::HashSet<String>,
    /// What Fleet is told of the pages' asks, one at a time and in order, so an end never reaches it before its raise.
    to_fleet: Option<(async_channel::Sender<FleetCall>, Task<()>)>,
}

#[derive(Default)]
struct Pages {
    tabs: HashMap<String, Entity<BrowserTab>>,
    unattached: Option<Entity<BrowserTab>>,
    /// Hearing each page's asks, which go on to Fleet.
    heard: Vec<Subscription>,
}

/// One thing Fleet is told of a page's ask: the route, and its body.
struct FleetCall {
    port: u16,
    path: &'static str,
    body: String,
}

/// An act running on a Job's page: its sentence, and the page it is on, by entity (`None` until the page opens, for an `open` on a
/// Job with no page).
struct Mark {
    id: u64,
    sentence: SharedString,
    on: Option<u64>,
}

/// The sentence of the latest act running on `page`, so overlapping acts show the newest (`markOf`).
fn acting_on(marks: &[Mark], page: u64) -> Option<SharedString> {
    marks.iter().rev().find(|mark| mark.on == Some(page)).map(|mark| mark.sentence.clone())
}

/// Hears Fleet's news of the Jobs' browsers: an act asked for is done and answered.
pub fn follow(fleet: &Entity<crate::fleet::FleetConnection>, window: &mut Window, cx: &mut Context<JobView>) -> Subscription {
    cx.subscribe_in(fleet, window, |this, _, news: &crate::fleet::BrowserNews, window, cx| this.browser_news(news, window, cx))
}

impl JobView {
    /// An agent's acts, each done on its own and answered once, whatever becomes of it.
    fn browser_news(&mut self, news: &crate::fleet::BrowserNews, window: &mut Window, cx: &mut Context<Self>) {
        let port = self.port(cx);
        for request in news.requests.iter().cloned() {
            cx.spawn_in(window, async move |this, cx| {
                let host = live::LiveHost { view: this, cx: cx.clone(), port };
                acts::act_on(&host, request).await;
            })
            .detach();
        }
        // An ask answered in Needs you settles the tab's strip, with Fleet told nothing since it said so.
        for answered in &news.answered {
            let Some(tab) = self.browsers.by_job.get(answered.session_id.as_str()).and_then(|held| held.tabs.get(&answered.ask_id)).cloned() else { continue };
            let answer = asks::answer_of(answered.choice);
            tab.update(cx, |tab, cx| tab.answered(answer, true, window, cx));
        }
    }

    /// Fleet is told of a page's ask, after what it was told before.
    fn tell_fleet(&mut self, path: &'static str, body: Result<String, ipc::Unencodable>, cx: &mut Context<Self>) {
        let (Some(port), Ok(body)) = (self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()), body) else { return };
        let (queue, _) = self.browsers.to_fleet.get_or_insert_with(|| {
            let (queue, calls) = async_channel::unbounded::<FleetCall>();
            let worker = cx.background_executor().clone();
            let task = cx.spawn(async move |_, _| {
                while let Ok(call) = calls.recv().await {
                    let done = worker
                        .spawn(async move {
                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                            runtime.block_on(crate::transport::post(call.port, call.path, call.body))
                        })
                        .await;
                    // A Fleet that refuses or is gone has nothing more to be told: the tab's strip works without it.
                    if let Err(why) = done {
                        crate::log::Note::warn("a page's ask did not reach Fleet").field("why", why).emit();
                    }
                }
            });
            (queue, task)
        });
        let _ = queue.try_send(FleetCall { port, path, body });
    }

    /// What a tab tells Fleet of its page's ask.
    fn browser_tab_told(&mut self, job: &str, event: &BrowserTabEvent, cx: &mut Context<Self>) {
        match event {
            BrowserTabEvent::Raised { tab, text, options } => {
                let body = ipc::RaiseBrowserAsk { session_id: ipc::SessionId::carried(job.to_string()), ask_id: tab.clone(), text: text.clone(), options: options.clone() };
                self.tell_fleet("/sessions/browser/ask", ipc::encode(&body), cx);
            }
            BrowserTabEvent::Ended { tab } => self.browser_ask_ended(job, tab, cx),
        }
    }

    fn browser_ask_ended(&mut self, job: &str, tab: &str, cx: &mut Context<Self>) {
        let body = ipc::EndBrowserAsk { session_id: ipc::SessionId::carried(job.to_string()), ask_id: tab.to_string() };
        self.tell_fleet("/sessions/browser/ask/end", ipc::encode(&body), cx);
    }

    /// A tab closed or a Job gone: its page's ask is dismissed, and Fleet told it is over.
    fn browser_closed(&mut self, job: &str, tab: &str, page: &Entity<BrowserTab>, cx: &mut Context<Self>) {
        if page.update(cx, |page, cx| page.close(cx)) {
            self.browser_ask_ended(job, tab, cx);
        }
    }

    /// Job `job`'s Browser tabs in pane order.
    fn browser_ids(&self, job: &str) -> Vec<String> {
        let Some(arrangement) = self.arrangement_of(job) else { return Vec::new() };
        tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == "browser").map(|tab| tab.id.clone()).collect()
    }

    /// Each Browser tab Job `job` has gets a page, once, and a page whose tab is gone is dropped with it; a page an agent opened
    /// before there was a tab goes to the first tab there is. Called as the Job draws and before an act reaches it.
    pub(super) fn sync_browsers(&mut self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if self.arrangement_of(job).is_none() {
            return;
        }
        let ids = self.browser_ids(job);
        let last = self.browsers.last.clone();
        let repository = self.record(job, cx).and_then(|record| record.manifest_id.as_ref().map(|manifest| manifest.as_str().to_string())).unwrap_or_else(|| choices::NO_REPOSITORY.to_string());
        // A tab that closed dismisses the ask its page had, and Fleet is told it is over.
        let gone: Vec<(String, Entity<BrowserTab>)> =
            self.browsers.by_job.get(job.as_ref()).map(|held| held.tabs.iter().filter(|(id, _)| !ids.contains(id)).map(|(id, page)| (id.clone(), page.clone())).collect()).unwrap_or_default();
        for (id, page) in gone {
            if let Some(held) = self.browsers.by_job.get_mut(job.as_ref()) {
                held.tabs.remove(&id);
            }
            self.browser_closed(job, &id, &page, cx);
        }
        let held = self.browsers.by_job.entry(job.to_string()).or_default();
        let taking = match (ids.iter().find(|id| !held.tabs.contains_key(*id)), held.unattached.take()) {
            (Some(first), Some(open)) => {
                held.tabs.insert(first.clone(), open.clone());
                Some((first.clone(), open))
            }
            _ => None,
        };
        if let Some((first, open)) = taking {
            open.update(cx, |page, cx| page.attach(first, cx));
        }
        for id in ids {
            let held = self.browsers.by_job.entry(job.to_string()).or_default();
            if !held.tabs.contains_key(&id) {
                let tab = id.clone();
                let page = cx.new(|cx| BrowserTab::new(job.clone(), tab, last.clone(), window, cx));
                let heard = self.hear_tab(job, &page, window, cx);
                let held = self.browsers.by_job.entry(job.to_string()).or_default();
                held.tabs.insert(id, page);
                held.heard.push(heard);
            }
        }
        if let Some(held) = self.browsers.by_job.get(job.as_ref()) {
            for page in held.tabs.values().chain(held.unattached.iter()) {
                page.update(cx, |page, _| page.set_repository(repository.clone()));
            }
        }
    }

    /// Hears what a tab tells Fleet of its page's ask.
    fn hear_tab(&mut self, job: &SharedString, page: &Entity<BrowserTab>, window: &mut Window, cx: &mut Context<Self>) -> Subscription {
        let job = job.to_string();
        cx.subscribe_in(page, window, move |this, _, event: &BrowserTabEvent, _, cx| this.browser_tab_told(&job, event, cx))
    }

    /// The page Job `job`'s agent's acts reach, as it stands: the first Browser tab shown, else the one shown last, else the first.
    fn browser_held(&self, job: &str, cx: &App) -> Option<Entity<BrowserTab>> {
        let held = self.browsers.by_job.get(job)?;
        let mut pages: Vec<(String, Entity<BrowserTab>)> = self.browser_ids(job).into_iter().filter_map(|id| held.tabs.get(&id).map(|tab| (id, tab.clone()))).collect();
        if let Some(open) = &held.unattached {
            pages.push((UNATTACHED.to_string(), open.clone()));
        }
        let shown: Vec<(String, bool)> = pages.iter().map(|(id, tab)| (id.clone(), tab.read(cx).shown(cx))).collect();
        let last = self.browsers.last.borrow().get(job).cloned();
        let at = acts::jobs_page(&shown, last.as_deref())?;
        pages.into_iter().nth(at).map(|(_, tab)| tab)
    }

    /// The Job's page for an act, made and ready. None: the Job has no Browser tab and no page.
    pub(super) fn browser_page(&mut self, job: &str, window: &mut Window, cx: &mut Context<Self>) -> Option<Entity<BrowserTab>> {
        self.sync_browsers(&SharedString::from(job.to_string()), window, cx);
        self.browser_held(job, cx)
    }

    /// An act starts on the Job's page, and its sentence lights the page's mark.
    pub(super) fn browser_mark(&mut self, job: &str, sentence: SharedString, window: &mut Window, cx: &mut Context<Self>) -> u64 {
        self.sync_browsers(&SharedString::from(job.to_string()), window, cx);
        let on = self.browser_held(job, cx).map(|page| page.entity_id().as_u64());
        self.browsers.next += 1;
        let id = self.browsers.next;
        self.browsers.running.entry(job.to_string()).or_default().push(Mark { id, sentence, on });
        self.browser_marked(job, cx);
        id
    }

    /// The act is over: its mark goes, and the page's goes dim if no other act is on it.
    pub(super) fn browser_unmark(&mut self, job: &str, id: u64, cx: &mut Context<Self>) {
        if let Some(marks) = self.browsers.running.get_mut(job) {
            marks.retain(|mark| mark.id != id);
            if marks.is_empty() {
                self.browsers.running.remove(job);
            }
        }
        self.browser_marked(job, cx);
    }

    /// Each of the Job's pages is told the act running on it.
    fn browser_marked(&mut self, job: &str, cx: &mut Context<Self>) {
        let marks = self.browsers.running.get(job).map(Vec::as_slice).unwrap_or(&[]);
        let Some(held) = self.browsers.by_job.get(job) else { return };
        for page in held.tabs.values().chain(held.unattached.iter()) {
            let sentence = acting_on(marks, page.entity_id().as_u64());
            page.update(cx, |page, cx| page.set_acting(sentence, cx));
        }
    }

    /// **An agent's `open` on a Job with no Browser tab opens a page for it**, which the Job's panes then hold in a tab of its own,
    /// as the owner chose on 28 Oct 2026 over refusing the agent until he opens one. The tab takes the page over.
    pub(super) fn browser_open(&mut self, job: &str, window: &mut Window, cx: &mut Context<Self>) -> Entity<BrowserTab> {
        let name = SharedString::from(job.to_string());
        let last = self.browsers.last.clone();
        let open = cx.new(|cx| BrowserTab::new(name.clone(), UNATTACHED.to_string(), last, window, cx));
        // The act that opened it is on it.
        for mark in self.browsers.running.entry(job.to_string()).or_default() {
            mark.on.get_or_insert(open.entity_id().as_u64());
        }
        let heard = self.hear_tab(&name, &open, window, cx);
        let held = self.browsers.by_job.entry(job.to_string()).or_default();
        held.unattached = Some(open.clone());
        held.heard.push(heard);
        self.rearrange_in(&name, None, true, |arrangement| Cow::Owned(tree::arrive(arrangement, "browser", Extras::default())), cx);
        self.sync_browsers(&name, window, cx);
        self.browser_marked(job, cx);
        cx.notify();
        open
    }

    /// The body of a Browser tab in front: its head over its page. None where the tab has no page yet.
    pub(super) fn browser_body(&self, job: &SharedString, tab: &Tab) -> Option<AnyElement> {
        let page = self.browsers.by_job.get(job.as_ref())?.tabs.get(&tab.id)?;
        Some(gpui::IntoElement::into_any_element(page.clone()))
    }

    /// The Job is gone, and so are its pages.
    pub(super) fn forget_browsers(&mut self, job: &str, cx: &mut Context<Self>) {
        // Each page's ask is dismissed and Fleet told it is over, before the pages go.
        let pages: Vec<(String, Entity<BrowserTab>)> = self
            .browsers
            .by_job
            .get(job)
            .map(|held| held.tabs.iter().map(|(id, page)| (id.clone(), page.clone())).collect())
            .unwrap_or_default();
        for (id, page) in pages {
            self.browser_closed(job, &id, &page, cx);
        }
        self.browsers.by_job.remove(job);
        self.browsers.running.remove(job);
        self.browsers.last.borrow_mut().remove(job);
    }

    /// Back, Forward and Reload, as the registry binds them (`history`'s two slots, `browser_reload`), on the Browser tab in front in
    /// the pane the keys are in. **A page that holds the keys hands ⌘ keys back to Bridge** (`web::keys::is_bridges`), so they arrive
    /// here either way. True where it was taken.
    pub(super) fn browser_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        let here = self.here(window, cx);
        let arrangement = self.arrangement().into_owned();
        let Some(front) = tree::pane_by_id(&arrangement, &here).and_then(super::panes_place::front_of).filter(|tab| tab.kind == "browser") else { return false };
        let Some(page) = self.browsers.by_job.get(job.as_ref()).and_then(|held| held.tabs.get(&front.id)).cloned() else { return false };
        match (act.id, act.slot) {
            ("history", 0) => page.update(cx, |page, cx| page.go_back(cx)),
            ("history", 1) => page.update(cx, |page, cx| page.go_forward(cx)),
            ("browser_reload", _) => page.update(cx, |page, cx| page.reload(cx)),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(id: u64, sentence: &str, on: Option<u64>) -> Mark {
        Mark { id, sentence: sentence.to_string().into(), on }
    }

    #[test]
    fn the_page_shows_the_newest_act_running_on_it_and_none_where_none_is() {
        let marks = [mark(1, "Opening example.com", Some(7)), mark(2, "Reading the page", Some(7)), mark(3, "Clicking Save", Some(9))];
        assert_eq!(acting_on(&marks, 7).as_deref(), Some("Reading the page"));
        assert_eq!(acting_on(&marks, 9).as_deref(), Some("Clicking Save"));
        assert_eq!(acting_on(&marks, 11), None);
        assert_eq!(acting_on(&[mark(4, "Opening example.com", None)], 7), None, "an act on no page yet lights none");
    }
}
