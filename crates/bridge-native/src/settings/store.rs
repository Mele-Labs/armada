//! Settings › Claims store: the store a project's Jobs send their claims to, drawn as split rows (the approved take
//! "Rows", 14 Nov 2026). The store the project names with its team, whether this Mac has joined it and when it was
//! deployed; the Macs signed in to it; Deploy, which asks for a Cloudflare token in place where Cloudflare refused
//! every sign-in this Mac has, and reads Update the store once this Mac has deployed it (a store it only joined has no
//! such row); and Join by address. **Every sentence a refusal carries is Fleet's**
//! (`crates/ipc/operations/set_project_store.toml`, `deploy_project_store.toml`).
//!
//! **A token is never written anywhere.** It is held in a masked field in memory, sent in the deploy's body and
//! dropped once the deploy works, so it is kept out of the drafts file the other boxes use.

use std::collections::BTreeMap;

use gpui::{div, prelude::*, svg, Animation, AnimationExt, AnyElement, ClipboardItem, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla, Keystroke, Role, SharedString, Subscription, Task, Window};
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

/// What the token ask says first: our words, over Cloudflare's own under it.
const ASK_HEADLINE: &str = "Cloudflare needs a token that can make Workers on your account.";

/// How to make that token on Cloudflare's page, which opens filled in with the two rights a deploy needs.
const ASK_STEPS: [&str; 3] = [
    "Open Cloudflare's token page. It opens filled in with the two rights a deploy needs: Workers Scripts Edit and Account Settings Read.",
    "Press Continue to summary, then Create Token.",
    "Copy the token Cloudflare shows once and paste it here. Not the Global API Key, and not the test command under it.",
];

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
    /// The address was copied a moment ago, so its press reads Copied.
    copied: bool,
    pub(super) token: Entity<InputState>,
    pub(super) address: Entity<InputState>,
    pub(super) team: Entity<InputState>,
    keys: BTreeMap<&'static str, FocusHandle>,
    /// The login the project joins as, and each repository's own (`store_login.rs`).
    pub(super) logins: super::store_login::Logins,
    _read: Option<Task<()>>,
    _act: Option<Task<()>>,
    _copied: Option<Task<()>>,
    _fields: Vec<Subscription>,
}

const PRESSES: [&str; 7] = ["pick", "copy", "use-none", "deploy", "open-page", "deploy-again", "join"];

/// How long the address's press reads Copied.
const COPIED_FOR: std::time::Duration = std::time::Duration::from_secs(2);

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
        StorePage { project: None, state: None, ask: None, out: None, refused: BTreeMap::new(), join_said: None, copied: false, token, address, team, keys: BTreeMap::new(), logins: super::store_login::Logins::new(), _read: None, _act: None, _copied: None, _fields: fields }
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

/// What the Deploy row is for a project's store: Deploy where it names none, Update where this Mac deployed it, and no
/// row where it was only joined by its address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Deploys {
    Deploy,
    Update,
    Neither,
}

fn deploys(state: &ProjectStoreState) -> Deploys {
    match &state.store {
        None => Deploys::Deploy,
        Some(store) if store.deployed.is_some() => Deploys::Update,
        Some(_) => Deploys::Neither,
    }
}

/// How many characters of an address a sentence carries before it is cut.
const ADDRESS_SHOWN: usize = 56;

/// The address as a sentence carries it, and the whole of it for a tooltip where it was cut.
fn trimmed(address: &str) -> (String, Option<String>) {
    if address.chars().count() <= ADDRESS_SHOWN {
        return (address.to_string(), None);
    }
    let head: String = address.chars().take(ADDRESS_SHOWN - 1).collect();
    (format!("{head}\u{2026}"), Some(address.to_string()))
}

fn draft_key(project: &str, field: &str) -> String {
    format!("store:{project}:{field}")
}

impl SettingsOverlay {
    // ---- held on another Mac --------------------------------------------------------------------

    /// The name of the Mac that keeps the project picked, where it is a copy of that Mac's project. **Its place and its
    /// store are changed on that Mac alone**; everything else of the project is edited here as for his own.
    pub(super) fn kept_by(&self, cx: &gpui::App) -> Option<SharedString> {
        let (id, _) = self.scope.project.as_ref()?;
        let mac = self.scope.projects.as_ref()?.projects.iter().find(|one| one.id == *id)?.mac.as_deref()?;
        Some(crate::job_view::name_of(crate::pairing::macs(cx), mac, false).into())
    }

