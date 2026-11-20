//! Settings › This Mac › Account: one page of three stacked sections, the take the owner picked on 19 Nov 2026
//! (`docs/concepts/account.md`, *In native Bridge*). Who you are: a card for each GitHub login with Sign out, and a
//! dashed card that signs another in with the device code. Your devices: Settings › Macs' own parts (`macs.rs`). What
//! it turns on: a row for each project with a store, naming the login that joins it, each opening its Claims store.
//!
//! **Cancel is Bridge's own.** Fleet has no act that stops a sign-in, so Cancel puts the code away; Fleet polls the
//! forge until the code runs out, and the next start replaces it.
//! **Kit's row is not drawn yet**: its place is the first of What it turns on, once Kit's wire says what it is in
//! step with.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{div, prelude::*, svg, Animation, AnimationExt, AnyElement, App, Context, FocusHandle, FontWeight, Hsla, Keystroke, Role, SharedString, Task, Window};
use ipc::{AccountFlow, Accounts, ProjectStoreState, SignInOutcome};

use super::entries::Entry;
use super::overlay::SettingsOverlay;
use super::rows::rows_card;
use super::store::refused_line;
use super::view::{bone, ringed, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// While a code waits, the time left is drawn again this often, and Fleet is asked every other time.
const TICK: Duration = Duration::from_secs(1);

/// How long the code's press reads Copied.
const COPIED_FOR: Duration = Duration::from_secs(2);

/// The act out. One at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    Start,
    SignOut(u64),
}

/// The page's own state: what Fleet said of the logins and of each project's store, the act out and what it said,
/// and the key of each press.
pub struct AccountPage {
    pub(super) accounts: Option<Result<Accounts, String>>,
    /// Each project's id, name and store, in the projects' order. `None` while they are read.
    pub(super) stores: Option<Vec<(String, String, ProjectStoreState)>>,
    asked: bool,
    /// The code put away with Cancel: a sign-in showing it draws Add again.
    pub(super) cancelled: Option<String>,
    /// When the code shown runs out, in Unix seconds, from the last read.
    expires_at: i64,
    pub(super) out: Option<Out>,
    /// What a press was refused with, by its key: `start`, or `sign-out:<id>`.
    pub(super) refused: BTreeMap<String, String>,
    /// What signing out said where the store kept this Mac's row.
    signed_out_said: Option<String>,
    copied: bool,
    pub(super) keys: BTreeMap<String, FocusHandle>,
    _reads: Vec<Task<()>>,
    _act: Option<Task<()>>,
    _tick: Option<Task<()>>,
    _copied: Option<Task<()>>,
}

impl AccountPage {
    pub fn new() -> AccountPage {
        AccountPage {
            accounts: None,
            stores: None,
            asked: false,
            cancelled: None,
            expires_at: 0,
            out: None,
            refused: BTreeMap::new(),
            signed_out_said: None,
            copied: false,
            keys: BTreeMap::new(),
            _reads: Vec::new(),
            _act: None,
            _tick: None,
            _copied: None,
        }
    }

    /// The page was left, or Settings shut: it is read again the next time it is drawn.
    pub fn reopen(&mut self) {
        self.asked = false;
        self._tick = None;
    }

    fn held(&self) -> Option<&Accounts> {
        self.accounts.as_ref().and_then(|read| read.as_ref().ok())
    }

    fn adding(&self) -> Adding {
        let Some(accounts) = self.held() else { return Adding::Add };
        let left = self.expires_at - chrono::Utc::now().timestamp();
        adding(&accounts.flow, self.cancelled.as_deref(), self.refused.get("start").map(String::as_str), left)
    }

    /// Takes in what Fleet answered of the logins, and counts the code down from it.
    fn took(&mut self, accounts: Accounts) {
        if let AccountFlow::Code { expires_in_seconds, .. } = &accounts.flow {
            self.expires_at = chrono::Utc::now().timestamp() + *expires_in_seconds as i64;
        }
        self.accounts = Some(Ok(accounts));
    }
}

/// What the dashed card draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Adding {
    /// Add a GitHub login.
    Add,
    /// A code waits to be typed at `address`, with `left` seconds.
    Waiting { code: String, address: String, left: i64 },
    /// The code was typed and Fleet sets the login up.
    Setup { login: String },
    /// The last sign-in ended without one, or Fleet would not begin one: its sentence, and Try again.
    Ended { said: String },
}

/// The dashed card's state, from Fleet's flow, the code put away with Cancel, and a start Fleet refused.
pub fn adding(flow: &AccountFlow, cancelled: Option<&str>, refused: Option<&str>, left: i64) -> Adding {
    if let Some(said) = refused {
        return Adding::Ended { said: said.to_string() };
    }
    match flow {
        AccountFlow::Code { code, address, .. } if cancelled != Some(code.as_str()) => Adding::Waiting { code: code.clone(), address: address.clone(), left: left.max(0) },
        AccountFlow::Setup { login, .. } => Adding::Setup { login: login.clone() },
        // A sign-in he put away ends unseen: its code running out is not news.
        AccountFlow::Idle { ended: Some(ended) } if cancelled.is_none() && ended.outcome != SignInOutcome::SignedIn => Adding::Ended { said: ended.said.clone().unwrap_or_else(|| ended_words(ended.outcome).to_string()) },
        _ => Adding::Add,
    }
}

