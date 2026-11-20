//! A Job's API Client tab: one request line (method, address, environment, Send), the saved requests and this Job's sends beside
//! the request being edited, and the answer under it. Fleet serves every route (`GET /requests`, `POST /requests/send`), and a
//! send made here lands in the Job's thread as a row, as Classic's does.
//!
//! The tab is an entity the Job view holds for each tab of the kind (`job_view/api_client_tab.rs`), so what was typed and the
//! answer stay while another tab is in front. The address and the body are also kept as drafts, so closing the tab does not lose
//! them. `model.rs` works out what is shown, `wire.rs` makes the calls, and `view.rs` draws. Saving a request, a collection's
//! shared headers, the environments with their secrets and the runs under a saved request are `save.rs`, `environments.rs`
//! and `runs.rs`, each both the state and the drawing of its part.

mod environments;
mod mock;
mod model;
mod runs;
mod save;
mod view;
mod wire;

use std::rc::Rc;

use gpui::{prelude::*, Anchor, Context, DismissEvent, Entity, FocusHandle, Focusable, ListAlignment, ListState, Role, SharedString, Subscription, Task, WeakEntity, Window};
use gpui_component::input::{InlineToken, InputContent, InputEvent, InputState, TextareaState};
use ipc::{Placed, RequestCollection, RequestEnvironmentRead, RequestHeader, RequestSent, Requests};
use environments::EnvForm;
use runs::Runs;
use save::{CollectionEdit, SaveRow};

use crate::dropdown::{Dropdown, Entry, Picked};
use crate::fleet::FleetConnection;
use crate::job_view::JobView;
use crate::keymap::Act;
use model::{Filter, Line};
use wire::{Answered, ToSend};

pub use model::{SentLine, Start};

/// The tab's kind in a Job's panes, Classic's name for it.
pub const KIND: &str = "request";

/// What the tab is called on its strip and in a menu.
pub const NAME: &str = "API Client";

/// A request of the Job's thread with the answer it was kept with, to open again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    pub id: String,
    pub start: Start,
    pub sent: RequestSent,
}

/// What Fleet's read has come to.
pub(crate) enum Load {
    Reading,
    Failed(String),
    Read(Rc<Requests>),
}

/// What is open in the editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Open {
    Blank,
    /// A saved request, by its collection's id and its own.
    Saved(String, String),
    /// A request of this Job's thread, by its row's id.
    Sent(String),
    /// A run of a saved request, by its id: the request as it went and the answer it had.
    Run(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Headers,
    Body,
    Environment,
}

impl Part {
    pub const ALL: [Part; 3] = [Part::Headers, Part::Body, Part::Environment];

    pub fn word(self) -> &'static str {
        match self {
            Part::Headers => "Headers",
            Part::Body => "Body",
            Part::Environment => "Environment",
        }
    }
}

/// Which rows of headers a call is about: the request's own, or the shared ones of the collection being edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rows {
    Request,
    Collection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Show {
    Body,
    Headers,
}

/// What a send came to: Fleet's answer, or Fleet's own words for refusing it.
pub(crate) enum Reply {
    Answered(Answered),
    Refused(String),
}

/// One header of the request: a name and a value, each a field.
pub(crate) struct HeaderRow {
    pub name: Entity<InputState>,
    pub value: Entity<InputState>,
    _subs: Vec<Subscription>,
}

/// A header row's fields as typed, a row with no name left out.
pub(crate) fn headers_of(rows: &[HeaderRow], cx: &gpui::App) -> Vec<RequestHeader> {
    rows.iter()
        .map(|row| RequestHeader { name: row.name.read(cx).value().trim().to_string(), value: row.value.read(cx).value().trim().to_string() })
        .filter(|one| !one.name.is_empty())
        .collect()
}