    /// Whether a press changes the store, which a copy leaves to the Mac that keeps the project.
    fn changes_store(press: &str) -> bool {
        matches!(press, "use-none" | "deploy" | "deploy-again" | "join")
    }

    /// What the tooltip of a control the keeping Mac holds says.
    fn held_tip(mac: &str, of: &str) -> String {
        format!("Kept on {mac}. Only {mac} can change {of}.")
    }

    /// For a copy: where its file is kept, which is the other Mac's folder or bucket, as a row that cannot be pressed.
    /// None for his own project.
    pub(super) fn kept_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let mac = self.kept_by(cx)?;
        let (id, _) = self.scope.project.as_ref()?;
        let place = crate::titlebar::place_of(&self.scope.projects.as_ref()?.projects.iter().find(|one| one.id == *id)?.place);
        let said = match place.kind {
            crate::titlebar::RunsPlaceKind::Mac => format!("On {mac}"),
            _ => {
                let phrase = place.phrase();
                let mut chars = phrase.chars();
                chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
            }
        };
        let title = "Where its file is kept";
        let chip = pill(look, "kept-place", Some("lock"), &said, look.color(token::FG_MUTED), look.color(token::BG_SUNKEN), Some(&Self::held_tip(&mac, "where it is kept")));
        let row = Row {
            id: "kept-row-place".into(),
            title: title.into(),
            tip: None,
            chips: Vec::new(),
            says: vec![says(look, format!("Only {mac} can change it.")).into_any_element()],
            changed: false,
            glyph: Some(("folder", look.color(token::FG_SUBTLE))),
            control: div().flex().child(chip).into_any_element(),
        };
        let spread = self.spread(look, window, cx);
        Some(rows_card(look, "kept-place", vec![labelled(split_row(look, spread, row, true), Role::Row, title).into_any_element()]).into_any_element())
    }

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
        page.logins.opened.clear();
        page.logins.refused = None;
        let (address, team, token) = (page.address.clone(), page.team.clone(), page.token.clone());
        let kept = |field: &str, cx: &mut Context<Self>| crate::drafts::kept(&draft_key(&id, field), cx).unwrap_or_default();
        let (said_address, said_team) = (kept("address", cx), kept("team", cx));
        address.update(cx, |state, cx| state.set_value(said_address, window, cx));
        team.update(cx, |state, cx| state.set_value(said_team, window, cx));
        token.update(cx, |state, cx| state.set_value("", window, cx));
        // The logins signed in here, which Joins as lists.
        self.read_accounts(window, cx);
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
        self.login_keys(cx);
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
            stops.extend(key("copy"));
            stops.extend(key("use-none"));
            stops.extend(self.login_stops(cx));
        }
        if page.ask.is_some() {
            stops.extend(key("open-page"));
            stops.push(page.token.read(cx).focus_handle(cx));
            stops.extend(key("deploy-again"));
        } else if deploys(state) != Deploys::Neither {
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
        if self.login_key(key, window, cx) {
            return true;
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
        if Self::changes_store(name) && self.kept_by(cx).is_some() {
            return;
        }
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
            "copy" => self.copy_address(window, cx),
            _ => {}
        }
    }

    // ---- acts -----------------------------------------------------------------------------------

    /// Writes the store's address to the clipboard, and reads Copied for a moment.
    fn copy_address(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(address) = self.store.state().and_then(|state| state.store.as_ref()).map(|store| store.address.clone()) else { return };
        cx.write_to_clipboard(ClipboardItem::new_string(address));
        self.store.copied = true;
        self.store._copied = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.store.copied = false;
                cx.notify();
            });
        }));
        cx.notify();
    }

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
        let sent_token = !typed.is_empty();
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
                        // A token Cloudflare turned down is no use to edit: the field is emptied for the next paste.
                        if sent_token {
                            this.store.token.update(cx, |state, cx| state.set_value("", window, cx));
                        }
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
        let press = self.wire.set_project_store(&project, Some(JoinStore { address, team: (!team.is_empty()).then_some(team), account_id: None }), cx);
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
        let mut row = |look: &Look, id: &'static str, title: &'static str, says_words: String, says_tip: Option<String>, glyph: &'static str, edge: Option<Hsla>, control: AnyElement| {
            let said = says(look, says_words);
            let said = match says_tip {
                Some(full) => said.id(SharedString::from(format!("{id}-says"))).tooltip(tip(full)).into_any_element(),
                None => said.into_any_element(),
            };
            let row = Row { id: id.into(), title: title.into(), tip: None, chips: Vec::new(), says: vec![said], changed: false, glyph: Some((glyph, hue)), control };
            let drawn = split_row(look, spread, row, rows.is_empty()).when_some(edge, |row, edge| row.border_1().border_color(edge).bg(Hsla { a: 0.1, ..edge }));
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
                let address = div().flex().items_center().gap(look.length(token::SPACE_2)).w_full().child(div().min_w_0().child(address)).child(self.copy_press(look, window, cx));
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
        if let Some(mac) = self.kept_by(cx) {
            line = line.child(pill(look, "store-kept", Some("lock"), &format!("Kept on {mac}"), look.color(token::FG_MUTED), look.color(token::BG_SUNKEN), Some(&Self::held_tip(&mac, "its store"))));
        }
        control = control.child(line);
        if state.store.is_some() {
            control = control.child(self.press(look, "use-none", "Use none", None, Weight::Quiet, busy == Some(Act::UseNone), false, window, cx));
        }
        control = control.children(page.refused.get(Act::UseNone.key()).map(|said| refused_line(look, "store-none-refused", said)));
        row(look, "store-row-store", "Claims store", format!("Where {project}'s Jobs send their claims. With none, its Jobs send nothing."), None, "database", None, control.into_any_element());

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
            row(look, "store-row-macs", "Macs signed in", "Oldest first, as the store answered.".into(), None, "laptop", None, div().w_full().child(body).into_any_element());
        }

        // Deploy, or Update where this Mac deployed the store; a store it only joined has neither
        let updating = deploys(state) == Deploys::Update;
        if deploys(state) != Deploys::Neither {
            let mut control = div().w_full().flex().flex_col().items_start().gap(look.length(token::SPACE_3));
            match &page.ask {
                None => control = control.child(self.press(look, "deploy", if updating { "Update" } else { "Deploy" }, Some("upload"), Weight::Primary, busy == Some(Act::Deploy), false, window, cx)),
                Some(ask) => control = control.child(self.token_ask(ask, updating, look, window, cx)),
            }
            if !asking {
                control = control.children(page.refused.get(Act::Deploy.key()).map(|said| refused_line(look, "store-deploy-refused", said)));
            }
            control = control.children(page.join_said.as_ref().map(|said| noted_line(look, "store-join-said", said)));
            let edge = if asking { Some(look.color(token::STATUS_AWAITING_REVIEW)) } else { page.refused.get(Act::Deploy.key()).map(|_| look.color(token::STATUS_COMPLETED_FAILED)) };
            match state.store.as_ref().filter(|_| updating) {
                Some(store) => {
                    let (shown, full) = trimmed(&store.address);
                    row(look, "store-row-deploy", "Update the store", format!("Puts Armada's newest store on {shown}. Claims and joined Macs stay."), full, "upload", edge, control.into_any_element());
                }
                None => row(look, "store-row-deploy", "Deploy", "Puts a store on your Cloudflare account and joins this Mac to it.".into(), None, "upload", edge, control.into_any_element()),
            }
        }

        // Join a store
        let held = self.kept_by(cx).is_some();
        let off = page.address.read(cx).value().trim().is_empty();
        let joining = busy == Some(Act::Join);
        let join = self.press(look, "join", "Join", None, Weight::Plain, joining, off, window, cx);
        let form = if spread.stacked {
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(field(look, "store-address-field", "Address", &page.address, held))
                .child(div().flex().items_end().gap(look.length(token::SPACE_3)).child(div().flex_1().min_w_0().child(field(look, "store-team-field", "Team", &page.team, held))).child(join))
        } else {
            div()
                .w_full()
                .flex()
                .items_end()
                .gap(look.length(token::SPACE_3))
                .child(div().flex_1().min_w_0().child(field(look, "store-address-field", "Address", &page.address, held)))
                .child(div().flex_none().w(look.length(token::SPACE_12) * 3.).child(field(look, "store-team-field", "Team", &page.team, held)))
                .child(join)
        };
        let control = div().w_full().flex().flex_col().gap(look.length(token::SPACE_3)).child(form).children(page.refused.get(Act::Join.key()).map(|said| refused_line(look, "store-join-refused", said)));
        let edge = page.refused.get(Act::Join.key()).map(|_| look.color(token::STATUS_COMPLETED_FAILED));
        row(look, "store-row-join", "Join a store", "Join one by its address. Fill Team only for a store a GitHub organisation shares, as org or org/team; leave it empty for your own.".into(), None, "link", edge, control.into_any_element());

        // Joins as, and the repositories with a login of their own, stand under the store.
        let under = self.login_rows(state, look, spread, window, cx);
        let at = rows.len().min(1);
        rows.splice(at..at, under);
        rows_card(look, "store-rows", rows).into_any_element()
    }

    /// The ask that opens under Deploy: our headline over Cloudflare's own sentence, Cloudflare's page with the steps to
    /// make a token on it, a masked field and Deploy again.
    fn token_ask(&self, ask: &TokenAsk, updating: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let ink = look.color(token::STATUS_AWAITING_REVIEW);
        let muted = |id: &'static str, said: &str| {
            labelled(div().id(id).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.to_string())), Role::Status, said.to_string())
        };
        let said = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(svg().flex_none().mt(look.length(token::SPACE_1) / 2.).path(icon_path("triangle-alert")).size(look.length(token::SPACE_4)).text_color(ink))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(labelled(div().id("store-ask-headline").text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(ASK_HEADLINE), Role::Status, ASK_HEADLINE))
                    .children((!ask.said.is_empty()).then(|| muted("store-ask-said", &ask.said))),
            );
        let steps = div().id("store-ask-steps").w_full().flex().flex_col().gap(look.length(token::SPACE_1)).children(ASK_STEPS.iter().enumerate().map(|(at, step)| {
            let item = div()
                .id(SharedString::from(format!("store-ask-step-{}", at + 1)))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(div().flex_none().w(look.length(token::SPACE_4)).child(SharedString::from(format!("{}.", at + 1))))
                .child(div().flex_1().min_w_0().child(*step));
            labelled(item, Role::ListItem, *step)
        }));
        let deploying = self.store.out == Some(Act::Deploy);
        let line = div()
            .flex()
            .items_end()
            .gap(look.length(token::SPACE_3))
            .child(div().flex_1().min_w_0().child(field(look, "store-token-field", "Cloudflare token", &self.store.token, false)))
            .child(self.press(look, "deploy-again", if updating { "Update again" } else { "Deploy again" }, Some("upload"), Weight::Primary, deploying, false, window, cx));
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
            .child(labelled(steps, Role::List, "Steps to make the token"))
            .child(line)
            .children(self.store.refused.get(Act::Deploy.key()).map(|said| refused_line(look, "store-deploy-refused", said)))
            .into_any_element()
    }

    /// The address's own press: an icon with a tooltip, a check and Copied for a moment once pressed.
    fn copy_press(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.store.keys.get("copy").cloned() else { return div().into_any_element() };
        let copied = self.store.copied;
        let said = if copied { "Copied" } else { "Copy the address" };
        let ink = if copied { look.color(token::STATUS_COMPLETED_SUCCESS) } else { look.color(token::FG_SUBTLE) };
        let button = div()
            .id("store-copy")
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_1))
            .h(look.length(token::H_CONTROL_SM))
            .min_w(look.length(token::H_CONTROL_SM))
            .when(copied, |button| button.px(look.length(token::SPACE_2)))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(ink)
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .tooltip(tip(said))
            .on_click(cx.listener(|this, _, window, cx| this.press_store("copy", window, cx)))
            .child(svg().flex_none().path(icon_path(if copied { "check" } else { "copy" })).size(look.length(token::SPACE_4)).text_color(ink))
            .children(copied.then_some("Copied"));
        let button = ringed(look, button, &focus, window).track_focus(&focus);
        labelled(button, Role::Button, said).into_any_element()
    }

    /// One press: its words and glyph, busy with a turning mark and its own word while its act is out.
    #[allow(clippy::too_many_arguments)]
    fn press(&self, look: &Look, name: &'static str, said: &'static str, glyph: Option<&'static str>, weight: Weight, busy: bool, blank: bool, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.store.keys.get(name).cloned() else { return div().into_any_element() };
        let held = if Self::changes_store(name) { self.kept_by(cx) } else { None };
        let off = busy || blank || held.is_some() || self.store.out.is_some() && name != "pick";
        let (ink, fill, edge) = match weight {
            Weight::Primary => (look.color(token::FG_INVERSE), look.color(token::ACCENT), look.color(token::ACCENT)),
            Weight::Plain => (look.color(token::FG_DEFAULT), look.color(token::BG_OVERLAY), look.color(token::BORDER_STRONG)),
            Weight::Quiet => (look.color(token::FG_MUTED), gpui::transparent_black(), look.color(token::BORDER_DEFAULT)),
        };
        let updating = self.store.state().is_some_and(|state| deploys(state) == Deploys::Update);
        let shown = match (busy, name) {
            (true, "deploy" | "deploy-again") if updating => "Updating",
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
            .when_some(held.clone(), |button, mac| button.tooltip(tip(Self::held_tip(&mac, "its store"))))
            .children(mark)
            .child(shown);
        let button = ringed(look, button, &focus, window).track_focus(&focus);
        // A name is what a person hears: the press as it reads now, and the Mac that keeps what it would change.
        let heard = match &held {
            Some(mac) => format!("{shown}, kept on {mac}"),
            None => shown.to_string(),
        };
        labelled(button, Role::Button, heard).into_any_element()
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
fn field(look: &Look, id: &'static str, label: &'static str, input: &Entity<InputState>, held: bool) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_MUTED)).child(label))
        .child(labelled(div().id(id).w_full().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(input).disabled(held)), Role::TextInput, label))
        .into_any_element()
}