/// Words for an end Fleet sent no sentence with.
fn ended_words(outcome: SignInOutcome) -> &'static str {
    match outcome {
        SignInOutcome::Denied => "Turned down on GitHub.",
        SignInOutcome::Expired => "The code ran out.",
        SignInOutcome::Unreachable => "GitHub did not answer.",
        SignInOutcome::Refused | SignInOutcome::SignedIn => "GitHub said no.",
    }
}

/// One row of What it turns on: a project with a store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnsOn {
    pub project: String,
    pub name: String,
    /// The team a team store admits; none for his own.
    pub team: Option<String>,
    pub joins: Joins,
    pub joined: bool,
}

/// Who joins a store: the login the project picked, or the store's sentence where none can be named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Joins {
    Login(String),
    Said(String),
}

/// A row for each project that names a store, in the projects' order. A login is named where this Mac is signed in
/// to it; else the row carries what the store said.
pub fn turns_on(stores: &[(String, String, ProjectStoreState)], accounts: Option<&Accounts>) -> Vec<TurnsOn> {
    stores
        .iter()
        .filter_map(|(project, name, state)| {
            let store = state.store.as_ref()?;
            let login = store.account_id.and_then(|id| accounts?.profiles.iter().find(|one| one.account_id == id)).map(|one| one.login.clone());
            let joins = match login {
                Some(login) => Joins::Login(login),
                None => Joins::Said(state.macs_unread.clone().unwrap_or_default()),
            };
            Some(TurnsOn { project: project.clone(), name: name.clone(), team: store.team.clone(), joins, joined: state.joined_here })
        })
        .collect()
}

impl SettingsOverlay {
    fn account_shown(&self, cx: &App) -> bool {
        self.open && self.shown(cx).is_some_and(|one| one.id == "fleet:account")
    }

    /// Ask Fleet for the logins and for each project's store, once each time the page is drawn after it was left.
    pub(super) fn read_account(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.account.asked {
            return;
        }
        self.account.asked = true;
        self.read_accounts(window, cx);
        let projects = self.wire.projects_now(cx);
        self.account._reads.push(cx.spawn_in(window, async move |this, cx| {
            let projects = projects.await.map(|all| all.projects).unwrap_or_default();
            let Ok(reads) = this.update(cx, |this, cx| projects.iter().map(|one| (one.id.clone(), one.name.clone(), this.wire.project_store(&one.id, cx))).collect::<Vec<_>>()) else { return };
            let mut stores = Vec::new();
            for (id, name, read) in reads {
                // A store that cannot be read is no row: the project's own page says why.
                if let Ok(state) = read.await {
                    stores.push((id, name, state));
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.account.stores = Some(stores);
                cx.notify();
            });
        }));
    }

