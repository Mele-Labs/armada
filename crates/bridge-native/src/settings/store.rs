//! Settings › Claims store: the store a project's Jobs send their claims to, drawn as split rows (the approved take
//! "Rows", 14 Nov 2026). The store the project names with its team, whether this Mac has joined it and when it was
//! deployed; the Macs signed in to it; Deploy, which asks for a Cloudflare token in place where Cloudflare refused
//! every sign-in this Mac has; and Join by address. **Every sentence a refusal carries is Fleet's**
//! (`crates/ipc/operations/set_project_store.toml`, `deploy_project_store.toml`).
//!
//! **A token is never written anywhere.** It is held in a masked field in memory, sent in the deploy's body and
//! dropped once the deploy works, so it is kept out of the drafts file the other boxes use.

use std::collections::BTreeMap;

use gpui::{div, prelude::*, svg, Animation, AnimationExt, AnyElement, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla, Keystroke, Role, SharedString, Subscription, Task, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{DeployProjectStore, JoinStore, ProjectStoreState, StoreDeployed};

use super::entries::Entry;
use super::overlay::SettingsOverlay;
use super::rows::{says, split_row, rows_card, Row, Spread};
use super::view::{refusal, ringed, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The act out, and so the press that is drawn busy. One at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    UseNone,
    Deploy,
    Join,
}

impl Act {
    /// The key the act's refusal is kept under.
    fn key(self) -> &'static str {
        match self {
            Act::UseNone => "none",
            Act::Deploy => "deploy",
            Act::Join => "join",
        }
    }
}

/// Cloudflare refused every sign-in this Mac has: its sentence, and the page that makes a token.
struct TokenAsk {
    said: String,
    page: String,
}

/// The page's own state: what Fleet said of the project's store, the act out and what it said, the three fields and
/// the key of each press.
pub struct StorePage {
    /// The project the state below is for. `None` until the page is drawn for one, and again once it is left.
    pub(super) project: Option<String>,
    pub(super) state: Option<Result<ProjectStoreState, String>>,
    ask: Option<TokenAsk>,
    pub(super) out: Option<Act>,
    /// A press's refusal, said under it, by `Act::key`.
    pub(super) refused: BTreeMap<&'static str, String>,
    /// Why this Mac could not join the store a deploy made, as the deploy said it.
    join_said: Option<String>,
    pub(super) token: Entity<InputState>,
    pub(super) address: Entity<InputState>,
    pub(super) team: Entity<InputState>,
    keys: BTreeMap<&'static str, FocusHandle>,
    _read: Option<Task<()>>,
    _act: Option<Task<()>>,
    _fields: Vec<Subscription>,
}

const PRESSES: [&str; 6] = ["pick", "use-none", "deploy", "open-page", "deploy-again", "join"];

impl StorePage {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsOverlay>) -> StorePage {
        let token = cx.new(|cx| InputState::new(window, cx));
        token.update(cx, |state, cx| state.set_masked(true, window, cx));
        let address = cx.new(|cx| InputState::new(window, cx));
        let team = cx.new(|cx| InputState::new(window, cx));
        // Enter in the token field presses Deploy again, and in either of Join's fields presses Join.
        let fields = vec![
            cx.subscribe_in(&token, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.press_store("deploy-again", window, cx);
                }
            }),
            cx.subscribe_in(&address, window, |this, _, event: &InputEvent, window, cx| this.field_event("address", event, window, cx)),
            cx.subscribe_in(&team, window, |this, _, event: &InputEvent, window, cx| this.field_event("team", event, window, cx)),
        ];
        StorePage { project: None, state: None, ask: None, out: None, refused: BTreeMap::new(), join_said: None, token, address, team, keys: BTreeMap::new(), _read: None, _act: None, _fields: fields }
    }

    /// Settings opened again: the store is read again the next time the page is drawn.
    pub fn reopen(&mut self) {
        self.project = None;
    }

    fn state(&self) -> Option<&ProjectStoreState> {
        self.state.as_ref().and_then(|read| read.as_ref().ok()).filter(|state| Some(&state.project) == self.project.as_ref())
    }

}