/// A request waiting on the read, which gives it its `{{base}}` back.
struct Waiting {
    open: Open,
    start: Start,
    answer: Option<RequestSent>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pick {
    Method,
    Environment,
    Collection,
}

struct Menu {
    menu: Entity<Dropdown>,
    _subs: Vec<Subscription>,
}

const METHOD_IDS: [&str; 7] = ["method:GET", "method:POST", "method:PUT", "method:PATCH", "method:DELETE", "method:HEAD", "method:OPTIONS"];
const ENVIRONMENT_NONE: &str = "environment:none";
const ENVIRONMENT_IDS: [&str; 24] = [
    "environment:0", "environment:1", "environment:2", "environment:3", "environment:4", "environment:5", "environment:6", "environment:7", "environment:8", "environment:9", "environment:10", "environment:11",
    "environment:12", "environment:13", "environment:14", "environment:15", "environment:16", "environment:17", "environment:18", "environment:19", "environment:20", "environment:21", "environment:22", "environment:23",
];

const COLLECTION_IDS: [&str; 24] = [
    "collection:0", "collection:1", "collection:2", "collection:3", "collection:4", "collection:5", "collection:6", "collection:7", "collection:8", "collection:9", "collection:10", "collection:11",
    "collection:12", "collection:13", "collection:14", "collection:15", "collection:16", "collection:17", "collection:18", "collection:19", "collection:20", "collection:21", "collection:22", "collection:23",
];

/// The saved requests an address finds, at most.
const FOUND: usize = 6;

pub struct ApiClient {
    job: SharedString,
    view: WeakEntity<JobView>,
    /// None answers from the mock: a walk's or a mock Job's.
    fleet: Option<Entity<FleetConnection>>,
    pub(crate) focus: FocusHandle,
    pub(crate) list_focus: FocusHandle,
    pub(crate) method_focus: FocusHandle,
    pub(crate) environment_focus: FocusHandle,
    pub(crate) send_focus: FocusHandle,
    pub(crate) add_focus: FocusHandle,
    pub(crate) save_focus: FocusHandle,
    pub(crate) chip_focus: [FocusHandle; 4],
    pub(crate) part_focus: [FocusHandle; 3],
    pub(crate) show_focus: [FocusHandle; 2],
    pub(crate) read: Load,
    pub(crate) sent: Vec<SentLine>,
    sent_version: Option<u64>,
    pub(crate) open: Open,
    pub(crate) method: String,
    pub(crate) environment: Option<String>,
    pub(crate) address: Entity<InputState>,
    pub(crate) body: Entity<TextareaState>,
    pub(crate) headers: Vec<HeaderRow>,
    /// What a mock Job keeps between calls, as Fleet would; none where Fleet answers.
    store: Option<mock::Store>,
    pub(crate) part: Part,
    /// The runs under the saved request that is open, and the one the open run came from.
    pub(crate) runs: Option<Runs>,
    /// The inline row under the request line that asks a new request for a name and a collection.
    pub(crate) saving: Option<SaveRow>,
    pub(crate) save_task: Option<Task<()>>,
    /// Fleet's own words for refusing the last save of the request.
    pub(crate) save_said: Option<String>,
    /// The saved request to select once the read after its save lands.
    select_after: Option<(String, String)>,
    /// The collection whose shared headers are open on the right, in place of the request.
    pub(crate) collection_edit: Option<CollectionEdit>,
    /// The environments of the repository, a form for each of the person's own, and the one being added.
    pub(crate) forms: Vec<EnvForm>,
    pub(crate) adding: Option<EnvForm>,
    pub(crate) env_add_focus: FocusHandle,
    pub(crate) env_next: u64,
    pub(crate) env_active: Option<u64>,
    opening: Option<Task<()>>,
    pub(crate) filter: Filter,
    pub(crate) lines: Vec<Line>,
    pub(crate) list: ListState,
    /// The row the keys are on in the list.
    pub(crate) cursor: Option<usize>,
    /// The saved requests the address finds, and the one the arrows are on.
    pub(crate) found: Vec<(usize, usize)>,
    pub(crate) picked: Option<usize>,
    pub(crate) sending: Option<Task<()>>,
    pub(crate) reply: Option<Reply>,
    pub(crate) show: Show,
    pub(crate) reply_list: ListState,
    menu: Option<Menu>,
    waiting: Option<Waiting>,
    reading: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

/// An address as the field holds it: its text, with each `{{name}}` one token.
fn content_of(text: &str) -> InputContent {
    let mut content = InputContent::new(text.to_string());
    for (range, name) in model::names(text) {
        let token = InlineToken::new(name, text[range.clone()].to_string());
        if let Ok(next) = content.clone().with_token(range, token) {
            content = next;
        }
    }
    content
}

fn draft_key(job: &str, field: &str) -> String {
    format!("request:{job}:{field}")
}

impl ApiClient {
    /// The Job view is being updated as the tab is made, so what it holds comes in with the call.
    pub fn new(job: SharedString, view: Entity<JobView>, fleet: Option<Entity<FleetConnection>>, sent: (Option<u64>, Vec<SentLine>), window: &mut Window, cx: &mut Context<Self>) -> Self {
        let kept_address = crate::drafts::kept(&draft_key(&job, "address"), cx).unwrap_or_default();
        let kept_body = crate::drafts::kept(&draft_key(&job, "body"), cx).unwrap_or_default();
        let address = cx.new(|cx| InputState::new(window, cx).placeholder("{{base}}/path"));
        address.update(cx, |input, cx| input.set_value(content_of(&kept_address), window, cx));
        let body = cx.new(|cx| TextareaState::new(window, cx).auto_grow(6, 16));
        body.update(cx, |input, cx| input.set_value(kept_body, window, cx));
        let subs = vec![
            cx.subscribe_in(&address, window, |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.address_typed(input, window, cx),
                InputEvent::PressEnter { .. } => this.entered(window, cx),
                InputEvent::Focus | InputEvent::Blur => cx.notify(),
            }),
            cx.subscribe_in(&body, window, |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    crate::drafts::keep(&draft_key(&this.job, "body"), input.read(cx).value().as_ref(), cx);
                    cx.notify();
                }
                InputEvent::PressEnter { secondary: true, .. } => this.send(window, cx),
                InputEvent::Focus | InputEvent::Blur | InputEvent::PressEnter { .. } => cx.notify(),
            }),
            cx.observe(&view, |this, view, cx| this.sync_sent(&view, cx)),
        ];
        let store = fleet.is_none().then(|| mock::Store::new(&job));
        let mut tab = ApiClient {
            job,
            view: view.downgrade(),
            fleet,
            focus: cx.focus_handle(),
            list_focus: cx.focus_handle().tab_stop(true),
            method_focus: cx.focus_handle().tab_stop(true),
            environment_focus: cx.focus_handle().tab_stop(true),
            send_focus: cx.focus_handle().tab_stop(true),
            add_focus: cx.focus_handle().tab_stop(true),
            save_focus: cx.focus_handle().tab_stop(true),
            chip_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            part_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            show_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            read: Load::Reading,
            sent: sent.1,
            sent_version: sent.0,
            open: Open::Blank,
            method: "GET".into(),
            environment: None,
            address,
            body,
            headers: Vec::new(),
            store,
            part: Part::Headers,
            runs: None,
            saving: None,
            save_task: None,
            save_said: None,
            select_after: None,
            collection_edit: None,
            forms: Vec::new(),
            adding: None,
            env_add_focus: cx.focus_handle().tab_stop(true),
            env_next: 0,
            env_active: None,
            opening: None,
            filter: Filter::ThisJob,
            lines: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            cursor: None,
            found: Vec::new(),
            picked: None,
            sending: None,
            reply: None,
            show: Show::Body,
            reply_list: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            menu: None,
            waiting: None,
            reading: None,
            _subs: subs,
        };
        tab.set_headers(&[], window, cx);
        tab.relist(true);
        let weak = cx.weak_entity();
        tab.list.set_scroll_handler(move |event, window, cx| {
            if event.visible_range.end >= event.count {
                let weak = weak.clone();
                window.defer(cx, move |window, cx| {
                    weak.update(cx, |this, cx| this.want_more_runs(window, cx)).ok();
                });
            }
        });
        tab.read_requests(window, cx);
        tab
    }

    // ---- reading ---------------------------------------------------------------------------------

    fn port(&self, cx: &gpui::App) -> Option<u16> {
        self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// Make a call on Fleet's port, or on the mock's store where there is no Fleet, and hand its answer to `then`: at once for the
    /// mock, and on the main thread once Fleet has answered. The task returned is kept by the caller, or the call is cancelled.
    pub(crate) fn ask<A: 'static, T: 'static>(
        &mut self,
        arg: A,
        live: impl FnOnce(u16, &gpui::App, A) -> Task<Result<T, String>>,
        mock: impl FnOnce(&mut mock::Store, A) -> Result<T, String>,
        then: impl FnOnce(&mut Self, Result<T, String>, &mut Window, &mut Context<Self>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Task<()>> {
        let asked = match &self.fleet {
            None => {
                let answer = self.store.as_mut().map(|store| mock(store, arg)).unwrap_or_else(|| Err("Nothing to ask".to_string()));
                then(self, answer, window, cx);
                return None;
            }
            Some(_) => match self.port(cx) {
                Some(port) => live(port, cx, arg),
                None => {
                    then(self, Err("Fleet is not connected".to_string()), window, cx);
                    return None;
                }
            },
        };
        Some(cx.spawn_in(window, async move |this, cx| {
            let answer = asked.await;
            let _ = this.update_in(cx, |this, window, cx| then(this, answer, window, cx));
        }))
    }

    /// Ask Fleet for the collections and environments. What is drawn stays until the answer lands.
    pub fn read_requests(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let answer = match &self.fleet {
            None => {
                let read = self.store.as_ref().map(mock::Store::read).ok_or_else(|| "Nothing to read".to_string());
                return self.landed(read, window, cx);
            }
            Some(_) => match self.port(cx) {
                Some(port) => wire::read(port, &self.job, cx),
                None => return self.landed(Err("Fleet is not connected".into()), window, cx),
            },
        };
        self.reading = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            let _ = this.update_in(cx, |this, window, cx| this.landed(answer, window, cx));
        }));
    }

    fn landed(&mut self, answer: Result<Requests, String>, window: &mut Window, cx: &mut Context<Self>) {
        self.reading = None;
        match answer {
            Ok(read) => self.read = Load::Read(Rc::new(read)),
            // A read that fails after one landed keeps the one drawn.
            Err(_) if matches!(self.read, Load::Read(_)) => {}
            Err(why) => self.read = Load::Failed(why),
        }
        self.sync_forms(window, cx);
        self.settle_collection_edit();
        if let Some((collection, request)) = self.select_after.take() {
            self.select_saved(&collection, &request, window, cx);
        }
        self.relist(false);
        if let Some(waiting) = self.waiting.take() {
            self.open_waiting(waiting, window, cx);
        }
        cx.notify();
    }

    /// The Job's requests in the thread moved: the list says so once the thread's version has.
    fn sync_sent(&mut self, view: &Entity<JobView>, cx: &mut Context<Self>) {
        let job = self.job.to_string();
        let version = view.read(cx).requested_version(&job, cx);
        if version == self.sent_version {
            return;
        }
        let lines = view.read(cx).requested(&job, cx);
        self.sent_version = version;
        if lines != self.sent {
            self.sent = lines;
            self.relist(false);
            cx.notify();
        }
    }

    pub(crate) fn requests(&self) -> Option<&Rc<Requests>> {
        match &self.read {
            Load::Read(read) => Some(read),
            _ => None,
        }
    }

    /// The list under the filter. A filter that moved starts at the top; a read or a send keeps the place.
    pub(crate) fn relist(&mut self, reset: bool) {
        let shown = self.runs.as_ref().map(Runs::shown);
        self.lines = model::lines(self.requests().map(|read| read.as_ref()), &self.sent, self.filter, shown);
        if reset {
            self.list.reset(self.lines.len());
        } else {
            let old = self.list.item_count();
            self.list.splice(0..old, self.lines.len());
        }
        self.cursor = self.cursor.filter(|at| self.lines.get(*at).is_some_and(Line::is_stop));
    }

    /// The saved request at a place in the read.
    pub(crate) fn saved_at(&self, collection: usize, request: usize) -> Option<(&Placed<RequestCollection>, &ipc::SavedRequest)> {
        let collection = self.requests()?.collections.get(collection)?;
        Some((collection, collection.item.requests.get(request)?))
    }

    /// The collection the open request is saved in, whose headers go with it.
    pub(crate) fn collection(&self) -> Option<&Placed<RequestCollection>> {
        let Open::Saved(collection, _) = &self.open else { return None };
        self.requests()?.collections.iter().find(|one| one.item.id == *collection)
    }

    /// The saved request that is open.
    pub(crate) fn saved(&self) -> Option<&ipc::SavedRequest> {
        let Open::Saved(_, request) = &self.open else { return None };
        self.collection()?.item.requests.iter().find(|one| one.id == *request)
    }

    pub(crate) fn environments(&self) -> &[Placed<RequestEnvironmentRead>] {
        self.requests().map(|read| read.environments.as_slice()).unwrap_or_default()
    }

    /// The environment picked, where Fleet still lists it.
    pub(crate) fn environment_read(&self) -> Option<&RequestEnvironmentRead> {
        let name = self.environment.as_deref()?;
        self.environments().iter().map(|one| &one.item).find(|one| one.name == name)
    }

    // ---- what the editor holds -------------------------------------------------------------------

    pub(crate) fn address_text(&self, cx: &gpui::App) -> String {
        self.address.read(cx).value().to_string()
    }

    pub(crate) fn body_text(&self, cx: &gpui::App) -> String {
        self.body.read(cx).value().to_string()
    }

    /// The request's own headers as typed: a row with no name is not sent.
    pub(crate) fn own_headers(&self, cx: &gpui::App) -> Vec<RequestHeader> {
        headers_of(&self.headers, cx)
    }

    /// The first header row whose name holds a space, which no header name can.
    pub(crate) fn bad_header(&self, cx: &gpui::App) -> Option<usize> {
        let names: Vec<String> = self.headers.iter().map(|row| row.name.read(cx).value().to_string()).collect();
        model::bad_header(&names)
    }

    pub(crate) fn can_send(&self, cx: &gpui::App) -> bool {
        self.sending.is_none() && !self.address_text(cx).trim().is_empty() && self.bad_header(cx).is_none()
    }

    pub(crate) fn header_row(&self, rows: Rows, header: Option<&RequestHeader>, window: &mut Window, cx: &mut Context<Self>) -> HeaderRow {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let value = cx.new(|cx| InputState::new(window, cx).placeholder("Value"));
        if let Some(header) = header {
            name.update(cx, |input, cx| input.set_value(header.name.clone(), window, cx));
            value.update(cx, |input, cx| input.set_value(header.value.clone(), window, cx));
        }
        // ↵ in the request's own rows sends; in the collection's it saves the collection.
        let typed = move |this: &mut ApiClient, event: &InputEvent, window: &mut Window, cx: &mut Context<ApiClient>| match (event, rows) {
            (InputEvent::PressEnter { .. }, Rows::Request) => this.entered(window, cx),
            (InputEvent::PressEnter { .. }, Rows::Collection) => this.save_collection(window, cx),
            _ => cx.notify(),
        };
        let subs = vec![
            cx.subscribe_in(&name, window, move |this, _, event: &InputEvent, window, cx| typed(this, event, window, cx)),
            cx.subscribe_in(&value, window, move |this, _, event: &InputEvent, window, cx| typed(this, event, window, cx)),
        ];
        HeaderRow { name, value, _subs: subs }
    }

    /// The request's headers as rows, one empty row where it has none.
    fn set_headers(&mut self, headers: &[RequestHeader], window: &mut Window, cx: &mut Context<Self>) {
        self.headers = self.rows_of(Rows::Request, headers, window, cx);
    }

    /// Headers as rows, one empty row where there are none.
    pub(crate) fn rows_of(&self, which: Rows, headers: &[RequestHeader], window: &mut Window, cx: &mut Context<Self>) -> Vec<HeaderRow> {
        let mut rows: Vec<HeaderRow> = headers.iter().map(|header| self.header_row(which, Some(header), window, cx)).collect();
        if rows.is_empty() {
            rows.push(self.header_row(which, None, window, cx));
        }
        rows
    }

    pub(crate) fn rows_mut(&mut self, which: Rows) -> Option<&mut Vec<HeaderRow>> {
        match which {
            Rows::Request => Some(&mut self.headers),
            Rows::Collection => self.collection_edit.as_mut().map(|edit| &mut edit.rows),
        }
    }

    pub(crate) fn add_header(&mut self, which: Rows, window: &mut Window, cx: &mut Context<Self>) {
        let row = self.header_row(which, None, window, cx);
        let name = row.name.clone();
        if let Some(rows) = self.rows_mut(which) {
            rows.push(row);
        }
        name.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    pub(crate) fn remove_header(&mut self, which: Rows, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let blank = self.header_row(which, None, window, cx);
        if let Some(rows) = self.rows_mut(which) {
            if at < rows.len() {
                rows.remove(at);
            }
            if rows.is_empty() {
                rows.push(blank);
            }
        }
        cx.notify();
    }

    /// Put a request in the editor: its method, address, body and headers, and the answer it came with where it has one.
    fn fill(&mut self, open: Open, start: Start, answer: Option<RequestSent>, window: &mut Window, cx: &mut Context<Self>) {
        self.open = open;
        // Another request is in the editor: what was asked of the last one, and its collection's headers, are put away.
        self.saving = None;
        self.save_said = None;
        self.collection_edit = None;
        self.expand_runs(window, cx);
        self.method = start.method.to_ascii_uppercase();
        let text = start.url.clone();
        self.address.update(cx, |input, cx| input.set_value(content_of(&text), window, cx));
        crate::drafts::keep(&draft_key(&self.job, "address"), &start.url, cx);
        let words = start.body.clone().unwrap_or_default();
        self.body.update(cx, |input, cx| input.set_value(words.clone(), window, cx));
        crate::drafts::keep(&draft_key(&self.job, "body"), &words, cx);
        self.set_headers(&start.headers, window, cx);
        if let Some(name) = start.environment {
            self.environment = Some(name).filter(|name| self.environments().iter().any(|one| one.item.name == *name));
        }
        self.found.clear();
        self.picked = None;
        self.reply = answer.map(|sent| Reply::Answered(Answered::of(sent)));
        self.show = Show::Body;
        self.reset_reply();
        self.relist(false);
        cx.notify();
    }

    fn reset_reply(&mut self) {
        let count = match &self.reply {
            Some(Reply::Answered(answered)) => answered.lines.len(),
            _ => 0,
        };
        self.reply_list.reset(count);
    }

    /// A request to start from, as Classic's `startOf` and a thread row's Open: its address reads `{{base}}` again where it
    /// starts with its environment's base. Held until the read lands where it has not.
    pub fn start(&mut self, recorded: Recorded, window: &mut Window, cx: &mut Context<Self>) {
        let waiting = Waiting { open: Open::Sent(recorded.id), start: recorded.start, answer: Some(recorded.sent) };
        if matches!(self.read, Load::Reading) {
            self.waiting = Some(waiting);
        } else {
            self.open_waiting(waiting, window, cx);
        }
        self.focus_address(window, cx);
    }

    fn open_waiting(&mut self, waiting: Waiting, window: &mut Window, cx: &mut Context<Self>) {
        let start = waiting.start.rebased(self.environments());
        self.fill(waiting.open, start, waiting.answer, window, cx);
    }

    pub fn focus_address(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.address.update(cx, |input, cx| input.focus(window, cx));
    }

    // ---- the address -----------------------------------------------------------------------------

    /// The address changed: each `{{name}}` typed becomes a token, the draft is kept, and the saved requests it names are found.
    fn address_typed(&mut self, input: &Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) {
        let (value, spans) = {
            let state = input.read(cx);
            (state.value().to_string(), state.tokens().iter().map(|span| span.range()).collect::<Vec<_>>())
        };
        let loose: Vec<_> = model::names(&value).into_iter().filter(|(range, _)| !spans.contains(range)).collect();
        if !loose.is_empty() {
            input.update(cx, |input, cx| {
                for (range, name) in loose {
                    let text = value[range.clone()].to_string();
                    let _ = input.replace_range_with_token(range, InlineToken::new(name, text), window, cx);
                }
            });
        }
        crate::drafts::keep(&draft_key(&self.job, "address"), &value, cx);
        self.found = self.requests().map(|read| model::found(read, &value, FOUND)).unwrap_or_default();
        self.picked = None;
        cx.notify();
    }

    /// Whether the saved requests the address found are drawn: while the address has the keys.
    pub(crate) fn finding(&self, window: &Window, cx: &gpui::App) -> bool {
        !self.found.is_empty() && self.address.read(cx).focus_handle(cx).is_focused(window)
    }

    /// ↵ in a field of the request line: the saved request the arrows are on, else Send.
    fn entered(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.picked.and_then(|at| self.found.get(at).copied()) {
            Some((collection, request)) if self.finding(window, cx) => self.open_saved(collection, request, window, cx),
            _ => self.send(window, cx),
        }
    }

    fn move_found(&mut self, by: isize, cx: &mut Context<Self>) {
        let last = self.found.len().saturating_sub(1);
        self.picked = Some(match (self.picked, by) {
            (None, by) if by > 0 => 0,
            (None, _) => last,
            (Some(at), by) if by > 0 => (at + 1).min(last),
            (Some(at), _) => at.saturating_sub(1),
        });
        crate::ui_sounds::moved();
        cx.notify();
    }

    // ---- the list --------------------------------------------------------------------------------

    pub(crate) fn set_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
        if self.filter != filter {
            self.filter = filter;
            self.cursor = None;
            self.relist(true);
            cx.notify();
        }
    }

    pub(crate) fn open_saved(&mut self, collection: usize, request: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((placed, saved)) = self.saved_at(collection, request) else { return };
        let open = Open::Saved(placed.item.id.clone(), saved.id.clone());
        let start = Start { method: saved.method.clone(), url: saved.url.clone(), environment: self.environment.clone(), headers: saved.headers.clone(), body: saved.body.clone() };
        self.fill(open, start, None, window, cx);
        self.focus_address(window, cx);
    }

    pub(crate) fn open_sent(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.sent.get(at).map(|line| line.id.clone()) else { return };
        let job = self.job.to_string();
        let Some(recorded) = self.view.upgrade().and_then(|view| view.read(cx).requested_row(&job, &id, cx)) else { return };
        let start = recorded.start.rebased(self.environments());
        self.fill(Open::Sent(id), start, Some(recorded.sent), window, cx);
        self.focus_address(window, cx);
    }

    /// A row of the list, opened.
    pub(crate) fn open_line(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        match self.lines.get(at).cloned() {
            Some(Line::Collection(collection)) => self.open_collection(collection, window, cx),
            Some(Line::Saved(collection, request)) => self.open_saved(collection, request, window, cx),
            Some(Line::Run(run)) => self.open_run(run, window, cx),
            Some(Line::Sent(sent)) => self.open_sent(sent, window, cx),
            _ => {}
        }
    }

    /// ↑ and ↓ in the list: the next row that is not a heading, held at either end.
    fn move_cursor(&mut self, by: isize, cx: &mut Context<Self>) {
        let rows: Vec<usize> = self.lines.iter().enumerate().filter(|(_, line)| line.is_stop()).map(|(at, _)| at).collect();
        let Some(&first) = rows.first() else { return };
        let here = self.cursor.and_then(|at| rows.iter().position(|row| *row == at));
        let next = match (here, by) {
            (None, _) => first,
            (Some(at), by) if by > 0 => rows[(at + 1).min(rows.len() - 1)],
            (Some(at), _) => rows[at.saturating_sub(1)],
        };
        if self.cursor != Some(next) {
            crate::ui_sounds::moved();
            self.cursor = Some(next);
            self.list.scroll_to_reveal_item(next);
            cx.notify();
        }
    }

    /// Whether a row of the list is the request open in the editor.
    pub(crate) fn is_open(&self, line: &Line) -> bool {
        // A collection open on the right has taken the editor's place, so no request row is the open one.
        if self.collection_edit.is_some() && !matches!(line, Line::Collection(_)) {
            return false;
        }
        match (line, &self.open) {
            (Line::Saved(collection, request), Open::Saved(open_collection, open_request)) => self
                .saved_at(*collection, *request)
                .is_some_and(|(placed, saved)| placed.item.id == *open_collection && saved.id == *open_request),
            (Line::Sent(at), Open::Sent(id)) => self.sent.get(*at).is_some_and(|line| line.id == *id),
            (Line::Run(at), Open::Run(id)) => self.runs.as_ref().and_then(|runs| runs.runs.get(*at)).is_some_and(|run| run.id == *id),
            (Line::Collection(at), _) => {
                let id = self.requests().and_then(|read| read.collections.get(*at)).map(|one| one.item.id.as_str());
                id.is_some() && self.collection_edit.as_ref().map(|edit| edit.id.as_str()) == id
            }
            _ => false,
        }
    }

    // ---- the parts -------------------------------------------------------------------------------

    pub(crate) fn set_part(&mut self, part: Part, cx: &mut Context<Self>) {
        if self.part != part {
            self.part = part;
            cx.notify();
        }
    }

    /// `[` and `]`: the part before or after this one of Headers, Body and Environment, round the end.
    fn step_part(&mut self, by: isize, cx: &mut Context<Self>) {
        let at = Part::ALL.iter().position(|one| *one == self.part).unwrap_or(0) as isize;
        let next = (at + by).rem_euclid(Part::ALL.len() as isize) as usize;
        self.set_part(Part::ALL[next], cx);
    }

    pub(crate) fn set_show(&mut self, show: Show, cx: &mut Context<Self>) {
        if self.show != show {
            self.show = show;
            cx.notify();
        }
    }

    /// ⌘E: none, then each environment in the order Fleet lists them, then none again.
    pub(crate) fn cycle_environment(&mut self, cx: &mut Context<Self>) {
        let names: Vec<String> = self.environments().iter().map(|one| one.item.name.clone()).collect();
        let at = self.environment.as_ref().and_then(|name| names.iter().position(|one| one == name));
        self.environment = match at {
            None => names.first().cloned(),
            Some(at) => names.get(at + 1).cloned(),
        };
        cx.notify();
    }

    // ---- sending ---------------------------------------------------------------------------------

    pub(crate) fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_send(cx) {
            return;
        }
        let body = self.body_text(cx);
        // A saved request sent as it was saved, in method and address, is one of its runs: Fleet keeps the run of a `Saved` send
        // and none of an `Own` one. What was edited in its headers and body still goes, over what is kept.
        let address = self.address_text(cx).trim().to_string();
        let saved = self
            .collection()
            .zip(self.saved())
            .filter(|(_, saved)| saved.method.eq_ignore_ascii_case(&self.method) && saved.url == address)
            .map(|(collection, saved)| (collection.item.id.clone(), saved.id.clone()));
        let to_send = ToSend {
            saved,
            method: self.method.clone(),
            url: self.address_text(cx).trim().to_string(),
            environment: self.environment_read().map(|one| one.name.clone()),
            collection: self.collection().map(|one| one.item.id.clone()),
            headers: self.own_headers(cx),
            body: Some(body).filter(|body| !body.is_empty()),
        };
        let asked = match &self.fleet {
            None => {
                let answered = mock::send(&to_send);
                if let Some(store) = self.store.as_mut() {
                    store.kept_run(&to_send, &answered);
                }
                return self.sent_landed(Ok(answered), window, cx);
            }
            Some(_) => match self.port(cx) {
                Some(port) => wire::send(port, &self.job, to_send, cx),
                None => return self.sent_landed(Err("Fleet is not connected".into()), window, cx),
            },
        };
        self.sending = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = asked.await;
            let _ = this.update_in(cx, |this, window, cx| this.sent_landed(answer, window, cx));
        }));
        cx.notify();
    }

    fn sent_landed(&mut self, answer: Result<Answered, String>, window: &mut Window, cx: &mut Context<Self>) {
        self.sending = None;
        self.reply = Some(match answer {
            Ok(answered) => Reply::Answered(answered),
            Err(said) => Reply::Refused(said),
        });
        self.show = Show::Body;
        self.reset_reply();
        // A server the send needed may have started since the last read, and a saved request's send may be a run now.
        self.read_requests(window, cx);
        self.expand_runs(window, cx);
        cx.notify();
    }

    // ---- menus -----------------------------------------------------------------------------------

    /// The method's or the environment's menu, under its chip.
    pub(crate) fn pick(&mut self, which: Pick, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (name, entries, chip) = match which {
            Pick::Method => (
                "Method",
                model::METHODS.iter().zip(METHOD_IDS).map(|(method, id)| Entry::Choice { id, label: (*method).into(), on: *method == self.method }).collect::<Vec<_>>(),
                "Method",
            ),
            Pick::Environment => {
                let mut entries = vec![Entry::Choice { id: ENVIRONMENT_NONE, label: "No environment".into(), on: self.environment_read().is_none() }];
                entries.extend(self.environments().iter().zip(ENVIRONMENT_IDS).map(|(one, id)| Entry::Choice {
                    id,
                    label: one.item.name.clone().into(),
                    on: self.environment.as_deref() == Some(one.item.name.as_str()),
                }));
                ("Environment", entries, "Environment")
            }
            Pick::Collection => {
                let chosen = self.saving.as_ref().and_then(|row| row.collection.clone());
                let entries = self
                    .requests()
                    .map(|read| read.collections.as_slice())
                    .unwrap_or_default()
                    .iter()
                    .zip(COLLECTION_IDS)
                    .map(|(one, id)| Entry::Choice { id, label: one.item.name.clone().into(), on: chosen.as_deref() == Some(one.item.id.as_str()) })
                    .collect::<Vec<_>>();
                ("Collection", entries, "Collection")
            }
        };
        let at = cx
            .try_global::<crate::labels::LabelBook>()
            .and_then(|book| book.find(window.window_handle().window_id(), Role::Button, chip))
            .map(|drawn| gpui::point(drawn.bounds.origin.x, drawn.bounds.origin.y + drawn.bounds.size.height))
            .unwrap_or_else(|| window.mouse_position());
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new(name, entries, at, Anchor::TopLeft, from_keys, cx));
        let restore = back.clone();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.menu = None;
            this.menu_picked(which, id, cx);
            if let Some(back) = restore.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            if let Some(back) = back.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(Menu { menu, _subs: vec![picked, dismissed] });
        cx.notify();
    }

    fn menu_picked(&mut self, which: Pick, id: &str, cx: &mut Context<Self>) {
        match which {
            Pick::Method => {
                if let Some(at) = METHOD_IDS.iter().position(|one| *one == id) {
                    self.method = model::METHODS[at].to_string();
                }
            }
            Pick::Environment => {
                self.environment = ENVIRONMENT_IDS.iter().position(|one| *one == id).and_then(|at| self.environments().get(at)).map(|one| one.item.name.clone());
            }
            Pick::Collection => {
                let picked = COLLECTION_IDS.iter().position(|one| *one == id).and_then(|at| self.requests().and_then(|read| read.collections.get(at))).map(|one| one.item.id.clone());
                if let Some(row) = self.saving.as_mut() {
                    row.collection = picked;
                }
            }
        }
        cx.notify();
    }

    // ---- keys ------------------------------------------------------------------------------------

    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            // The Send chord is the message box's own act; the request line answers it while it has the keys.
            "send_message" => self.send(window, cx),
            "api_client_environment" => self.cycle_environment(cx),
            "api_client_part" => self.step_part(if act.slot == 0 { -1 } else { 1 }, cx),
            "api_client_save" => self.save_pressed(window, cx),
            _ => cx.propagate(),
        }
    }

    /// Esc: the saved requests an address found go away first, then the keys go back to the thread. A menu keeps its own Esc.
    pub(crate) fn escape(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key != "escape" || self.menu.is_some() {
            return;
        }
        cx.stop_propagation();
        if !self.found.is_empty() {
            self.found.clear();
            self.picked = None;
            cx.notify();
            return;
        }
        if self.back_out(window, cx) {
            return;
        }
        if let Some(view) = self.view.upgrade() {
            view.update(cx, |view, cx| view.onto_thread(window, cx));
        }
    }

    /// ↑ ↓ ↵ while the address has the keys and has found saved requests.
    pub(crate) fn finding_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.finding(window, cx) || event.keystroke.modifiers.modified() {
            return;
        }
        match event.keystroke.key.as_str() {
            "down" => self.move_found(1, cx),
            "up" => self.move_found(-1, cx),
            "enter" if self.picked.is_some() => self.entered(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// ↑ ↓ ↵ in the list.
    pub(crate) fn list_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        match event.keystroke.key.as_str() {
            "down" => self.move_cursor(1, cx),
            "up" => self.move_cursor(-1, cx),
            "enter" | "space" => {
                if let Some(at) = self.cursor {
                    self.open_line(at, window, cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    pub(crate) fn menu_element(&self) -> Option<gpui::AnyElement> {
        self.menu.as_ref().map(|open| open.menu.clone().into_any_element())
    }

}