    /// `GET /accounts`, and again each other second while a code waits or Fleet sets a login up.
    pub(super) fn read_accounts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let read = self.wire.accounts(cx);
        self.account._reads.push(cx.spawn_in(window, async move |this, cx| {
            let answer = read.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match answer {
                    Ok(accounts) => this.account.took(accounts),
                    Err(refused) => this.account.accounts = Some(Err(refused.said)),
                }
                this.watch_flow(window, cx);
                cx.notify();
            });
        }));
    }

    /// While a sign-in is in flight, the time left is drawn each second and Fleet is asked each other second.
    fn watch_flow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let flying = matches!(self.account.held().map(|one| &one.flow), Some(AccountFlow::Code { .. } | AccountFlow::Setup { .. }));
        if !flying {
            self.account._tick = None;
            return;
        }
        if self.account._tick.is_some() {
            return;
        }
        self.account._tick = Some(cx.spawn_in(window, async move |this, cx| {
            let mut ticks: u32 = 0;
            loop {
                cx.background_executor().timer(TICK).await;
                ticks += 1;
                let going = this
                    .update_in(cx, |this, window, cx| {
                        if !this.account_shown(cx) {
                            return false;
                        }
                        if ticks % 2 == 0 {
                            this.read_accounts(window, cx);
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !going {
                    break;
                }
            }
        }));
    }

    // ---- keys -----------------------------------------------------------------------------------

    pub(super) fn account_keys(&mut self, cx: &mut Context<Self>) {
        let mut names: Vec<String> = ["add", "open-address", "copy-code", "cancel", "again"].into_iter().map(String::from).collect();
        if let Some(accounts) = self.account.held() {
            names.extend(accounts.profiles.iter().map(|one| format!("sign-out:{}", one.account_id)));
        }
        if let Some(stores) = &self.account.stores {
            names.extend(stores.iter().map(|(id, _, _)| format!("store:{id}")));
        }
        for name in names {
            self.account.keys.entry(name).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
    }

    /// The page's stops, in the order it draws them: the logins, the sign-in app while none is registered, the
    /// devices, then each store's row.
    pub(super) fn account_stops(&self, cx: &App) -> Vec<FocusHandle> {
        let page = &self.account;
        let key = |name: &str| page.keys.get(name).cloned();
        let mut stops = Vec::new();
        if let Some(accounts) = page.held() {
            stops.extend(accounts.profiles.iter().filter_map(|one| key(&format!("sign-out:{}", one.account_id))));
        }
        if self.sign_in_app_owed() {
            stops.extend(self.sign_in_app_stops());
            stops.extend(self.section_stops("account", cx));
        } else if page.held().is_some() {
            match page.adding() {
                Adding::Add => stops.extend(key("add")),
                Adding::Waiting { .. } => stops.extend(["open-address", "copy-code", "cancel"].into_iter().filter_map(key)),
                Adding::Setup { .. } => {}
                Adding::Ended { .. } => stops.extend(key("again")),
            }
        }
        stops.extend(self.macs_stops());
        let rows = page.stores.as_deref().map(|stores| turns_on(stores, page.held())).unwrap_or_default();
        stops.extend(rows.iter().filter_map(|row| key(&format!("store:{}", row.project))));
        stops
    }

    /// Enter and Space press the control the keys are on.
    pub(super) fn account_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt || !matches!(key.key.as_str(), "enter" | "space") {
            return false;
        }
        let Some(name) = self.account.keys.iter().find(|(_, focus)| focus.is_focused(window)).map(|(name, _)| name.clone()) else { return false };
        self.press_account(&name, window, cx);
        true
    }

    pub(super) fn press_account(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        match name {
            "add" | "again" => self.start_sign_in(window, cx),
            "cancel" => self.cancel_sign_in(window, cx),
            "open-address" => {
                if let Adding::Waiting { address, .. } = self.account.adding() {
                    self.wire.open_link(&address, cx);
                }
            }
            "copy-code" => self.copy_code(window, cx),
            _ => {
                if let Some(id) = name.strip_prefix("sign-out:").and_then(|id| id.parse::<u64>().ok()) {
                    self.sign_out(id, window, cx);
                } else if let Some(project) = name.strip_prefix("store:") {
                    self.open_store(project.to_string(), window, cx);
                }
            }
        }
    }

    fn focus_account(&self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(to) = self.account.keys.get(name).cloned() {
            window.defer(cx, move |window, cx| window.focus(&to, cx));
        }
    }

    // ---- acts -----------------------------------------------------------------------------------

    /// `POST /accounts/start_sign_in`: the code to type comes back, and the card shows it.
    fn start_sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.account.out.is_some() || !self.live {
            return;
        }
        let page = &mut self.account;
        page.out = Some(Out::Start);
        page.refused.remove("start");
        page.cancelled = None;
        let press = self.wire.start_sign_in(cx);
        self.account._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.account.out = None;
                match answer {
                    Ok(accounts) => {
                        this.account.took(accounts);
                        this.watch_flow(window, cx);
                        this.focus_account("open-address", window, cx);
                    }
                    Err(refused) => {
                        this.account.refused.insert("start".into(), refused.said);
                        this.focus_account("again", window, cx);
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// The code is put away and Add is drawn again. Fleet is not told: it has no act for it.
    fn cancel_sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Adding::Waiting { code, .. } = self.account.adding() {
            self.account.cancelled = Some(code);
            self.focus_account("add", window, cx);
            cx.notify();
        }
    }

    fn copy_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Adding::Waiting { code, .. } = self.account.adding() else { return };
        self.wire.copy(&code, cx);
        self.account.copied = true;
        self.account._copied = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.account.copied = false;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// `POST /accounts/sign_out`: the login leaves this Mac, and its card with it.
    fn sign_out(&mut self, account_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.account.out.is_some() || !self.live {
            return;
        }
        let key = format!("sign-out:{account_id}");
        self.account.out = Some(Out::SignOut(account_id));
        self.account.refused.remove(&key);
        self.account.signed_out_said = None;
        let press = self.wire.sign_out(account_id, cx);
        self.account._act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.account.out = None;
                match answer {
                    Ok(signed) => {
                        this.account.signed_out_said = signed.said;
                        this.account.took(signed.accounts);
                        this.focus_account("add", window, cx);
                    }
                    Err(refused) => {
                        this.account.refused.insert(key, refused.said);
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Settings › Claims store, with the project picked.
    fn open_store(&mut self, project: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.account.stores.as_ref().and_then(|stores| stores.iter().find(|(id, _, _)| *id == project)).map(|(_, name, _)| name.clone()) else { return };
        if !self.query_text(cx).is_empty() {
            self.query.update(cx, |query, cx| query.set_value("", window, cx));
        }
        self.edit(Some((project, name)), window, cx);
        self.picked = "project:claims-store";
        window.focus(&self.side, cx);
        cx.notify();
    }

    // ---- drawn ----------------------------------------------------------------------------------

    pub(super) fn render_account(&self, entry: &'static Entry, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        // The band names this Mac, as Classic's header does.
        let this_mac = match &self.macs_list() {
            Some(macs) => macs.iter().find(|one| one.this_mac).map(|one| one.name.clone()).unwrap_or_default(),
            None => String::new(),
        };
        let mut sections = vec![self.who_section(look, window, cx), section(look, "account-devices", "cpu", look.color(token::STATUS_RUNNING), "Your devices", self.render_macs(look, window, cx))];
        sections.extend(self.turns_on_section(look, window, cx));
        self.rows_page(entry, this_mac.into(), false, Vec::new(), sections, look, window, cx)
    }

    fn who_section(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let page = &self.account;
        let mut body: Vec<AnyElement> = Vec::new();
        match &page.accounts {
            None => body.push(cards_row(look, (0..2).map(|_| login_bone(look)).collect())),
            Some(Err(said)) => body.push(refused_line(look, "account-read-refused", said)),
            Some(Ok(accounts)) => {
                let mut cards: Vec<AnyElement> = accounts.profiles.iter().map(|one| self.login_card(one, look, window, cx)).collect();
                // A sign-in needs the sign-in app: until one is registered its press stands in the dashed card's place.
                if !self.sign_in_app_owed() {
                    cards.push(self.adding_card(look, window, cx));
                }
                body.push(cards_row(look, cards));
            }
        }
        if let Some(said) = &page.signed_out_said {
            body.push(labelled(div().id("account-signed-out-said").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())), Role::Status, said.clone()).into_any_element());
        }
        if self.sign_in_app_owed() {
            let spread = self.spread(look, window, cx);
            let registering = self.sign_in_app_row(spread, look, window, cx);
            let first = registering.is_none();
            let fields = self.list.iter().flat_map(|list| super::entries::settings_in(list, "account")).enumerate().map(|(n, one)| self.field_row(one, spread, first && n == 0, look, window, cx));
            body.push(rows_card(look, "settings-card-account", registering.into_iter().chain(fields).collect()).into_any_element());
        }
        section(look, "account-who", "user", look.color(token::STATUS_COMPLETED_SUCCESS), "Who you are", body)
    }

    fn login_card(&self, profile: &ipc::Profile, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let ink = look.color(token::STATUS_COMPLETED_SUCCESS);
        let key = format!("sign-out:{}", profile.account_id);
        let busy = self.account.out == Some(Out::SignOut(profile.account_id));
        let name = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .child(svg().flex_none().path(icon_path("user")).size(look.length(token::SPACE_4)).text_color(ink))
            .child(labelled(
                div().id(SharedString::from(format!("account-login-name-{}", profile.account_id))).min_w_0().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).tooltip(tip(profile.login.clone())).child(SharedString::from(profile.login.clone())),
                Role::Heading,
                profile.login.clone(),
            ));
        let orgs = (!profile.orgs.is_empty()).then(|| {
            let said = profile.orgs.join(", ");
            div().id(SharedString::from(format!("account-orgs-{}", profile.account_id))).truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip(format!("Organisations: {said}"))).child(SharedString::from(said))
        });
        let chip = state_chip(look, SharedString::from(format!("account-signed-in-{}", profile.account_id)), "check", "Signed in", ink, look.color(token::STATUS_COMPLETED_SUCCESS_BG), "Signed in on this Mac");
        let press = self.account_press(look, &key, "Sign out", None, Weight::Quiet, busy, format!("Sign out {}", profile.login), window, cx);
        let card = login_frame(look, SharedString::from(format!("account-login-{}", profile.account_id)), ink.opacity(0.45), false)
            .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(name).children(orgs))
            .child(div().flex().child(chip))
            .child(div().flex_1())
            .child(div().flex().child(press))
            .children(self.account.refused.get(&key).map(|said| refused_line(look, "account-sign-out-refused", said)));
        labelled(card, Role::Group, profile.login.clone()).into_any_element()
    }

    /// The dashed card: Add a GitHub login, the code while it waits, Fleet setting a login up, or how the last ended.
    fn adding_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let busy = self.account.out == Some(Out::Start);
        match self.account.adding() {
            Adding::Add => {
                let Some(focus) = self.account.keys.get("add").cloned() else { return div().into_any_element() };
                let ink = look.color(token::FG_MUTED);
                let mark = if busy { spinner(look, "account-add-busy", ink) } else { svg().flex_none().path(icon_path("plus")).size(look.length(token::SPACE_4)).text_color(ink).into_any_element() };
                let said = if busy { "Starting" } else { "Add a GitHub login" };
                let card = login_frame(look, "account-add", look.color(token::BORDER_DEFAULT), true)
                    .justify_center()
                    .items_center()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(ink)
                    .when(!busy, |card| card.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(|this, _, window, cx| this.press_account("add", window, cx))))
                    .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(mark).child(said));
                labelled(ringed(look, card, &focus, window).track_focus(&focus), Role::Button, said).into_any_element()
            }
            Adding::Waiting { code, address, left } => {
                let ink = look.color(token::STATUS_AWAITING_APPROVAL);
                let link = self.account_press(look, "open-address", &address.trim_start_matches("https://").to_string(), Some("external-link"), Weight::Link, false, format!("Open {address}"), window, cx);
                let code_line = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(labelled(div().id("account-code").font_family(super::view::mono(look)).text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(code.clone())), Role::Label, format!("Code {code}")))
                    .child(self.code_copy_press(look, window, cx));
                let wait = div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(spinner(look, "account-code-wait", ink))
                    .child(labelled(div().id("account-code-left").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip("Time left on the code")).child(SharedString::from(format!("{} left", super::macs::time_left(left)))), Role::Timer, format!("{} left", super::macs::time_left(left))));
                let card = login_frame(look, "account-adding", ink.opacity(0.6), true)
                    .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Type the code at"))
                    .child(div().flex().child(link))
                    .child(code_line)
                    .child(wait)
                    .child(div().flex_1())
                    .child(div().flex().child(self.account_press(look, "cancel", "Cancel", None, Weight::Quiet, false, "Cancel".into(), window, cx)));
                labelled(card, Role::Group, "Adding a GitHub login").into_any_element()
            }
            Adding::Setup { login } => {
                let ink = look.color(token::STATUS_RUNNING);
                let card = login_frame(look, "account-adding", ink.opacity(0.6), true)
                    .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(svg().flex_none().path(icon_path("user")).size(look.length(token::SPACE_4)).text_color(ink)).child(div().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(login.clone()))))
                    .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(spinner(look, "account-setup-busy", ink)).child(labelled(div().id("account-setup-said").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Setting up"), Role::Status, "Setting up")));
                labelled(card, Role::Group, "Adding a GitHub login").into_any_element()
            }
            Adding::Ended { said } => {
                let ink = look.color(token::STATUS_AWAITING_REVIEW);
                let line = div()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(svg().flex_none().mt(look.length(token::SPACE_1) / 2.).path(icon_path("triangle-alert")).size(look.length(token::SPACE_4)).text_color(ink))
                    .child(labelled(div().id("account-ended-said").flex_1().min_w_0().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said.clone())), Role::Status, said.clone()));
                let card = login_frame(look, "account-adding", ink.opacity(0.6), true)
                    .child(line)
                    .child(div().flex_1())
                    .child(div().flex().child(self.account_press(look, "again", "Try again", Some("rotate-cw"), Weight::Plain, busy, "Try again".into(), window, cx)));
                labelled(card, Role::Group, "Adding a GitHub login").into_any_element()
            }
        }
    }

    /// What it turns on: a row for each project with a store. None while no project names one.
    fn turns_on_section(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let page = &self.account;
        let rows: Vec<AnyElement> = match &page.stores {
            None => (0..2).map(|n| labelled(div().id(SharedString::from(format!("account-store-bone-{n}"))).child(store_row_frame(look).child(bone(look, look.length(token::SPACE_4), look.length(token::SPACE_4))).child(bone(look, look.length(token::SPACE_12) * 4., look.length(token::LEADING_SM)))), Role::Image, "Loading").into_any_element()).collect(),
            Some(stores) => {
                // Kit's row goes first here once Kit's wire says what this Mac is in step with.
                let rows = turns_on(stores, page.held());
                if rows.is_empty() {
                    return None;
                }
                rows.iter().map(|row| self.store_row(row, look, window, cx)).collect()
            }
        };
        let list = labelled(div().id("account-turns-on").flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::List, "Project stores");
        Some(section(look, "account-turns-on-section", "layers", look.color(token::KIND_DELIVERY), "What it turns on", vec![list.into_any_element()]))
    }

    fn store_row(&self, row: &TurnsOn, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let (ink, wash, glyph, said, why) = if row.joined {
            (look.color(token::STATUS_COMPLETED_SUCCESS), look.color(token::STATUS_COMPLETED_SUCCESS_BG), "check", "Joined", "This Mac has joined the store")
        } else {
            (look.color(token::STATUS_AWAITING_REVIEW), look.color(token::STATUS_AWAITING_REVIEW_BG), "triangle-alert", "Not joined", "This Mac has not joined the store")
        };
        let kind = match &row.team {
            Some(team) => format!("Team store {team}"),
            None => "Own store".to_string(),
        };
        let joins = match &row.joins {
            Joins::Login(login) => div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("Joins as {login}"))),
            Joins::Said(said) => div().flex().items_center().gap(look.length(token::SPACE_1)).text_color(look.color(token::STATUS_AWAITING_REVIEW)).child(SharedString::from(said.clone())),
        };
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(look.length(token::SPACE_2))
                    .child(div().flex_none().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.name.clone())))
                    .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(kind))),
            )
            .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).child(joins));
        let chip = state_chip(look, SharedString::from(format!("account-store-joined-{}", row.project)), glyph, said, ink, wash, why);
        let link = self.account_press(look, &format!("store:{}", row.project), "Claims store", Some("database"), Weight::Link, false, format!("Claims store for {}", row.name), window, cx);
        let item = store_row_frame(look)
            .id(SharedString::from(format!("account-store-{}", row.project)))
            .child(svg().flex_none().path(icon_path("server")).size(look.length(token::SPACE_4)).text_color(ink))
            .child(words)
            .child(div().flex_none().w(look.length(token::SPACE_12) * 2.).flex().child(chip))
            .child(div().flex_none().w(look.length(token::SPACE_12) * 2.5).flex().justify_end().child(link));
        labelled(item, Role::ListItem, row.name.clone()).into_any_element()
    }

    fn code_copy_press(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.account.keys.get("copy-code").cloned() else { return div().into_any_element() };
        let copied = self.account.copied;
        let said = if copied { "Copied" } else { "Copy the code" };
        let ink = if copied { look.color(token::STATUS_COMPLETED_SUCCESS) } else { look.color(token::FG_SUBTLE) };
        let button = div()
            .id("account-copy-code")
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_1))
            .h(look.length(token::H_CONTROL_SM))
            .min_w(look.length(token::H_CONTROL_SM))
            .when(copied, |button| button.px(look.length(token::SPACE_2)))
            .rounded(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_XS))
            .text_color(ink)
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .tooltip(tip(said))
            .on_click(cx.listener(|this, _, window, cx| this.press_account("copy-code", window, cx)))
            .child(svg().flex_none().path(icon_path(if copied { "check" } else { "copy" })).size(look.length(token::SPACE_4)).text_color(ink))
            .children(copied.then_some("Copied"));
        labelled(ringed(look, button, &focus, window).track_focus(&focus), Role::Button, said).into_any_element()
    }

    /// One press, busy with a turning mark while its act is out. `heard` is what VoiceOver reads.
    #[allow(clippy::too_many_arguments)]
    fn account_press(&self, look: &Look, key: &str, said: &str, glyph: Option<&'static str>, weight: Weight, busy: bool, heard: String, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.account.keys.get(key).cloned() else { return div().into_any_element() };
        let off = busy || !self.live || (self.account.out.is_some() && weight != Weight::Link);
        let (ink, fill, edge) = match weight {
            Weight::Plain => (look.color(token::FG_DEFAULT), look.color(token::BG_OVERLAY), look.color(token::BORDER_STRONG)),
            Weight::Quiet => (look.color(token::FG_MUTED), gpui::transparent_black(), look.color(token::BORDER_DEFAULT)),
            Weight::Link => (look.color(token::ACCENT), gpui::transparent_black(), gpui::transparent_black()),
        };
        let mark = if busy { Some(spinner(look, SharedString::from(format!("account-busy-{key}")), ink)) } else { glyph.map(|glyph| svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(ink).into_any_element()) };
        let name = key.to_string();
        let button = div()
            .id(SharedString::from(format!("account-{key}")))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(if weight == Weight::Link { look.length(token::H_CONTROL_SM) } else { look.length(token::H_CONTROL) })
            .px(if weight == Weight::Link { look.length(token::SPACE_1) } else { look.length(token::SPACE_3) })
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(fill)
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(ink)
            .when(off && !busy, |button| button.opacity(0.5))
            .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| this.press_account(&name, window, cx))))
            .children(mark)
            .child(SharedString::from(said.to_string()));
        let role = if weight == Weight::Link { Role::Link } else { Role::Button };
        labelled(ringed(look, button, &focus, window).track_focus(&focus), role, heard).into_any_element()
    }

    fn macs_list(&self) -> Option<&Vec<ipc::Mac>> {
        self.macs.list()
    }
}

