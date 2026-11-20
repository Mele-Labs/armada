//! The API Client among a Job's panes: its entity for each tab of the kind, the Job's own requests it lists from the thread, and the
//! Open on a thread row that starts one. `crate::api_client` works out and draws what the tab shows; this holds its place.
//!
//! Each tab's entity is held app-wide by Job view and tab, as the Tokens tab's is, so what was typed and the answer stay while
//! another tab is in front and the Job view carries no field of them.

use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, App, Context, Entity, EntityId, Global, SharedString, Window};
use ipc::{SessionRow, ThreadItem};

use super::pane_tree::{self as tree, Tab};
use super::JobView;
use crate::api_client::{ApiClient, Recorded, SentLine, Start, KIND};

/// Each Job view's API Client tabs, by Job and then by tab id.
#[derive(Default)]
struct Held {
    tabs: HashMap<(EntityId, String), HashMap<String, Entity<ApiClient>>>,
}

impl Global for Held {}

impl JobView {
    /// How far the Job's thread has moved: the API Client lists its requests again only when this has.
    pub(crate) fn requested_version(&self, job: &str, cx: &App) -> Option<u64> {
        self.thread(job, cx).map(|held| held.version)
    }

    /// The requests of the Job's thread, oldest first, without the answers' bodies.
    pub(crate) fn requested(&self, job: &str, cx: &App) -> Vec<SentLine> {
        let Some(held) = self.thread(job, cx) else { return Vec::new() };
        held.items
            .iter()
            .filter_map(|item| match item {
                ThreadItem::Row { row: SessionRow::Requested { id, by, method, url, status, error, .. } } => {
                    Some(SentLine { id: id.clone(), by: *by, method: method.clone(), url: url.clone(), status: *status, error: error.clone() })
                }
                _ => None,
            })
            .collect()
    }

    /// One request of the Job's thread whole, as Classic's `startOf` takes it, with the answer it was kept with.
    pub(crate) fn requested_row(&self, job: &str, row: &str, cx: &App) -> Option<Recorded> {
        let held = self.thread(job, cx)?;
        held.items.iter().find_map(|item| match item {
            ThreadItem::Row { row: SessionRow::Requested { id, method, url, environment, headers, body, status, ms, error, response, truncated, .. } } if id == row => Some(recorded(
                id,
                Start { method: method.clone(), url: url.clone(), environment: environment.clone(), headers: headers.clone(), body: body.clone() },
                ipc::RequestSent {
                    url: url.clone(),
                    status: *status,
                    ms: *ms,
                    headers: Vec::new(),
                    body: response.clone(),
                    truncated: *truncated,
                    error: error.clone(),
                    note: None,
                    saved: None,
                },
            )),
            _ => None,
        })
    }

    fn api_client_ids(&self, job: &str) -> Vec<String> {
        let Some(arrangement) = self.arrangement_of(job) else { return Vec::new() };
        tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == KIND).map(|tab| tab.id.clone()).collect()
    }

    /// Each API Client tab the Job has gets its entity, once; one whose tab closed goes with it.
    fn sync_api_clients(&self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let me = cx.entity_id();
        let ids = self.api_client_ids(job);
        let mut held = cx.default_global::<Held>().tabs.remove(&(me, job.to_string())).unwrap_or_default();
        held.retain(|id, _| ids.contains(id));
        for id in ids {
            if held.contains_key(&id) {
                continue;
            }
            let sent = (self.requested_version(job, cx), self.requested(job, cx));
            let (view, fleet) = (cx.entity(), self.fleet.clone());
            let tab = cx.new(|cx| ApiClient::new(job.clone(), view, fleet, sent, window, cx));
            held.insert(id, tab);
        }
        if !held.is_empty() {
            cx.default_global::<Held>().tabs.insert((me, job.to_string()), held);
        }
    }

    fn held_api_client(&self, job: &str, tab: &str, cx: &Context<Self>) -> Option<Entity<ApiClient>> {
        cx.try_global::<Held>()?.tabs.get(&(cx.entity_id(), job.to_string()))?.get(tab).cloned()
    }

    /// The body of an API Client tab in front, its entity made as it first draws.
    pub(super) fn api_client_body(&self, job: &SharedString, tab: &Tab, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.sync_api_clients(job, window, cx);
        let held = self.held_api_client(job, &tab.id, cx)?;
        Some(div().id(SharedString::from(format!("pane-body:{}", tab.id))).flex_1().min_h_0().flex().flex_col().child(div().size_full().child(held)).into_any_element())
    }

    /// Open the API Client of the Job shown, with `recorded` in its request line where one is given: the one the Job already has
    /// comes to the front, and none makes one.
    pub fn open_api_client(&mut self, recorded: Option<Recorded>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        match self.api_client_ids(&job).into_iter().next() {
            Some(id) => {
                self.look_in(&job, move |arrangement| tree::activate(arrangement, &id), cx);
            }
            None => self.add_tab(KIND, window, cx),
        }
        self.sync_api_clients(&job, window, cx);
        let front = self.api_client_ids(&job).into_iter().find_map(|id| self.held_api_client(&job, &id, cx));
        if let Some(tab) = front {
            tab.update(cx, |tab, cx| match recorded {
                Some(recorded) => tab.start(recorded, window, cx),
                None => tab.focus_address(window, cx),
            });
        }
        cx.notify();
    }

    /// A pane's + made an API Client tab: the keys go to its address, as the tab's body takes them.
    pub(super) fn focus_api_client(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.sync_api_clients(&job, window, cx);
        let front = self.arrangement().focus.clone();
        let tab = tree::pane_by_id(&self.arrangement(), &front).and_then(super::panes_place::front_of).filter(|tab| tab.kind == KIND).map(|tab| tab.id.clone());
        if let Some(client) = tab.and_then(|id| self.held_api_client(&job, &id, cx)) {
            window.defer(cx, move |window, cx| client.update(cx, |client, cx| client.focus_address(window, cx)));
        }
    }
}

fn recorded(id: &str, start: Start, sent: ipc::RequestSent) -> Recorded {
    Recorded { id: id.to_string(), start, sent }
}