/// `14 Nov`, from the stamp Fleet wrote the deploy at.
fn deployed_on(stamp: &str) -> Option<String> {
    let at = chrono::DateTime::parse_from_rfc3339(stamp).ok()?;
    Some(at.with_timezone(&chrono::Local).format("%-d %b").to_string())
}

fn draft_key(project: &str, field: &str) -> String {
    format!("store:{project}:{field}")
}

impl SettingsOverlay {
    // ---- reading --------------------------------------------------------------------------------

    /// Read the store the project picked names, once each time the page is drawn for a project. Every project has
    /// none to read. A project's two typed fields come back as he left them.
    pub(super) fn read_store(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, _)) = self.scope.project.clone() else {
            self.store.project = None;
            return;
        };
        if self.store.project.as_deref() == Some(id.as_str()) {
            return;
        }
        let page = &mut self.store;
        if page.state().is_none_or(|state| state.project != id) {
            page.state = None;
        }
        page.project = Some(id.clone());
        page.ask = None;
        page.out = None;
        page._act = None;
        page.refused.clear();
        page.join_said = None;
        let (address, team, token) = (page.address.clone(), page.team.clone(), page.token.clone());
        let kept = |field: &str, cx: &mut Context<Self>| crate::drafts::kept(&draft_key(&id, field), cx).unwrap_or_default();
        let (said_address, said_team) = (kept("address", cx), kept("team", cx));
        address.update(cx, |state, cx| state.set_value(said_address, window, cx));
        team.update(cx, |state, cx| state.set_value(said_team, window, cx));
        token.update(cx, |state, cx| state.set_value("", window, cx));
        let read = self.wire.project_store(&id, cx);
        self.store._read = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = read.await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.store.project.as_deref() == Some(id.as_str()) {
                    this.store.state = Some(answer.map_err(|refused| refused.said));
                    cx.notify();
                }
            });
        }));
    }

    /// A typed field changed: its words are kept for the project, and Join is ready once an address is typed.
    fn field_event(&mut self, field: &'static str, event: &InputEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            InputEvent::Change => {
                let Some(project) = self.store.project.clone() else { return };
                let words = if field == "address" { &self.store.address } else { &self.store.team }.read(cx).value().to_string();
                crate::drafts::keep(&draft_key(&project, field), &words, cx);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => self.press_store("join", window, cx),
            _ => {}
        }
    }

    // ---- keys -----------------------------------------------------------------------------------

    pub(super) fn store_keys(&mut self, cx: &mut Context<Self>) {
        for name in PRESSES {
            self.store.keys.entry(name).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
    }

    /// The page's stops, in the order it draws them.
    pub(super) fn store_stops(&self, cx: &gpui::App) -> Vec<FocusHandle> {
        let page = &self.store;
        let key = |name: &str| page.keys.get(name).cloned();
        let mut stops = Vec::new();
        if self.scope.project.is_none() {
            stops.extend(key("pick"));
            return stops;
        }
        let Some(state) = page.state() else { return stops };
        if state.store.is_some() {
            stops.extend(key("use-none"));
        }
        if page.ask.is_some() {
            stops.extend(key("open-page"));
            stops.push(page.token.read(cx).focus_handle(cx));
            stops.extend(key("deploy-again"));
        } else {
            stops.extend(key("deploy"));
        }
        stops.push(page.address.read(cx).focus_handle(cx));
        stops.push(page.team.read(cx).focus_handle(cx));
        stops.extend(key("join"));
        stops
    }

    /// Enter and Space press the control the keys are on, and Esc shuts the token ask.
    pub(super) fn store_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        match key.key.as_str() {
            "escape" if self.store.ask.is_some() => {
                self.shut_ask(window, cx);
                true
            }
            "enter" | "space" => {
                let Some(name) = self.store.keys.iter().find(|(_, focus)| focus.is_focused(window)).map(|(name, _)| *name) else { return false };
                self.press_store(name, window, cx);
                true
            }
            _ => false,
        }
    }

    /// Esc: the ask goes, and the keys with it, to Deploy.
    fn shut_ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.store.ask = None;
        self.store.refused.remove(Act::Deploy.key());
        if let Some(to) = self.store.keys.get("deploy").cloned() {
            window.defer(cx, move |window, cx| window.focus(&to, cx));
        }
        cx.notify();
    }

    pub(super) fn press_store(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        match name {
            "pick" => self.open_picker(true, window, cx),
            "use-none" => self.use_none(window, cx),
            "deploy" | "deploy-again" => self.deploy_store(window, cx),
            "open-page" => {
                if let Some(page) = self.store.ask.as_ref().map(|ask| ask.page.clone()) {
                    self.wire.open_link(&page, cx);
                }
            }
            "join" => self.join_store(window, cx),
            _ => {}
        }
    }

    // ---- acts -----------------------------------------------------------------------------------

    /// Takes the act as the one out, clearing what it last said; `false` while another is out.
    fn begin_store(&mut self, act: Act) -> Option<String> {
        let project = self.store.project.clone()?;
        if self.store.out.is_some() {
            return None;
        }
        self.store.out = Some(act);
        self.store.refused.remove(act.key());
        self.store.join_said = None;
        Some(project)
    }

    /// The act answered. Nothing is done for a project no longer on the page.
    fn answered_store(&mut self, project: &str, act: Act, answer: Result<(), String>, cx: &mut Context<Self>) -> bool {
        if self.store.project.as_deref() != Some(project) {
            return false;
        }
        self.store.out = None;
        if let Err(said) = answer {
            self.store.refused.insert(act.key(), said);
        }
        cx.notify();
        true
    }

    /// Where the keys wait while the press they were on is no longer drawn.
    fn hand_keys_to(&self, name: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(to) = self.store.keys.get(name).cloned() {
            window.defer(cx, move |window, cx| window.focus(&to, cx));
        }
    }

    /// `POST /projects/project/:project/store` with no store: its Jobs send nothing from now on.
    fn use_none(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.begin_store(Act::UseNone) else { return };
        let held = self.store.keys.get("use-none").is_some_and(|focus| focus.is_focused(window));
        let press = self.wire.set_project_store(&project, None, cx);
        self.store._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let taken = match answer {
                    Ok(state) => {
                        let current = this.store.project.as_deref() == Some(state.project.as_str());
                        if current {
                            this.store.state = Some(Ok(state));
                        }
                        Ok(())
                    }
                    Err(refused) => Err(refused.said),
                };
                let done = taken.is_ok();
                if this.answered_store(&project, Act::UseNone, taken, cx) && done && held {
                    this.hand_keys_to("deploy", window, cx);
                }
            });
        }));
        cx.notify();
    }

    /// `POST /projects/project/:project/store/deploy`. The token goes only where one was typed into the ask.
    fn deploy_store(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = if self.store.ask.is_some() { self.store.token.read(cx).value().trim().to_string() } else { String::new() };
        let team = self.store.state().and_then(|state| state.store.as_ref()).and_then(|store| store.team.clone());
        let Some(project) = self.begin_store(Act::Deploy) else { return };
        let deploy = DeployProjectStore { team, cloudflare_token: (!typed.is_empty()).then_some(typed) };
        let press = self.wire.deploy_project_store(&project, deploy, cx);
        self.store._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let (outcome, said) = match answer {
                    Ok(done) => (Some(done), Ok(())),
                    Err(refused) => (None, Err(refused.said)),
                };
                if !this.answered_store(&project, Act::Deploy, said, cx) {
                    return;
                }
                match outcome {
                    Some(StoreDeployed::Deployed { state, join_said }) => {
                        let asked = this.store.ask.take().is_some();
                        this.store.state = Some(Ok(state));
                        this.store.join_said = join_said;
                        this.store.token.update(cx, |state, cx| state.set_value("", window, cx));
                        if asked {
                            this.hand_keys_to("deploy", window, cx);
                        }
                    }
                    Some(StoreDeployed::NeedsToken { page, said }) => {
                        this.store.ask = Some(TokenAsk { said, page });
                        let token = this.store.token.read(cx).focus_handle(cx);
                        window.defer(cx, move |window, cx| window.focus(&token, cx));
                    }
                    None => {}
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// `POST /projects/project/:project/store` with the address and team typed. A join that is taken empties both
    /// fields; one that is refused keeps what was typed.
    fn join_store(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let address = self.store.address.read(cx).value().trim().to_string();
        if address.is_empty() {
            return;
        }
        let team = self.store.team.read(cx).value().trim().to_string();
        let Some(project) = self.begin_store(Act::Join) else { return };
        let press = self.wire.set_project_store(&project, Some(JoinStore { address, team: (!team.is_empty()).then_some(team) }), cx);
        self.store._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let (joined, said) = match answer {
                    Ok(state) => (Some(state), Ok(())),
                    Err(refused) => (None, Err(refused.said)),
                };
                if !this.answered_store(&project, Act::Join, said, cx) {
                    return;
                }
                if let Some(state) = joined {
                    this.store.state = Some(Ok(state));
                    this.store.ask = None;
                    for field in [this.store.address.clone(), this.store.team.clone()] {
                        field.update(cx, |state, cx| state.set_value("", window, cx));
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    // ---- drawn ----------------------------------------------------------------------------------

    pub(super) fn render_store(&self, entry: &'static Entry, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let spread = self.spread(look, window, cx);
        let Some((_, name)) = &self.scope.project else {
            let card = self.every_project(look, spread, window, cx);
            return self.rows_page(entry, "Where a project's Jobs send their claims.".into(), false, Vec::new(), vec![card], look, window, cx);
        };
        let purpose = SharedString::from(format!("Where {name}'s Jobs send their claims, and the Macs signed in to it."));
        let body = match (&self.store.state, self.store.state()) {
            (_, Some(state)) => self.store_rows(entry, name, state, look, spread, window, cx),
            (Some(Err(said)), _) => refused_line(look, "store-read-refused", said),
            _ => self.row_bones(look, spread),
        };
        self.rows_page(entry, purpose, false, Vec::new(), vec![body], look, window, cx)
    }

    /// With Every project picked: one line, and the press that opens the picker.
    fn every_project(&self, look: &Look, spread: Spread, window: &Window, cx: &Context<Self>) -> AnyElement {
        let said = "Each project picks its own store.";
        let control = div().flex().child(self.press(look, "pick", "Pick a project", Some("layers"), Weight::Plain, false, false, window, cx));
        let row = Row { id: "store-row-every".into(), title: said.into(), tip: None, chips: Vec::new(), says: Vec::new(), changed: false, glyph: None, control: control.into_any_element() };
        rows_card(look, "store-every", vec![labelled(split_row(look, spread, row, true), Role::Row, said).into_any_element()]).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn store_rows(&self, entry: &Entry, project: &str, state: &ProjectStoreState, look: &Look, spread: Spread, window: &Window, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(entry.hue);
        let page = &self.store;
        let busy = page.out;
        let asking = page.ask.is_some();
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut row = |look: &Look, id: &'static str, title: &'static str, says_words: String, glyph: &'static str, edge: Option<Hsla>, control: AnyElement| {
            let row = Row { id: id.into(), title: title.into(), tip: None, chips: Vec::new(), says: vec![says(look, says_words).into_any_element()], changed: false, glyph: Some((glyph, hue)), control };
            let drawn = split_row(look, spread, row, rows.is_empty()).when_some(edge, |row, edge| row.child(div().absolute().left_0().top_0().bottom_0().w(look.length(token::BORDER_WIDTH) * 3.).bg(edge)));
            rows.push(labelled(drawn, Role::Row, title).into_any_element());
        };

        // Claims store
        let mut line = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        let mut control = div().flex().flex_col().items_start().gap(look.length(token::SPACE_3)).min_w_0();
        match &state.store {
            None => {
                line = line.child(pill(look, "store-none", None, "None", look.color(token::STATUS_NOT_STARTED), look.color(token::STATUS_NOT_STARTED_BG), None));
            }
            Some(store) => {
                let address = labelled(
                    div().id("store-address").max_w_full().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).tooltip(tip(store.address.clone())).child(SharedString::from(store.address.clone())),
                    Role::Label,
                    "Store address",
                );
                control = control.w_full().child(address);
                if let Some(team) = &store.team {
                    line = line.child(pill(look, "store-team", None, team, look.color(token::ACCENT_HOVER), look.color(token::ACCENT_MUTED), Some("The team this store admits")));
                }
                line = if state.joined_here {
                    line.child(pill(look, "store-joined", Some("check"), "Joined here", look.color(token::STATUS_COMPLETED_SUCCESS), look.color(token::STATUS_COMPLETED_SUCCESS_BG), Some("This Mac has joined it")))
                } else {
                    line.child(pill(look, "store-joined", Some("triangle-alert"), "Not joined here", look.color(token::STATUS_AWAITING_REVIEW), look.color(token::STATUS_AWAITING_REVIEW_BG), Some("This Mac has not joined it")))
                };
                if let Some(on) = store.deployed.as_deref().and_then(deployed_on) {
                    let worker = store.worker.as_ref().map(|worker| format!("Worker {worker}"));
                    line = line.child(pill(look, "store-deployed", None, &format!("Deployed {on}"), look.color(token::STATUS_NOT_STARTED), look.color(token::STATUS_NOT_STARTED_BG), worker.as_deref()));
                }
            }
        }
        control = control.child(line);
        if state.store.is_some() {
            control = control.child(self.press(look, "use-none", "Use none", None, Weight::Quiet, busy == Some(Act::UseNone), false, window, cx));
        }
        control = control.children(page.refused.get(Act::UseNone.key()).map(|said| refused_line(look, "store-none-refused", said)));
        row(look, "store-row-store", "Claims store", format!("Where {project}'s Jobs send their claims. With none, its Jobs send nothing."), "database", None, control.into_any_element());

        // Macs signed in, only with a store
        if state.store.is_some() {
            let body = if state.macs.is_empty() {
                let said = state.macs_unread.clone().unwrap_or_default();
                div().flex().items_start().gap(look.length(token::SPACE_2)).child(svg().flex_none().mt(look.length(token::SPACE_1) / 2.).path(icon_path("triangle-alert")).size(look.length(token::SPACE_4)).text_color(look.color(token::STATUS_AWAITING_REVIEW))).child(div().id("store-macs-unread").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said))).into_any_element()
            } else {
                let items = state.macs.iter().enumerate().map(|(n, mac)| {
                    let item = div()
                        .id(SharedString::from(format!("store-mac-{n}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .when(n > 0, |item| item.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                        .child(svg().flex_none().path(icon_path("laptop")).size(look.length(token::SPACE_4)).text_color(look.color(token::STATUS_COMPLETED_SUCCESS)))
                        .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(mac.clone())));
                    labelled(item, Role::ListItem, mac.clone())
                });
                labelled(div().id("store-macs").w_full().flex().flex_col().children(items), Role::List, "Macs signed in").into_any_element()
            };
            row(look, "store-row-macs", "Macs signed in", "Oldest first, as the store answered.".into(), "laptop", None, div().w_full().child(body).into_any_element());
        }

        // Deploy
        let mut control = div().w_full().flex().flex_col().items_start().gap(look.length(token::SPACE_3));
        match &page.ask {
            None => control = control.child(self.press(look, "deploy", "Deploy", Some("upload"), Weight::Primary, busy == Some(Act::Deploy), false, window, cx)),
            Some(ask) => control = control.child(self.token_ask(ask, look, window, cx)),
        }
        if !asking {
            control = control.children(page.refused.get(Act::Deploy.key()).map(|said| refused_line(look, "store-deploy-refused", said)));
        }
        control = control.children(page.join_said.as_ref().map(|said| noted_line(look, "store-join-said", said)));
        let edge = if asking { Some(look.color(token::STATUS_AWAITING_REVIEW)) } else { page.refused.get(Act::Deploy.key()).map(|_| look.color(token::STATUS_COMPLETED_FAILED)) };
        row(look, "store-row-deploy", "Deploy", "Puts a store on your Cloudflare account and joins this Mac to it.".into(), "upload", edge, control.into_any_element());

        // Join a store
        let off = page.address.read(cx).value().trim().is_empty();
        let joining = busy == Some(Act::Join);
        let join = self.press(look, "join", "Join", None, Weight::Plain, joining, off, window, cx);
        let form = if spread.stacked {
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(field(look, "store-address-field", "Address", &page.address))
                .child(div().flex().items_end().gap(look.length(token::SPACE_3)).child(div().flex_1().min_w_0().child(field(look, "store-team-field", "Team", &page.team))).child(join))
        } else {
            div()
                .w_full()
                .flex()
                .items_end()
                .gap(look.length(token::SPACE_3))
                .child(div().flex_1().min_w_0().child(field(look, "store-address-field", "Address", &page.address)))
                .child(div().flex_none().w(look.length(token::SPACE_12) * 3.).child(field(look, "store-team-field", "Team", &page.team)))
                .child(join)
        };
        let control = div().w_full().flex().flex_col().gap(look.length(token::SPACE_3)).child(form).children(page.refused.get(Act::Join.key()).map(|said| refused_line(look, "store-join-refused", said)));
        let edge = page.refused.get(Act::Join.key()).map(|_| look.color(token::STATUS_COMPLETED_FAILED));
        row(look, "store-row-join", "Join a store", "Join one by its address. A team's store takes the team, as org or org/team.".into(), "link", edge, control.into_any_element());

        rows_card(look, "store-rows", rows).into_any_element()
    }

    /// The ask that opens under Deploy: Fleet's sentence, Cloudflare's page, a masked field and Deploy again.
    fn token_ask(&self, ask: &TokenAsk, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let ink = look.color(token::STATUS_AWAITING_REVIEW);
        let said = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(svg().flex_none().mt(look.length(token::SPACE_1) / 2.).path(icon_path("triangle-alert")).size(look.length(token::SPACE_4)).text_color(ink))
            .child(labelled(div().id("store-ask-said").text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(ask.said.clone())), Role::Status, ask.said.clone()));
        let deploying = self.store.out == Some(Act::Deploy);
        let line = div()
            .flex()
            .items_end()
            .gap(look.length(token::SPACE_3))
            .child(div().flex_1().min_w_0().child(field(look, "store-token-field", "Cloudflare token", &self.store.token)))
            .child(self.press(look, "deploy-again", "Deploy again", Some("upload"), Weight::Primary, deploying, false, window, cx));
        div()
            .id("store-ask")
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_4))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(ink.opacity(0.4))
            .bg(look.color(token::BG_SUNKEN).blend(ink.opacity(0.08)))
            .child(said)
            .child(div().flex().child(self.press(look, "open-page", "Open Cloudflare's token page", Some("external-link"), Weight::Plain, false, false, window, cx)))
            .child(line)
            .children(self.store.refused.get(Act::Deploy.key()).map(|said| refused_line(look, "store-deploy-refused", said)))
            .into_any_element()
    }

    /// One press: its words and glyph, busy with a turning mark and its own word while its act is out.
    #[allow(clippy::too_many_arguments)]
    fn press(&self, look: &Look, name: &'static str, said: &'static str, glyph: Option<&'static str>, weight: Weight, busy: bool, blank: bool, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.store.keys.get(name).cloned() else { return div().into_any_element() };
        let off = busy || blank || self.store.out.is_some() && name != "pick";
        let (ink, fill, edge) = match weight {
            Weight::Primary => (look.color(token::FG_INVERSE), look.color(token::ACCENT), look.color(token::ACCENT)),
            Weight::Plain => (look.color(token::FG_DEFAULT), look.color(token::BG_OVERLAY), look.color(token::BORDER_STRONG)),
            Weight::Quiet => (look.color(token::FG_MUTED), gpui::transparent_black(), look.color(token::BORDER_DEFAULT)),
        };
        let shown = match (busy, name) {
            (true, "deploy" | "deploy-again") => "Deploying",
            (true, "join") => "Joining",
            (true, _) => "Working",
            _ => said,
        };
        let mark = if busy {
            Some(svg().flex_none().path(icon_path("loader-circle")).size(look.length(token::SPACE_4)).text_color(ink).with_animation(
                SharedString::from(format!("store-busy-{name}")),
                Animation::new(look.duration(token::DURATION_SPIN)).repeat(),
                |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
            ).into_any_element())
        } else {
            glyph.map(|glyph| svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(ink).into_any_element())
        };
        let button = div()
            .id(SharedString::from(format!("store-{name}")))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(fill)
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(ink)
            .when(off && !busy, |button| button.opacity(0.5))
            .when(!off, |button| {
                let hover = match weight {
                    Weight::Primary => look.color(token::ACCENT_HOVER),
                    _ => look.color(token::BG_HOVER),
                };
                button.cursor_pointer().hover(move |style| style.bg(hover)).on_click(cx.listener(move |this, _, window, cx| this.press_store(name, window, cx)))
            })
            .when(name == "pick", |button| button.tooltip(tip("\u{2318}P")))
            .children(mark)
            .child(shown);
        let button = ringed(look, button, &focus, window).track_focus(&focus);
        // A name is what a person hears: the press as it reads now.
        labelled(button, Role::Button, shown).into_any_element()
    }
}