/// How a press is weighted: a plain press, the quiet Sign out and Cancel, or a link in the accent.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Weight {
    Plain,
    Quiet,
    Link,
}

/// A section: its glyph in its hue and its name, over its body.
fn section(look: &Look, id: &'static str, glyph: &'static str, hue: Hsla, said: &'static str, body: Vec<AnyElement>) -> AnyElement {
    let head = div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .child(div().flex_none().size(look.length(token::SPACE_6)).rounded(look.length(token::RADIUS_MD)).bg(hue.opacity(0.2)).flex().items_center().justify_center().child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(hue)))
        .child(labelled(div().id(SharedString::from(format!("{id}-head"))).text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(said), Role::Heading, said));
    labelled(div().id(id).flex().flex_col().gap(look.length(token::SPACE_3)).child(head).children(body), Role::Region, said).into_any_element()
}

/// The login cards side by side, wrapping where the page is narrow.
fn cards_row(look: &Look, cards: Vec<AnyElement>) -> AnyElement {
    div().flex().flex_wrap().items_stretch().gap(look.length(token::SPACE_3)).children(cards).into_any_element()
}

/// A login card's frame, every card the same width so each Sign out sits in the same place.
fn login_frame(look: &Look, id: impl Into<gpui::ElementId>, edge: Hsla, dashed: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id.into())
        .flex_none()
        .w(look.length(token::SPACE_12) * 4.75)
        .min_h(look.length(token::SPACE_12) * 3.)
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .p(look.length(token::SPACE_4))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge)
        .when(dashed, |card| card.border_dashed())
        .bg(look.color(token::SURFACE_CARD))
}