/// What Fleet refused with, in its own words, in a red box with a mark.
pub(super) fn refused_line(look: &Look, id: &'static str, said: &str) -> AnyElement {
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

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, VisualTestContext};

    use super::super::mock::MockFleet;
    use super::super::mock_store::{StoreCall, NOT_A_MEMBER};
    use super::super::wire::Wire;
    use super::*;

    fn opened<'a>(cx: &'a mut TestAppContext, project: Option<(&str, &str)>) -> (Entity<SettingsOverlay>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (overlay, cx) = cx.add_window_view(|window, cx| SettingsOverlay::new(Wire::Mock(Box::new(MockFleet::new())), window, cx));
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                this.open = true;
                this.picked = "project:claims-store";
                this.scope.project = project.map(|(id, name)| (id.to_string(), name.to_string()));
                this.store_keys(cx);
                this.read_store(window, cx);
            })
        });
        cx.run_until_parked();
        (overlay, cx)
    }

    fn press(overlay: &Entity<SettingsOverlay>, name: &str, cx: &mut VisualTestContext) {
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.press_store(name, window, cx)));
        cx.run_until_parked();
    }

    fn type_into(overlay: &Entity<SettingsOverlay>, field: &'static str, words: &str, cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                let input = match field {
                    "token" => this.store.token.clone(),
                    "address" => this.store.address.clone(),
                    _ => this.store.team.clone(),
                };
                input.update(cx, |state, cx| state.set_value(words.to_string(), window, cx));
            })
        });
        cx.run_until_parked();
    }

    fn calls(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Vec<StoreCall> {
        overlay.read_with(cx, |this, _| match &this.wire {
            Wire::Mock(mock) => mock.stores.calls.clone(),
            Wire::Live(_) => Vec::new(),
        })
    }

    /// Draws Settings and says whether a labelled element is on the page.
    fn on_page(overlay: &Entity<SettingsOverlay>, role: Role, name: &str, cx: &mut VisualTestContext) -> bool {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = overlay;
            window.draw(cx).clear(cx);
            cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some())
        })
    }

    /// The project picked is a copy of one the Mac named Studio keeps.
    fn kept_on_studio(overlay: &Entity<SettingsOverlay>, id: &str, cx: &mut VisualTestContext) {
        cx.update(|_, cx| crate::pairing::hold(vec![ipc::Mac { id: "mac-studio".into(), name: "Studio".into(), ..Default::default() }], cx));
        let copy = ipc::Project { id: id.into(), name: id.into(), mac: Some("mac-studio".into()), place: ipc::ProjectPlace::default(), ..ipc::Project::default() };
        overlay.update(cx, |this, _| this.scope.projects = Some(ipc::Projects { projects: vec![copy], loose: Vec::new(), left_out: Vec::new(), left_behind: Vec::new() }));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }

    #[gpui::test]
    fn a_copys_store_is_shown_as_the_keeping_macs_and_no_press_changes_it(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("armada", "Armada")));
        kept_on_studio(&overlay, "armada", cx);
        type_into(&overlay, "address", "https://store.example.com", cx);
        for name in ["use-none", "deploy", "deploy-again", "join"] {
            press(&overlay, name, cx);
        }
        assert!(calls(&overlay, cx).is_empty(), "Fleet is asked nothing of a store the other Mac keeps");
        assert!(on_page(&overlay, Role::Status, "Kept on Studio", cx), "the store row says which Mac keeps it");
        for name in ["Use none, kept on Studio", "Update, kept on Studio"] {
            assert!(on_page(&overlay, Role::Button, name, cx), "{name}");
        }
        assert!(on_page(&overlay, Role::Button, "Copy the address", cx), "reading the store is still there");
        overlay.read_with(cx, |this, cx| {
            assert_eq!(this.kept_by(cx).as_deref(), Some("Studio"));
            assert!(this.store.state().is_some_and(|state| state.store.is_some()), "its state is what Fleet answered");
        });
    }

    #[gpui::test]
    fn his_own_projects_store_is_pressed_as_before_and_wears_no_kept_chip(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("armada", "Armada")));
        assert!(!on_page(&overlay, Role::Status, "Kept on Studio", cx));
        assert!(on_page(&overlay, Role::Button, "Use none", cx));
        press(&overlay, "use-none", cx);
        assert_eq!(calls(&overlay, cx).len(), 1);
    }

    #[gpui::test]
    fn a_copys_place_is_shown_as_the_keeping_macs_on_the_page_of_its_macs(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("armada", "Armada")));
        kept_on_studio(&overlay, "armada", cx);
        overlay.update(cx, |this, _| this.picked = "project:shared");
        assert!(on_page(&overlay, Role::Status, "On Studio", cx), "where its file is kept, as the keeping Mac's");
        assert!(on_page(&overlay, Role::Row, "Where its file is kept", cx));
        overlay.update(cx, |this, cx| {
            this.scope.projects = None;
            cx.notify();
        });
        assert!(overlay.read_with(cx, |this, cx| this.kept_by(cx).is_none()), "his own project has no such row");
    }

    #[gpui::test]
    fn deploy_sends_the_token_only_when_one_is_typed(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("storefront", "Storefront")));
        // Typed before the ask is open, a token is not sent: the first press carries none.
        type_into(&overlay, "token", "cfut_early", cx);
        press(&overlay, "deploy", cx);
        assert!(overlay.read_with(cx, |this, _| this.store.ask.is_some()), "Fleet asked for a token");
        // Deploy again with the field empty sends none either.
        type_into(&overlay, "token", "", cx);
        press(&overlay, "deploy-again", cx);
        type_into(&overlay, "token", "cfut_typed", cx);
        press(&overlay, "deploy-again", cx);
        let tokens: Vec<Option<String>> = calls(&overlay, cx)
            .into_iter()
            .filter_map(|call| if let StoreCall::Deploy { deploy, .. } = call { Some(deploy.cloudflare_token) } else { None })
            .collect();
        assert_eq!(tokens, [None, None, Some("cfut_typed".to_string())]);
        overlay.read_with(cx, |this, cx| {
            assert!(this.store.ask.is_none(), "a deploy that works closes the ask");
            assert!(this.store.token.read(cx).value().is_empty(), "and the token is dropped");
            assert!(this.store.state().is_some_and(|state| state.store.is_some() && state.macs.len() == 3));
        });
    }

    #[gpui::test]
    fn a_token_cloudflare_cannot_read_asks_again_with_its_sentence_and_an_empty_field(cx: &mut TestAppContext) {
        let said = "Cloudflare could not read that as an API token. Paste the token it shows once after Create Token.";
        let (overlay, cx) = opened(cx, Some(("storefront", "Storefront")));
        press(&overlay, "deploy", cx);
        assert!(on_page(&overlay, Role::Status, ASK_HEADLINE, cx), "our headline leads the ask");
        assert!(on_page(&overlay, Role::Status, "Cloudflare refused every sign-in this Mac has.", cx), "Cloudflare's own sentence stands under it");
        for step in ASK_STEPS {
            assert!(on_page(&overlay, Role::ListItem, step, cx), "{step}");
        }
        type_into(&overlay, "token", "curl https://api.cloudflare.com/client/v4/user/tokens/verify", cx);
        press(&overlay, "deploy-again", cx);
        overlay.read_with(cx, |this, cx| {
            assert!(this.store.ask.is_some(), "the ask stays open");
            assert!(this.store.token.read(cx).value().is_empty(), "the paste Cloudflare could not read is dropped");
        });
        assert!(on_page(&overlay, Role::Status, said, cx), "and says why");
        type_into(&overlay, "token", "cfut_typed", cx);
        press(&overlay, "deploy-again", cx);
        assert!(overlay.read_with(cx, |this, _| this.store.ask.is_none()), "a token it can read deploys");
    }

    #[gpui::test]
    fn a_refusal_shows_fleets_sentence_and_gives_the_press_back(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("storefront", "Storefront")));
        type_into(&overlay, "address", "https://armada-store-acme-platform.mele.workers.dev", cx);
        type_into(&overlay, "team", NOT_A_MEMBER, cx);
        press(&overlay, "join", cx);
        overlay.read_with(cx, |this, cx| {
            assert_eq!(this.store.refused.get("join").map(String::as_str), Some("GitHub says you are not a member of acme/platform."));
            assert!(this.store.out.is_none(), "the press is back");
            assert_eq!(this.store.address.read(cx).value().as_ref(), "https://armada-store-acme-platform.mele.workers.dev", "what was typed is kept");
        });
        assert!(on_page(&overlay, Role::Alert, "GitHub says you are not a member of acme/platform.", cx), "drawn under the press");
        let said = "The Cloudflare account has no workers.dev subdomain yet.";
        overlay.update(cx, |this, _| {
            if let Wire::Mock(mock) = &mut this.wire {
                mock.stores.deploy_refusal = Some(said.into());
            }
        });
        press(&overlay, "deploy", cx);
        overlay.read_with(cx, |this, _| {
            assert_eq!(this.store.refused.get("deploy").map(String::as_str), Some(said));
            assert!(this.store.out.is_none());
        });
    }

    #[gpui::test]
    fn use_none_posts_an_absent_store(cx: &mut TestAppContext) {
        assert_eq!(serde_json::to_string(&ipc::SetProjectStore { store: None }).as_deref().ok(), Some("{}"), "absent is none on the wire");
        let (overlay, cx) = opened(cx, Some(("armada", "Armada")));
        assert!(overlay.read_with(cx, |this, _| this.store.state().is_some_and(|state| state.store.is_some())), "Armada names a store");
        press(&overlay, "use-none", cx);
        assert_eq!(calls(&overlay, cx), [StoreCall::Set { project: "armada".into(), store: None }]);
        overlay.read_with(cx, |this, _| assert!(this.store.state().is_some_and(|state| state.store.is_none() && state.macs.is_empty())));
    }

    #[gpui::test]
    fn the_page_asks_for_a_project_under_every_project(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, None);
        assert!(on_page(&overlay, Role::Button, "Pick a project", cx));
        assert!(!on_page(&overlay, Role::Button, "Deploy", cx), "no store acts without a project");
        assert_eq!(overlay.read_with(cx, |this, cx| this.store_stops(cx).len()), 1, "the press is the page's one stop");
        assert!(calls(&overlay, cx).is_empty(), "nothing is read or sent for every project");
    }

    #[gpui::test]
    fn a_store_this_mac_deployed_reads_update_where_none_reads_deploy(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("storefront", "Storefront")));
        assert!(on_page(&overlay, Role::Row, "Deploy", cx) && on_page(&overlay, Role::Button, "Deploy", cx), "with no store, Deploy as before");
        assert!(!on_page(&overlay, Role::Row, "Update the store", cx));
        press(&overlay, "deploy", cx);
        type_into(&overlay, "token", "cfut_typed", cx);
        press(&overlay, "deploy-again", cx);
        assert!(on_page(&overlay, Role::Row, "Update the store", cx), "the store it put up is the one to update");
        assert!(on_page(&overlay, Role::Button, "Update", cx));
        // The page's labels are not cleared between frames here, so what is gone is read from the state the rows are drawn from.
        assert_eq!(overlay.read_with(cx, |this, _| this.store.state().map(deploys)), Some(Deploys::Update), "nothing reads as if there were no store");
        // Updating sends the store's own team, and a kept sign-in needs no token.
        press(&overlay, "deploy", cx);
        let sent = calls(&overlay, cx).into_iter().filter(|call| matches!(call, StoreCall::Deploy { .. })).count();
        assert_eq!(sent, 3, "two presses to deploy, one to update");
        assert!(overlay.read_with(cx, |this, _| this.store.ask.is_none() && this.store.state().is_some_and(|state| state.store.is_some())));
    }

    #[gpui::test]
    fn armada_names_a_store_it_deployed_so_its_row_reads_update(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("armada", "Armada")));
        assert!(on_page(&overlay, Role::Row, "Update the store", cx));
        assert!(on_page(&overlay, Role::Button, "Update", cx));
        assert_eq!(overlay.read_with(cx, |this, cx| this.store_stops(cx).len()), 7, "Copy, Use none, Joins as, Update, the two fields and Join");
    }

    #[gpui::test]
    fn a_store_joined_by_address_has_no_deploy_row(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("storefront", "Storefront")));
        let before = overlay.read_with(cx, |this, cx| this.store_stops(cx).len());
        type_into(&overlay, "address", "https://someone-elses.example.com", cx);
        press(&overlay, "join", cx);
        assert!(overlay.read_with(cx, |this, _| this.store.state().is_some_and(|state| state.store.as_ref().is_some_and(|store| store.deployed.is_none()))));
        assert_eq!(overlay.read_with(cx, |this, _| this.store.state().map(deploys)), Some(Deploys::Neither), "neither row is drawn");
        assert_eq!(overlay.read_with(cx, |this, cx| this.store_stops(cx).len()), before + 2, "Copy and Joins as join the stops, and Use none stands where Deploy did");
    }

    #[gpui::test]
    fn copy_writes_the_address_and_reads_copied_for_a_moment(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("armada", "Armada")));
        assert!(on_page(&overlay, Role::Button, "Copy the address", cx));
        let address = overlay.read_with(cx, |this, _| this.store.state().and_then(|state| state.store.as_ref()).map(|store| store.address.clone())).expect("a store");
        assert!(overlay.read_with(cx, |this, cx| this.store_stops(cx).first() == this.store.keys.get("copy")), "the press is the page's first stop");
        // Enter on the focused press is the press.
        cx.update(|window, cx| {
            let focus = overlay.read(cx).store.keys.get("copy").cloned().expect("a key");
            window.focus(&focus, cx);
        });
        let taken = cx.update(|window, cx| overlay.update(cx, |this, cx| this.store_key(&Keystroke::parse("enter").expect("a key"), window, cx)));
        assert!(taken, "Enter presses the focused Copy");
        cx.run_until_parked();
        assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()), Some(address));
        assert!(on_page(&overlay, Role::Button, "Copied", cx));
        cx.executor().advance_clock(COPIED_FOR + std::time::Duration::from_millis(100));
        cx.run_until_parked();
        assert!(on_page(&overlay, Role::Button, "Copy the address", cx), "it reads Copy the address again");
    }

    #[test]
    fn a_long_address_is_cut_for_the_sentence_and_whole_in_the_tooltip() {
        let own = "https://armada-store.mele-labs.workers.dev";
        assert_eq!(trimmed(own), (own.to_string(), None), "his own store's address is read whole");
        let team = "https://armada-store-mele-labs-platform-team.mele-labs.workers.dev";
        let (shown, full) = trimmed(team);
        assert!(shown.ends_with('\u{2026}') && shown.chars().count() == ADDRESS_SHOWN);
        assert_eq!(full.as_deref(), Some(team));
    }

    #[gpui::test]
    fn escape_shuts_the_token_ask_and_the_press_is_drawn_again(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, Some(("storefront", "Storefront")));
        press(&overlay, "deploy", cx);
        assert!(overlay.read_with(cx, |this, _| this.store.ask.is_some()));
        let taken = cx.update(|window, cx| overlay.update(cx, |this, cx| this.store_key(&Keystroke::parse("escape").expect("a key"), window, cx)));
        assert!(taken, "Esc is the ask's while it is open");
        assert!(overlay.read_with(cx, |this, _| this.store.ask.is_none()));
        let taken = cx.update(|window, cx| overlay.update(cx, |this, cx| this.store_key(&Keystroke::parse("escape").expect("a key"), window, cx)));
        assert!(!taken, "with no ask, Esc is Settings'");
    }
}