/// How a press is weighted: Deploy's fill, a plain press, or the quiet Use none.
#[derive(Clone, Copy)]
enum Weight {
    Primary,
    Plain,
    Quiet,
}

/// A chip: its words in a state's ink on that state's wash, with a registry glyph before them where it has one, and a
/// tooltip saying what the state is.
fn pill(look: &Look, id: &'static str, glyph: Option<&'static str>, said: &str, ink: Hsla, wash: Hsla, why: Option<&str>) -> AnyElement {
    let chip = div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded_full()
        .bg(wash)
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(ink)
        .when_some(why.map(str::to_string), |chip, why| chip.tooltip(tip(why)))
        .children(glyph.map(|glyph| svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(ink)))
        .child(SharedString::from(said.to_string()));
    labelled(chip, Role::Status, said.to_string()).into_any_element()
}

/// A labelled text field: its name over a box that holds the words typed.
fn field(look: &Look, id: &'static str, label: &'static str, input: &Entity<InputState>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_MUTED)).child(label))
        .child(labelled(div().id(id).w_full().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(input)), Role::TextInput, label))
        .into_any_element()
}

/// What Fleet refused with, in its own words, in a red box with a mark.
fn refused_line(look: &Look, id: &'static str, said: &str) -> AnyElement {
    let ink = look.color(token::STATUS_COMPLETED_FAILED);
    let line = div()
        .id(id)
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(ink.opacity(0.4))
        .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
        .child(svg().flex_none().mt(look.length(token::SPACE_1) / 2.).path(icon_path("triangle-alert")).size(look.length(token::SPACE_4)).text_color(ink))
        .child(div().flex_1().min_w_0().child(refusal(look, said.to_string())));
    labelled(line, Role::Alert, said.to_string()).into_any_element()
}

/// Why this Mac could not join the store a deploy made, as the deploy said it: an amber mark and the sentence.
fn noted_line(look: &Look, id: &'static str, said: &str) -> AnyElement {
    let line = div()
        .id(id)
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_2))
        .child(svg().flex_none().mt(look.length(token::SPACE_1) / 2.).path(icon_path("triangle-alert")).size(look.length(token::SPACE_4)).text_color(look.color(token::STATUS_AWAITING_REVIEW)))
        .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.to_string())));
    labelled(line, Role::Status, said.to_string()).into_any_element()
}