/// A card's place while Fleet has not answered.
fn login_bone(look: &Look) -> AnyElement {
    let card = login_frame(look, SharedString::from("account-login-bone"), look.color(token::BORDER_SUBTLE), false)
        .child(bone(look, look.length(token::SPACE_12) * 2., look.length(token::LEADING_SM)))
        .child(bone(look, look.length(token::SPACE_12) * 1.5, look.length(token::H_BADGE)));
    labelled(card, Role::Image, "Loading").into_any_element()
}

fn store_row_frame(look: &Look) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::SURFACE_ROW))
}

/// A state as a chip: a registry glyph and a word in the state's ink on its wash, with a tooltip saying what it is.
fn state_chip(look: &Look, id: SharedString, glyph: &'static str, said: &'static str, ink: Hsla, wash: Hsla, why: &'static str) -> AnyElement {
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
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(ink)
        .tooltip(tip(why))
        .child(svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(ink))
        .child(said);
    labelled(chip, Role::Status, said).into_any_element()
}

fn spinner(look: &Look, id: impl Into<SharedString>, ink: Hsla) -> AnyElement {
    svg()
        .flex_none()
        .path(icon_path("loader-circle"))
        .size(look.length(token::SPACE_4))
        .text_color(ink)
        .with_animation(id.into(), Animation::new(look.duration(token::DURATION_SPIN)).repeat(), |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext, VisualTestContext};
    use ipc::{ProjectStore, SignInEnded, StoreDeploy, StoreJoin};

    use super::super::mock::MockFleet;
    use super::super::mock_account::{AccountCall, MockAccounts, ADDRESS, CODE, EXPIRED_SAID, PERSONAL, WORK};
    use super::super::mock_store::NO_LOGIN;
    use super::super::wire::Wire;
    use super::*;

    fn code() -> AccountFlow {
        AccountFlow::Code { code: CODE.into(), address: ADDRESS.into(), expires_in_seconds: 899 }
    }

    fn ended(outcome: SignInOutcome, said: Option<&str>) -> AccountFlow {
        AccountFlow::Idle { ended: Some(SignInEnded { outcome, said: said.map(String::from) }) }
    }

    #[test]
    fn the_dashed_card_shows_the_code_until_it_is_put_away() {
        assert_eq!(adding(&code(), None, None, 300), Adding::Waiting { code: CODE.into(), address: ADDRESS.into(), left: 300 });
        assert_eq!(adding(&code(), Some(CODE), None, 300), Adding::Add, "Cancel puts it away");
        assert_eq!(adding(&code(), None, None, -4), Adding::Waiting { code: CODE.into(), address: ADDRESS.into(), left: 0 }, "time left stops at none");
        assert_eq!(adding(&AccountFlow::Idle { ended: None }, None, None, 0), Adding::Add);
    }

    #[test]
    fn an_ended_sign_in_says_fleets_sentence_and_one_put_away_says_nothing() {
        assert_eq!(adding(&ended(SignInOutcome::Expired, Some(EXPIRED_SAID)), None, None, 0), Adding::Ended { said: EXPIRED_SAID.into() });
        assert_eq!(adding(&ended(SignInOutcome::Denied, None), None, None, 0), Adding::Ended { said: "Turned down on GitHub.".into() }, "a sentence where Fleet sent none");
        assert_eq!(adding(&ended(SignInOutcome::SignedIn, None), None, None, 0), Adding::Add, "one that signed in is its card");
        assert_eq!(adding(&ended(SignInOutcome::Expired, Some(EXPIRED_SAID)), Some(CODE), None, 0), Adding::Add, "a code he put away running out is not news");
        assert_eq!(adding(&AccountFlow::Idle { ended: None }, None, Some("GitHub could not be reached."), 0), Adding::Ended { said: "GitHub could not be reached.".into() }, "a start Fleet refused");
        assert_eq!(adding(&AccountFlow::Setup { login: "nickmele".into(), repository: String::new(), steps: ipc::SetUpSteps { identity: ipc::SetUpState::Done, store: ipc::SetUpState::Working, home: ipc::SetUpState::Waiting } }, None, None, 0), Adding::Setup { login: "nickmele".into() });
    }

    fn state(project: &str, store: Option<ProjectStore>, joined: bool, unread: Option<&str>) -> ProjectStoreState {
        ProjectStoreState { project: project.into(), store, macs: Vec::new(), macs_unread: unread.map(String::from), joined_here: joined, deploy: StoreDeploy::Ready, join: StoreJoin::Ready, token_page: String::new() }
    }

    fn store(team: Option<&str>, account_id: Option<u64>) -> ProjectStore {
        ProjectStore { address: "https://store.example".into(), team: team.map(String::from), worker: None, deployed: None, joins_unlimited: false, account_id }
    }

    #[test]
    fn what_it_turns_on_is_a_row_per_project_with_a_store_naming_its_login_or_the_stores_sentence() {
        let accounts = MockAccounts::new().accounts;
        let stores = vec![
            ("armada".to_string(), "Armada".to_string(), state("armada", Some(store(None, Some(PERSONAL))), true, None)),
            ("storefront".to_string(), "Storefront".to_string(), state("storefront", None, false, None)),
            ("ledger".to_string(), "Ledger".to_string(), state("ledger", Some(store(Some("acme"), None)), false, Some(NO_LOGIN))),
            ("pricing".to_string(), "Pricing".to_string(), state("pricing", Some(store(Some("acme"), Some(WORK))), false, None)),
        ];
        let rows = turns_on(&stores, Some(&accounts));
        assert_eq!(rows.iter().map(|row| row.project.as_str()).collect::<Vec<_>>(), ["armada", "ledger", "pricing"], "a project with no store has no row");
        assert_eq!(rows[0].joins, Joins::Login("nickmele".into()));
        assert!(rows[0].joined && rows[0].team.is_none());
        assert_eq!(rows[1].joins, Joins::Said(NO_LOGIN.into()), "none picked: the store's sentence");
        assert_eq!(rows[2].joins, Joins::Login("nmele-work".into()));
        assert_eq!(rows[2].team.as_deref(), Some("acme"));
        let signed_out = turns_on(&stores, None);
        assert_eq!(signed_out[2].joins, Joins::Said(String::new()), "a login this Mac is not signed in to is not named");
    }

    fn opened<'a>(cx: &'a mut TestAppContext, with: impl FnOnce(&mut MockFleet)) -> (Entity<SettingsOverlay>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let mut fleet = MockFleet::new();
        fleet.sign_in_app_registered();
        with(&mut fleet);
        let list = fleet.list.clone();
        let (overlay, cx) = cx.add_window_view(|window, cx| SettingsOverlay::new(Wire::Mock(Box::new(fleet)), window, cx));
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                this.open = true;
                this.live = true;
                this.picked = super::super::entries::ACCOUNT;
                this.set_list(list, window, cx);
                this.read_account(window, cx);
            });
        });
        cx.run_until_parked();
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.account_keys(cx)));
        (overlay, cx)
    }

    fn press(overlay: &Entity<SettingsOverlay>, name: &str, cx: &mut VisualTestContext) {
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.press_account(name, window, cx)));
        cx.run_until_parked();
    }

    fn calls(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Vec<AccountCall> {
        overlay.read_with(cx, |this, _| match &this.wire {
            Wire::Mock(mock) => mock.accounts.calls.clone(),
            Wire::Live(_) => Vec::new(),
        })
    }

    fn on_page(overlay: &Entity<SettingsOverlay>, role: Role, name: &str, cx: &mut VisualTestContext) -> bool {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = overlay;
            // The overlay is the test window's root, so no label root clears the book between frames: a fresh one
            // holds only what this frame drew.
            cx.set_global(crate::labels::LabelBook::default());
            window.draw(cx).clear(cx);
            cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some())
        })
    }

    #[gpui::test]
    fn the_page_draws_a_card_per_login_add_and_a_row_per_store(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |_| {});
        assert!(on_page(&overlay, Role::Button, "Sign out nickmele", cx));
        assert!(on_page(&overlay, Role::Button, "Sign out nmele-work", cx));
        assert!(on_page(&overlay, Role::Button, "Add a GitHub login", cx));
        assert!(on_page(&overlay, Role::Region, "Your devices", cx));
        assert!(on_page(&overlay, Role::Link, "Claims store for Armada", cx));
        assert!(on_page(&overlay, Role::Link, "Claims store for Ledger", cx));
        assert!(!on_page(&overlay, Role::Link, "Claims store for Storefront", cx), "Storefront names no store");
        assert!(!on_page(&overlay, Role::Button, "Register on GitHub", cx), "the sign-in app is registered");
    }

    #[gpui::test]
    fn with_no_sign_in_app_its_press_stands_where_add_would(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |mock| {
            if let Some(one) = mock.list.settings.iter_mut().find(|one| one.key == "account.clientId") {
                one.value = ipc::SettingValue::Text(String::new());
            }
        });
        assert!(on_page(&overlay, Role::Button, "Register on GitHub", cx));
        assert!(!on_page(&overlay, Role::Button, "Add a GitHub login", cx));
        assert!(on_page(&overlay, Role::Button, "Sign out nickmele", cx), "the logins are drawn all the same");
    }

    #[gpui::test]
    fn add_shows_the_code_and_cancel_puts_it_away_without_telling_fleet(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |_| {});
        press(&overlay, "add", cx);
        assert_eq!(calls(&overlay, cx), [AccountCall::StartSignIn]);
        assert!(on_page(&overlay, Role::Label, &format!("Code {CODE}"), cx));
        assert!(on_page(&overlay, Role::Link, &format!("Open {ADDRESS}"), cx));
        press(&overlay, "copy-code", cx);
        assert!(overlay.read_with(cx, |this, _| matches!(&this.wire, Wire::Mock(mock) if mock.handed.last().map(String::as_str) == Some(CODE))), "the code goes on the clipboard");
        press(&overlay, "cancel", cx);
        assert!(on_page(&overlay, Role::Button, "Add a GitHub login", cx));
        assert_eq!(calls(&overlay, cx), [AccountCall::StartSignIn], "Cancel sends nothing");
        press(&overlay, "add", cx);
        assert!(on_page(&overlay, Role::Label, &format!("Code {CODE}"), cx), "Add again shows a code again");
    }

    #[gpui::test]
    fn an_ended_sign_in_says_why_and_try_again_starts_one(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |mock| mock.accounts.ended());
        assert!(on_page(&overlay, Role::Status, EXPIRED_SAID, cx));
        press(&overlay, "again", cx);
        assert_eq!(calls(&overlay, cx), [AccountCall::StartSignIn]);
        assert!(on_page(&overlay, Role::Label, &format!("Code {CODE}"), cx));
    }

    #[gpui::test]
    fn a_refused_start_says_fleets_sentence_in_the_card(cx: &mut TestAppContext) {
        let said = "GitHub would not begin a sign-in: the app has the device flow off.";
        let (overlay, cx) = opened(cx, |mock| mock.accounts.start_refusal = Some(said.into()));
        press(&overlay, "add", cx);
        assert!(on_page(&overlay, Role::Status, said, cx));
        assert!(on_page(&overlay, Role::Button, "Try again", cx));
    }

    #[gpui::test]
    fn sign_out_takes_the_login_off(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |_| {});
        press(&overlay, &format!("sign-out:{WORK}"), cx);
        assert_eq!(calls(&overlay, cx), [AccountCall::SignOut(WORK)]);
        assert!(!on_page(&overlay, Role::Button, "Sign out nmele-work", cx));
        assert!(on_page(&overlay, Role::Button, "Sign out nickmele", cx));
    }

    #[gpui::test]
    fn a_stores_row_opens_its_projects_claims_store(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |_| {});
        press(&overlay, "store:ledger", cx);
        overlay.read_with(cx, |this, _| {
            assert_eq!(this.picked, "project:claims-store");
            assert_eq!(this.scope.project, Some(("ledger".into(), "Ledger".into())));
        });
    }

    #[gpui::test]
    fn the_keys_reach_every_press_in_the_order_drawn(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx, |_| {});
        overlay.read_with(cx, |this, cx| {
            let stops = this.account_stops(cx);
            let at = |name: &str| this.account.keys.get(name).and_then(|key| stops.iter().position(|stop| stop == key));
            let (first, second, add, ledger) = (at(&format!("sign-out:{PERSONAL}")), at(&format!("sign-out:{WORK}")), at("add"), at("store:ledger"));
            assert!(first < second && second < add && add < ledger, "{first:?} {second:?} {add:?} {ledger:?}");
            assert!(at("cancel").is_none(), "no code waits");
        });
    }

    #[gpui::test]
    fn the_title_bars_account_row_opens_settings_on_this_page(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let under = cx.new(|_| gpui::Empty);
            crate::settings::SettingsScreen::mock(under.into(), window, cx)
        });
        let overlay = screen.read_with(cx, |screen, _| screen.overlay.clone());
        // Settings was last left on another page: the row opens it on Account all the same.
        overlay.update(cx, |this, _| this.picked = "bridge:theme");
        cx.update(|_, cx| {
            let sink = cx.try_global::<crate::titlebar::AskedSink>().cloned().expect("Settings listens for what the bar asks");
            (sink.0)(&crate::titlebar::Asked::Account, cx);
        });
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            assert!(this.open, "Settings opened");
            assert_eq!(this.picked, super::super::entries::ACCOUNT);
            assert_eq!(this.scope.project, None, "the page is this Mac's, for no project");
        });
    }
}
