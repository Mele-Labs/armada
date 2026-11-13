//! Settings › Macs: this Mac, its Cloudflare tunnel, and every Mac and phone that reaches it, read off the Phone
//! Gateway (`gateway.rs`). The tunnel's rows are drawn as Classic's Funnel rows are (`PhonePairing`'s `WayIn`).
//!
//! **He opens it cold, with no walk** (the owner, 12 Nov). Each state of the tunnel shows the one press that moves
//! it on, and why where there is a why; nothing tells him to go and do something outside Bridge, and nothing asks
//! him to type. Cloudflare's sign-in page opens only when he presses for it.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{div, prelude::*, svg, AnyElement, App, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla, Keystroke, Role, SharedString, Subscription, Task, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{CloudflareState, GatewaySetup, Mac, MacColour, MacLink, MacPairLink, PairedPhone, Served};

use super::entries::Body;
use super::gateway::Tunnel;
use super::overlay::SettingsOverlay;
use super::view::{bone, card, ghost, mono, ringed, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The setup is read again this often until the tunnel runs (`RETRY_EVERY`).
const RETRY: Duration = Duration::from_secs(5);

/// The page's own state: what the Gateway answered, the press out and what it said, and the key of each control.
pub struct MacsPage {
    setup: Option<Result<GatewaySetup, String>>,
    macs: Option<Result<Vec<Mac>, String>>,
    phones: Option<Result<Vec<PairedPhone>, String>>,
    asked: bool,
    pressing: bool,
    /// The press's refusal, said under its button.
    said: Option<String>,
    unpair_said: Option<String>,
    /// The link Pair a Mac made, until it runs out.
    link: Option<MacPairLink>,
    /// Where the other Mac's link is pasted.
    paste: Entity<InputState>,
    paste_focus: FocusHandle,
    /// A pairing act's refusal, said under it, by the act's key: `pair`, `claim`, `confirm:<id>` or `forget:<id>`.
    refused: BTreeMap<String, String>,
    /// The pairing act out; another waits for its answer.
    out: Option<String>,
    keys: BTreeMap<String, FocusHandle>,
    _poll: Option<Task<()>>,
    _reads: Vec<Task<()>>,
    /// The link's time left, drawn again each second while it is good.
    _tick: Option<Task<()>>,
    _paste: Subscription,
}

/// Where the keys wait while the control they were on goes: the Devices list itself.
const AT_REST: &str = "devices";

/// The link's time left is drawn again this often, as Classic's `PhonePairing` counts its code down.
const TICK: Duration = Duration::from_secs(1);

impl MacsPage {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsOverlay>) -> MacsPage {
        let paste = cx.new(|cx| InputState::new(window, cx).placeholder("Paste the link from your other Mac"));
        let paste_focus = paste.read(cx).focus_handle(cx);
        // Enter in the box presses Claim.
        let sub = cx.subscribe_in(&paste, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.claim(window, cx);
            }
        });
        MacsPage {
            setup: None,
            macs: None,
            phones: None,
            asked: false,
            pressing: false,
            said: None,
            unpair_said: None,
            link: None,
            paste,
            paste_focus,
            refused: BTreeMap::new(),
            out: None,
            keys: BTreeMap::new(),
            _poll: None,
            _reads: Vec::new(),
            _tick: None,
            _paste: sub,
        }
    }

    /// Settings opened again: the Gateway is asked again the next time the page is drawn.
    pub fn reopen(&mut self) {
        self.asked = false;
        self.said = None;
        self.unpair_said = None;
        self.refused.clear();
    }

    pub fn paste_focused(&self, window: &Window) -> bool {
        self.paste_focus.is_focused(window)
    }

    fn cloudflare(&self) -> Option<&CloudflareState> {
        self.setup.as_ref().and_then(|setup| setup.as_ref().ok()).map(|setup| &setup.cloudflare)
    }

    /// The Funnel address phones reach this Mac at, while the tunnel does not run yet.
    fn funnel_row(&self) -> Option<&str> {
        let setup = self.setup.as_ref()?.as_ref().ok()?;
        funnel_of(setup).filter(|_| !matches!(setup.cloudflare, CloudflareState::Running { .. } | CloudflareState::Down { .. }))
    }
}

/// The address Tailscale Funnel serves, while the Gateway's address is still its own. Once the tunnel names the
/// address (`named`), the Funnel address is no longer on the wire.
fn funnel_of(setup: &GatewaySetup) -> Option<&str> {
    (setup.served == Served::Funnel && !setup.named).then_some(setup.address.as_deref()).flatten()
}

/// Whether Set up tunnel is the press that moves the state on. A tunnel that dropped signs in again the same way.
fn asks_setup(state: &CloudflareState) -> bool {
    matches!(state, CloudflareState::Missing | CloudflareState::SignedOut | CloudflareState::NoTunnel { .. } | CloudflareState::Down { .. })
}

/// The key of the one press a state draws.
fn press_of(state: &CloudflareState) -> Option<&'static str> {
    match state {
        _ if asks_setup(state) => Some("setup"),
        CloudflareState::SigningIn { .. } => Some("sign-in"),
        CloudflareState::Running { .. } => Some("copy"),
        _ => None,
    }
}

/// A Unix time as a stamp in his clock, as his time format says: the day comes with it when it is not today.
fn clock_of(at: i64) -> String {
    use chrono::{Local, TimeZone};
    Local.timestamp_opt(at, 0).single().map(|at| super::time::stamp_of(at, super::time::held(), Local::now())).unwrap_or_default()
}

/// Time left on a link, as Classic's `timeLeft` writes it: `4:32`.
fn time_left(seconds: i64) -> String {
    let whole = seconds.max(0);
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// The press a Mac's row carries, by its key's prefix and its words: Confirm on a claim waiting here, Forget on a
/// paired Mac or one waiting for Confirm there, and none on this Mac.
fn press_of_mac(mac: &Mac) -> Option<(&'static str, &'static str)> {
    match mac.link.as_ref()? {
        MacLink::ClaimWaiting => Some(("confirm", "Confirm")),
        MacLink::Linked | MacLink::Asleep { .. } | MacLink::OlderBuild | MacLink::NewerBuild | MacLink::ConfirmWaiting => Some(("forget", "Forget")),
    }
}

fn age_of(at: i64) -> String {
    format!("{} ago", crate::merge_strip::words::span_said((chrono::Utc::now().timestamp() - at).max(0)))
}

pub(crate) fn colour_of(look: &Look, colour: MacColour) -> Hsla {
    look.color(match colour {
        MacColour::Blue => token::SKETCH_BLUE,
        MacColour::Teal => token::SKETCH_TEAL,
        MacColour::Green => token::SKETCH_GREEN,
        MacColour::Orange => token::SKETCH_ORANGE,
    })
}

/// How a link stands, as its glyph and its tooltip.
fn link_said(link: &MacLink, why: Option<&str>) -> (&'static str, String) {
    match link {
        MacLink::Linked => ("link", "Linked".into()),
        MacLink::Asleep { since } => ("moon", format!("Asleep since {}", clock_of(*since))),
        MacLink::OlderBuild => ("arrow-down-to-line", "Runs an older build. Linked again once it updates.".into()),
        MacLink::NewerBuild => ("arrow-up-from-line", "Runs a newer build. Linked again once this Mac updates.".into()),
        MacLink::ClaimWaiting => ("hourglass", "Waiting for Confirm on this Mac".into()),
        // The Gateway's sentence names the Mac to press Confirm on.
        MacLink::ConfirmWaiting => ("hourglass", why.unwrap_or("Waiting for Confirm on the other Mac").into()),
    }
}

/// How a link stands, as its glyph, its ink and its tooltip.
fn link_mark(look: &Look, link: &MacLink, why: Option<&str>) -> (&'static str, Hsla, String) {
    let (glyph, said) = link_said(link, why);
    let ink = match link {
        MacLink::Linked => look.color(token::STATUS_COMPLETED_SUCCESS),
        MacLink::Asleep { .. } => look.color(token::FG_MUTED),
        MacLink::OlderBuild | MacLink::NewerBuild | MacLink::ClaimWaiting | MacLink::ConfirmWaiting => look.color(token::STATUS_ESCALATED),
    };
    (glyph, ink, said)
}

impl SettingsOverlay {
    fn macs_shown(&self, cx: &App) -> bool {
        self.open && self.shown(cx).is_some_and(|one| one.body == Body::Macs)
    }

    /// Ask the Gateway once each time Settings opens, while the page is drawn.
    pub(super) fn read_macs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.macs.asked {
            return;
        }
        self.macs.asked = true;
        self.read_setup(window, cx);
        self.read_devices(cx);
    }

    fn read_devices(&mut self, cx: &mut Context<Self>) {
        let (macs, phones) = (self.wire.gateway_macs(cx), self.wire.gateway_phones(cx));
        self.macs._reads = vec![
            cx.spawn(async move |this, cx| {
                let answer = macs.await;
                let _ = this.update(cx, |this, cx| {
                    this.macs.macs = Some(answer);
                    cx.notify();
                });
            }),
            cx.spawn(async move |this, cx| {
                let answer = phones.await;
                let _ = this.update(cx, |this, cx| {
                    this.macs.phones = Some(answer);
                    cx.notify();
                });
            }),
        ];
    }

    /// `GET /admin/setup`, and again every five seconds while the page is shown and the tunnel does not run. A
    /// state that no longer draws the control the keys were on puts them on the new state's press.
    fn read_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let read = self.wire.gateway_setup(cx);
        self.macs._poll = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = read.await;
            let again = this
                .update_in(cx, |this, window, cx| {
                    let runs = matches!(answer, Ok(GatewaySetup { cloudflare: CloudflareState::Running { .. }, .. }));
                    let held = ["setup", "sign-in", "copy"].into_iter().filter_map(|name| this.macs.keys.get(name)).any(|focus| focus.is_focused(window));
                    this.macs.setup = Some(answer);
                    if held && !this.macs_stops().iter().any(|focus| focus.is_focused(window)) {
                        let press = this.macs.cloudflare().and_then(|state| this.macs.keys.get(press_of(state)?)).or_else(|| this.macs.keys.get(AT_REST)).cloned();
                        if let Some(press) = press {
                            window.focus(&press, cx);
                        }
                    }
                    cx.notify();
                    !runs
                })
                .unwrap_or(false);
            if !again {
                return;
            }
            cx.background_executor().timer(RETRY).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.macs_shown(cx) {
                    this.read_setup(window, cx);
                }
            });
        }));
    }

    /// Keys for each control, made before they are drawn.
    pub(super) fn macs_keys(&mut self, cx: &mut Context<Self>) {
        let mut names: Vec<String> = ["setup", "sign-in", "copy", "pair", "copy-link", "claim"].into_iter().map(String::from).collect();
        if let Some(Ok(macs)) = &self.macs.macs {
            names.extend(macs.iter().filter_map(|mac| press_of_mac(mac).map(|(act, _)| format!("{act}:{}", mac.id))));
        }
        if let Some(Ok(phones)) = &self.macs.phones {
            names.extend(phones.iter().map(|one| format!("unpair:{}", one.id)));
        }
        for name in names {
            self.macs.keys.entry(name).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
        self.macs.keys.entry(AT_REST.into()).or_insert_with(|| cx.focus_handle());
    }

    /// The page's stops, in the order it draws them.
    pub(super) fn macs_stops(&self) -> Vec<FocusHandle> {
        let page = &self.macs;
        let key = |name: &str| page.keys.get(name).cloned();
        let mut stops = Vec::new();
        if page.funnel_row().is_some() {
            stops.extend(key("copy"));
        }
        stops.extend(page.cloudflare().and_then(press_of).and_then(key));
        stops.extend(key("pair"));
        if page.link.is_some() {
            stops.extend(key("copy-link"));
        }
        stops.push(page.paste_focus.clone());
        stops.extend(key("claim"));
        if let Some(Ok(macs)) = &page.macs {
            stops.extend(macs.iter().filter_map(|mac| press_of_mac(mac).and_then(|(act, _)| key(&format!("{act}:{}", mac.id)))));
        }
        if let Some(Ok(phones)) = &page.phones {
            stops.extend(phones.iter().filter_map(|one| key(&format!("unpair:{}", one.id))));
        }
        stops
    }

    /// Enter and Space press the control the keys are on.
    pub(super) fn macs_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt || !matches!(key.key.as_str(), "enter" | "space") {
            return false;
        }
        let Some(name) = self.macs.keys.iter().find(|(_, focus)| focus.is_focused(window)).map(|(name, _)| name.clone()) else { return false };
        self.press_macs(&name, window, cx);
        true
    }

    fn press_macs(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        match name {
            "setup" => self.set_up_tunnel(window, cx),
            "sign-in" => {
                if let Some(CloudflareState::SigningIn { login }) = self.macs.cloudflare() {
                    let login = login.clone();
                    self.wire.open_link(&login, cx);
                }
            }
            "copy" => {
                let address = match self.macs.cloudflare() {
                    Some(CloudflareState::Running { address }) => Some(address.clone()),
                    _ => self.macs.funnel_row().map(String::from),
                };
                if let Some(address) = address {
                    self.wire.copy(&address, cx);
                }
            }
            "pair" => self.pair(window, cx),
            "copy-link" => {
                if let Some(link) = self.macs.link.as_ref().map(|made| made.link.clone()) {
                    self.wire.copy(&link, cx);
                }
            }
            "claim" => self.claim(window, cx),
            _ => {
                if let Some(id) = name.strip_prefix("unpair:") {
                    self.unpair(id.to_string(), window, cx);
                } else if let Some(id) = name.strip_prefix("confirm:") {
                    self.confirm_mac(id.to_string(), window, cx);
                } else if let Some(id) = name.strip_prefix("forget:") {
                    self.forget_mac(id.to_string(), window, cx);
                }
            }
        }
    }

    /// Takes the act's key as the one out, clearing what it last said; `false` while another is out.
    fn begin(&mut self, act: &str) -> bool {
        if self.macs.out.is_some() {
            return false;
        }
        self.macs.out = Some(act.to_string());
        self.macs.refused.remove(act);
        true
    }

    /// The act answered: a refusal is kept to be said under it.
    fn answered<T>(&mut self, act: &str, answer: Result<T, String>) -> Option<T> {
        self.macs.out = None;
        match answer {
            Ok(answer) => Some(answer),
            Err(said) => {
                self.macs.refused.insert(act.to_string(), said);
                None
            }
        }
    }

    fn holds(&self, name: &str, window: &Window) -> bool {
        self.macs.keys.get(name).is_some_and(|focus| focus.is_focused(window))
    }

    /// `POST /admin/macs/pair/start`: the link to copy, and its time left.
    fn pair(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.begin("pair") {
            return;
        }
        let press = self.wire.pair_start(cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(link) = this.answered("pair", answer) {
                    this.macs.link = Some(link);
                    this.tick_link(window, cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Draws the time left each second, and drops the link once it runs out; Copy link's keys go to Pair a Mac.
    fn tick_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.macs._tick = Some(cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(TICK).await;
            let going = this
                .update_in(cx, |this, window, cx| {
                    let Some(link) = &this.macs.link else { return false };
                    let good = link.expires_at > chrono::Utc::now().timestamp();
                    if !good {
                        let held = this.holds("copy-link", window);
                        this.macs.link = None;
                        if let Some(pair) = held.then(|| this.macs.keys.get("pair").cloned()).flatten() {
                            window.focus(&pair, cx);
                        }
                    }
                    if this.macs_shown(cx) {
                        cx.notify();
                    }
                    good
                })
                .unwrap_or(false);
            if !going {
                break;
            }
        }));
    }

    /// `POST /admin/macs/pair/claim` with what is pasted. The Mac it names joins the list, waiting on its Confirm.
    fn claim(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.begin("claim") {
            return;
        }
        let link = self.macs.paste.read(cx).value().to_string();
        let press = self.wire.pair_claim(link, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.answered("claim", answer).is_some() {
                    this.macs.paste.update(cx, |paste, cx| paste.set_value("", window, cx));
                    this.read_devices(cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// `POST /admin/macs/pair/confirm`. The row is linked at once, and the keys move to its Forget.
    fn confirm_mac(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let act = format!("confirm:{id}");
        if !self.begin(&act) {
            return;
        }
        let press = self.wire.pair_confirm(id, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(mac) = this.answered(&act, answer) {
                    let forget = this.macs.keys.entry(format!("forget:{}", mac.id)).or_insert_with(|| cx.focus_handle().tab_stop(true)).clone();
                    if this.holds(&act, window) {
                        window.focus(&forget, cx);
                    }
                    if let Some(row) = this.macs.macs.as_mut().and_then(|macs| macs.as_mut().ok()).and_then(|macs| macs.iter_mut().find(|one| one.id == mac.id)) {
                        *row = mac;
                    }
                    this.read_devices(cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// `POST /admin/macs/forget`, at one press: the Gateway forgets the pairing on both Macs.
    fn forget_mac(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let act = format!("forget:{id}");
        if !self.begin(&act) {
            return;
        }
        let press = self.wire.forget_mac(id.clone(), cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.answered(&act, answer).is_some() {
                    if let Some(rest) = this.holds(&act, window).then(|| this.macs.keys.get(AT_REST).cloned()).flatten() {
                        window.focus(&rest, cx);
                    }
                    if let Some(Ok(macs)) = this.macs.macs.as_mut() {
                        macs.retain(|one| one.id != id);
                    }
                    this.read_devices(cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// `POST /admin/setup/cloudflare`. A sign-in to wait for moves the keys to its button; a tunnel made moves them
    /// when the next read draws it.
    fn set_up_tunnel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.macs.pressing {
            return;
        }
        self.macs.pressing = true;
        self.macs.said = None;
        let press = self.wire.start_tunnel(cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.macs.pressing = false;
                match answer {
                    Err(said) => this.macs.said = Some(said),
                    Ok(made) => {
                        if let (Tunnel::SignIn(login), Some(Ok(setup))) = (&made, this.macs.setup.as_mut()) {
                            setup.cloudflare = CloudflareState::SigningIn { login: login.clone() };
                        }
                        if let Some(focus) = matches!(made, Tunnel::SignIn(_)).then(|| this.macs.keys.get("sign-in").cloned()).flatten() {
                            window.focus(&focus, cx);
                        }
                        this.read_setup(window, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn unpair(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.macs.unpair_said = None;
        if let Some(rest) = self.macs.keys.get(AT_REST).cloned() {
            window.focus(&rest, cx);
        }
        let press = self.wire.gateway_unpair(id, cx);
        cx.spawn(async move |this, cx| {
            let answer = press.await;
            let _ = this.update(cx, |this, cx| {
                if let Err(said) = answer {
                    this.macs.unpair_said = Some(said);
                }
                this.read_devices(cx);
                cx.notify();
            });
        })
        .detach();
    }

    // ---- drawn --------------------------------------------------------------------------------

    pub(super) fn render_macs(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        vec![self.this_mac_card(look), self.tunnel_card(look, window, cx), self.devices_card(look, window, cx)]
    }

    fn this_mac_card(&self, look: &Look) -> AnyElement {
        let body = match &self.macs.macs {
            None => div().flex().items_center().gap(look.length(token::SPACE_3)).child(bone(look, look.length(token::SPACE_3), look.length(token::SPACE_3))).child(bone(look, look.length(token::SPACE_6) * 4., look.length(token::LEADING_SM))).into_any_element(),
            Some(Err(said)) => alert(look, "macs-this-refused", said),
            Some(Ok(macs)) => match macs.iter().find(|one| one.this_mac) {
                None => div().into_any_element(),
                Some(mac) => div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(chip(look, mac))
                    .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(mac.name.clone())))
                    .children(mac.build.as_ref().map(|build| build_mark(look, "macs-this-build", build)))
                    .into_any_element(),
            },
        };
        labelled(card(look, "settings-card-this-mac", "This Mac", None, vec![body]), Role::Region, "This Mac").into_any_element()
    }

    fn tunnel_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let mut body: Vec<AnyElement> = Vec::new();
        match &self.macs.setup {
            None => {
                for n in 0..2 {
                    body.push(step_row(look, bone(look, look.length(token::SPACE_4), look.length(token::SPACE_4)).into_any_element(), bone(look, look.length(token::SPACE_6) * (6 - n) as f32, look.length(token::LEADING_SM)).into_any_element(), Vec::new(), None, n == 0));
                }
            }
            Some(Err(said)) => body.push(alert(look, "macs-gateway-refused", said)),
            Some(Ok(setup)) => {
                body.extend(self.tunnel_rows(setup, look, window, cx));
                if let Some(version) = &setup.cloudflared {
                    let line = div().id("macs-cloudflared").flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).tooltip(tip("cloudflared version".to_string())).child(SharedString::from(format!("cloudflared {version}")));
                    body.push(div().flex().child(labelled(line, Role::Label, "cloudflared version")).into_any_element());
                }
            }
        }
        let rows = div().flex().flex_col().gap(look.length(token::SPACE_3)).children(body).into_any_element();
        labelled(card(look, "settings-card-tunnel", "Tunnel", None, vec![rows]), Role::Region, "Tunnel").into_any_element()
    }

    fn tunnel_rows(&self, setup: &GatewaySetup, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        let page = &self.macs;
        let state = &setup.cloudflare;
        let refused: Vec<AnyElement> = page.said.iter().map(|said| alert(look, "macs-setup-refused", said)).collect();
        // Phones Funnel already serves come first, and the tunnel's row is for the other Macs.
        let funnel = page.funnel_row();
        let mut rows: Vec<AnyElement> = funnel
            .map(|address| {
                let line = labelled(div().id("macs-funnel").child(reach_line(look, "Phones reach this Mac at", address)), Role::Status, format!("Phones reach this Mac at {address}"));
                step_row(look, glyph(look, "macs-funnel-mark", "check", look.color(token::STATUS_COMPLETED_SUCCESS), "Connected"), line.into_any_element(), Vec::new(), Some(self.macs_button("copy", "Copy address", Weight::Ghost, false, look, window, cx)), true)
            })
            .into_iter()
            .collect();
        let first = rows.is_empty();
        let row = match state {
            CloudflareState::Running { address } => {
                let served = setup.served == Served::Funnel;
                let (lead, said) = if served { ("New phones pair at", format!("New phones pair at {address}")) } else { ("Phones and your other Macs reach this Mac at", format!("Reached at {address}")) };
                let line = labelled(div().id("macs-running").child(reach_line(look, lead, address)), Role::Status, said);
                let lines: Vec<AnyElement> = served.then(|| caption(look, "Phones already paired keep using Tailscale Funnel while it is on.").into_any_element()).into_iter().collect();
                step_row(look, glyph(look, "macs-running-mark", "check", look.color(token::STATUS_COMPLETED_SUCCESS), "Connected"), line.into_any_element(), lines, Some(self.macs_button("copy", "Copy address", Weight::Ghost, false, look, window, cx)), true)
            }
            CloudflareState::Down { address, since, why } => {
                let name = div().font_family(mono(look)).text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(address.clone()));
                let since = format!("Not connected since {}", clock_of(*since));
                let mut lines = vec![labelled(div().id("macs-down").child(caption(look, &since)), Role::Status, since.clone()).into_any_element(), caption(look, why).into_any_element()];
                lines.extend(refused);
                step_row(look, glyph(look, "macs-down-mark", "cloud-off", look.color(token::STATUS_COMPLETED_FAILED), "Not connected"), name.into_any_element(), lines, Some(self.macs_button("setup", "Set up tunnel", Weight::Primary, page.pressing, look, window, cx)), true)
            }
            CloudflareState::SigningIn { .. } => {
                let mut lines = vec![caption(look, "Waiting for the sign-in on Cloudflare's page.").into_any_element()];
                lines.extend(refused);
                step_row(look, glyph(look, "macs-signing-mark", "hourglass", look.color(token::STATUS_ESCALATED), "Waiting for the sign-in"), step_name(look, "Signing in to Cloudflare"), lines, Some(self.macs_button("sign-in", "Open Cloudflare sign-in", Weight::Primary, false, look, window, cx)), first)
            }
            CloudflareState::Missing | CloudflareState::SignedOut | CloudflareState::NoTunnel { .. } => {
                let said = if funnel.is_some() { "Your other Macs need a Cloudflare tunnel." } else { "Phones and your other Macs reach this Mac through it." };
                let mut lines = vec![caption(look, said).into_any_element()];
                // The last try's failure, as Fleet wrote it, with Set up tunnel beside it.
                if let CloudflareState::NoTunnel { why: Some(why) } = state {
                    lines.push(alert(look, "macs-tunnel-why", why));
                }
                lines.extend(refused);
                step_row(look, not_done(look, "macs-tunnel-mark"), step_name(look, "No tunnel yet"), lines, Some(self.macs_button("setup", "Set up tunnel", Weight::Primary, page.pressing, look, window, cx)), first)
            }
        };
        rows.push(row);
        rows
    }

    fn devices_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let page = &self.macs;
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut first = true;
        // Each row, with what its press was refused under it.
        let mut take = |row: gpui::Div, refused: Option<AnyElement>, rows: &mut Vec<AnyElement>| {
            let row = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(row).children(refused);
            rows.push(row.when(!first, |row| row.pt(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE))).pb(look.length(token::SPACE_3)).into_any_element());
            first = false;
        };
        let refused_at = |act: &str| page.refused.get(act).map(|said| alert(look, SharedString::from(format!("macs-{act}-refused")), said));
        match (&page.macs, &page.phones) {
            (None, _) | (_, None) => {
                for _ in 0..3 {
                    take(div().flex().items_center().gap(look.length(token::SPACE_3)).child(bone(look, look.length(token::SPACE_3), look.length(token::SPACE_3))).child(bone(look, look.length(token::SPACE_6) * 5., look.length(token::LEADING_SM))), None, &mut rows);
                }
            }
            (Some(macs), Some(phones)) => {
                if let Err(said) = macs {
                    rows.push(alert(look, "macs-list-refused", said));
                }
                for mac in macs.iter().flatten() {
                    let mark = mac.link.as_ref().map(|link| {
                        let (name, ink, said) = link_mark(look, link, mac.why.as_deref());
                        glyph(look, SharedString::from(format!("macs-link-{}", mac.id)), name, ink, &said)
                    });
                    let asleep = matches!(mac.link, Some(MacLink::Asleep { .. }));
                    let act = press_of_mac(mac).map(|(act, said)| (format!("{act}:{}", mac.id), said));
                    let press = act.as_ref().and_then(|(key, said)| self.row_button(key, said, format!("{said} {}", mac.name), matches!(mac.link, Some(MacLink::ConfirmWaiting)), look, window, cx));
                    let row = device_row(look, chip(look, mac), &mac.name, mac.this_mac.then_some("This Mac"), mac.why.as_deref(), asleep)
                        .child(div().flex_none().children(mac.build.as_ref().map(|build| build_mark(look, format!("macs-build-{}", mac.id), build))))
                        .child(div().flex_none().w(look.length(token::SPACE_6)).flex().justify_center().children(mark))
                        .child(press_column(look, press));
                    take(row, act.and_then(|(key, _)| refused_at(&key)), &mut rows);
                }
                // A press refused on a Mac the list no longer draws is said at its foot.
                let drawn = |key: &str| macs.iter().flatten().any(|mac| press_of_mac(mac).is_some_and(|(act, _)| key == format!("{act}:{}", mac.id)));
                for (key, said) in page.refused.iter().filter(|(key, _)| (key.starts_with("confirm:") || key.starts_with("forget:")) && !drawn(key)) {
                    rows.push(alert(look, SharedString::from(format!("macs-{key}-refused")), said));
                }
                if let Err(said) = phones {
                    rows.push(alert(look, "macs-phones-refused", said));
                }
                for phone in phones.iter().flatten() {
                    let seen = phone.last_seen_at.map_or("Never seen".to_string(), |at| format!("Seen {}", age_of(at)));
                    let icon = svg().flex_none().path(icon_path("smartphone")).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(look.color(token::FG_MUTED)).into_any_element();
                    let unpair = self.unpair_button(phone, look, window, cx);
                    let row = device_row(look, icon, &phone.name, None, None, false)
                        .child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(seen)))
                        .child(div().flex_none().w(look.length(token::SPACE_6)))
                        .child(press_column(look, unpair));
                    take(row, None, &mut rows);
                }
            }
        }
        if let Some(said) = &page.unpair_said {
            rows.push(alert(look, "macs-unpair-refused", said));
        }
        let rest = page.keys.get(AT_REST).cloned();
        let list = labelled(div().id("macs-devices").flex().flex_col().when_some(rest, |list, rest| list.track_focus(&rest)).children(rows), Role::List, "Devices");
        let pair = self.macs_button("pair", "Pair a Mac", Weight::Ghost, page.out.as_deref() == Some("pair"), look, window, cx);
        let body = vec![self.pairing(look, window, cx), list.into_any_element()];
        labelled(card(look, "settings-card-devices", "Devices", Some(pair), body), Role::Region, "Devices").into_any_element()
    }

    /// The link Pair a Mac made, to copy with its time left, and the box the other Mac's link is pasted into.
    fn pairing(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let page = &self.macs;
        let refused = |act: &str| page.refused.get(act).map(|said| alert(look, SharedString::from(format!("macs-{act}-refused")), said));
        let made = page.link.as_ref().map(|made| {
            let left = time_left(made.expires_at - chrono::Utc::now().timestamp());
            // The code is at the link's end, so a narrow page wraps it rather than cutting it off.
            let link = div().id("macs-pair-link").flex_1().min_w_0().font_family(mono(look)).text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(made.link.clone()));
            let figure = div().id("macs-pair-left").flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip("Time left on this link")).child(SharedString::from(left.clone()));
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .child(labelled(link, Role::Label, made.link.clone()))
                .child(labelled(figure, Role::Label, format!("{left} left on this link")))
                .child(self.macs_button("copy-link", "Copy link", Weight::Ghost, false, look, window, cx))
        });
        let field = labelled(div().id("macs-paste").flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(&page.paste)), Role::TextInput, "Paste the link from your other Mac");
        let paste = div().flex().items_center().gap(look.length(token::SPACE_3)).child(field).child(self.macs_button("claim", "Claim", Weight::Ghost, page.out.as_deref() == Some("claim"), look, window, cx));
        div().flex().flex_col().gap(look.length(token::SPACE_2)).children(refused("pair")).children(made).child(paste).children(refused("claim")).into_any_element()
    }

    /// A press on one row of the list, named for the row's Mac.
    fn row_button(&self, key: &str, said: &'static str, name: String, withdraws: bool, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let focus = self.macs.keys.get(key)?.clone();
        let off = !self.live_gateway() || self.macs.out.as_deref() == Some(key);
        let button = ghost(look, SharedString::from(format!("macs-{key}")), said, off);
        let key = key.to_string();
        let button = ringed(look, button, &focus, window).track_focus(&focus).when(!off, |button| button.on_click(cx.listener(move |this, _, window, cx| this.press_macs(&key, window, cx))));
        let button = button.when(withdraws, |button| button.tooltip(tip("Withdraws this pairing")));
        Some(labelled(button, Role::Button, name).into_any_element())
    }

    fn unpair_button(&self, phone: &PairedPhone, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let name = format!("unpair:{}", phone.id);
        let focus = self.macs.keys.get(&name)?.clone();
        let button = ghost(look, SharedString::from(format!("macs-{name}")), "Unpair", !self.live_gateway());
        let button = ringed(look, button, &focus, window).track_focus(&focus).on_click(cx.listener(move |this, _, window, cx| this.press_macs(&name, window, cx)));
        Some(labelled(button, Role::Button, format!("Unpair {}", phone.name)).into_any_element())
    }

    fn live_gateway(&self) -> bool {
        !matches!(self.macs.setup, Some(Err(_)))
    }

    #[allow(clippy::too_many_arguments)]
    fn macs_button(&self, name: &'static str, said: &'static str, weight: Weight, off: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.macs.keys.get(name).cloned() else { return div().into_any_element() };
        let id = SharedString::from(format!("macs-{name}"));
        let button = match weight {
            Weight::Ghost => ghost(look, id, said, off),
            Weight::Primary => super::change_band::filled(look, id, said, off),
        };
        let button = ringed(look, button, &focus, window).track_focus(&focus).when(!off, |button| button.on_click(cx.listener(move |this, _, window, cx| this.press_macs(name, window, cx))));
        labelled(button, Role::Button, said).into_any_element()
    }
}

#[derive(Clone, Copy)]
enum Weight {
    Ghost,
    Primary,
}

/// One row as Classic's `.armada-phone__step`: the glyph, the name over its lines, and the press at the right.
fn step_row(look: &Look, mark: AnyElement, name: AnyElement, lines: Vec<AnyElement>, press: Option<AnyElement>, first: bool) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_3))
        .pb(look.length(token::SPACE_3))
        .when(!first, |row| row.pt(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
        .child(div().flex_none().w(look.length(token::SPACE_4)).pt(look.length(token::SPACE_1)).child(mark))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).child(name).children(lines))
        .children(press.map(|press| div().flex_none().child(press)))
        .into_any_element()
}

fn step_name(look: &Look, said: &'static str) -> AnyElement {
    div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(said).into_any_element()
}

fn caption(look: &Look, said: &str) -> gpui::Div {
    div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.to_string()))
}

fn glyph(look: &Look, id: impl Into<SharedString>, name: &'static str, ink: Hsla, said: &str) -> AnyElement {
    labelled(div().id(id.into()).flex_none().tooltip(tip(said.to_string())).child(svg().path(icon_path(name)).size(look.length(token::SPACE_4)).text_color(ink)), Role::Image, said.to_string()).into_any_element()
}

/// A sentence ending on an address, the address in mono.
fn reach_line(look: &Look, lead: &str, address: &str) -> gpui::Div {
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .text_size(look.length(token::TEXT_SM))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(format!("{lead}\u{a0}")))
        .child(div().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(address.to_string())))
}

/// A build's short hash, with a tooltip saying what the figure is.
fn build_mark(look: &Look, id: impl Into<SharedString>, build: &str) -> AnyElement {
    let said = format!("Build {build}");
    let hash = div().id(id.into()).flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip(said.clone())).child(SharedString::from(build.to_string()));
    labelled(hash, Role::Label, said).into_any_element()
}

fn not_done(look: &Look, id: &'static str) -> AnyElement {
    glyph(look, id, "circle-dashed", look.color(token::FG_SUBTLE), "Not done yet")
}

fn alert(look: &Look, id: impl Into<gpui::ElementId>, said: &str) -> AnyElement {
    labelled(super::view::refusal(look, said.to_string()).id(id), Role::Alert, said.to_string()).into_any_element()
}

/// A Mac's colour, as a small filled square beside its name.
fn chip(look: &Look, mac: &Mac) -> AnyElement {
    let said = format!("{} colour", mac.colour.name());
    labelled(div().id(SharedString::from(format!("macs-chip-{}", mac.id))).flex_none().size(look.length(token::SPACE_3)).rounded(look.length(token::RADIUS_SM)).bg(colour_of(look, mac.colour)), Role::Image, said).into_any_element()
}

/// A Devices row: its mark, its name, a quiet word after it, and the Gateway's why under it; the caller adds what
/// goes at the right. A dimmed row's name is in the muted ink.
fn device_row(look: &Look, mark: AnyElement, name: &str, aside: Option<&'static str>, why: Option<&str>, dim: bool) -> gpui::Div {
    let ink = if dim { look.color(token::FG_MUTED) } else { look.color(token::FG_DEFAULT) };
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .child(div().flex_none().w(look.length(token::SPACE_4)).flex().justify_center().child(mark))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(look.length(token::SPACE_2))
                        .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(ink).child(SharedString::from(name.to_string())))
                        .children(aside.map(|aside| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(aside))),
                )
                .children(why.map(|why| caption(look, why))),
        )
}

/// The column every row's press sits in, the same width down the list.
fn press_column(look: &Look, press: Option<AnyElement>) -> gpui::Div {
    div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_6)).flex().justify_end().children(press)
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, VisualTestContext};

    use super::super::gateway::MOCK_OTHER_LINK;
    use super::super::mock::MockFleet;
    use super::super::wire::Wire;
    use super::*;

    fn opened(cx: &mut TestAppContext) -> (Entity<SettingsOverlay>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (overlay, cx) = cx.add_window_view(|window, cx| SettingsOverlay::new(Wire::Mock(Box::new(MockFleet::new())), window, cx));
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.read_macs(window, cx)));
        cx.run_until_parked();
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.macs_keys(cx)));
        (overlay, cx)
    }

    fn press(overlay: &Entity<SettingsOverlay>, name: &str, cx: &mut VisualTestContext) {
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.press_macs(name, window, cx)));
        cx.run_until_parked();
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.macs_keys(cx)));
    }

    fn links(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Vec<(String, Option<MacLink>)> {
        overlay.read_with(cx, |this, _| this.macs.macs.clone().and_then(Result::ok).unwrap_or_default().into_iter().map(|mac| (mac.id, mac.link)).collect())
    }

    fn refused(overlay: &Entity<SettingsOverlay>, act: &str, cx: &mut VisualTestContext) -> Option<String> {
        overlay.read_with(cx, |this, _| this.macs.refused.get(act).cloned())
    }

    #[gpui::test]
    fn the_list_is_drawn_in_the_gateways_order_with_one_press_a_row_where_it_has_one(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        let ids: Vec<String> = links(&overlay, cx).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, ["mac-studio", "mac-work", "mac-mini", "mac-air", "mac-pro"]);
        overlay.read_with(cx, |this, _| {
            let presses: Vec<_> = this.macs.macs.iter().flatten().flatten().map(|mac| press_of_mac(mac).map(|(act, _)| act)).collect();
            assert_eq!(presses, [None, Some("forget"), Some("forget"), Some("forget"), Some("confirm")], "this Mac has no press; one waiting on Confirm there can be forgotten");
        });
    }

    #[test]
    fn each_link_state_has_its_glyph_and_tooltip() {
        let since = chrono::Utc::now().timestamp() - 3600;
        let said = |link: MacLink, why: Option<&str>| link_said(&link, why);
        assert_eq!(said(MacLink::Linked, None), ("link", "Linked".to_string()));
        let (glyph, asleep) = said(MacLink::Asleep { since }, Some("Mac mini went quiet for 90 seconds."));
        assert_eq!(glyph, "moon");
        let held = super::super::time::held();
        assert!(asleep.starts_with("Asleep since ") && asleep.matches(':').count() == if held.seconds { 2 } else { 1 }, "as his time format says: {asleep}");
        assert_eq!(said(MacLink::ClaimWaiting, None), ("hourglass", "Waiting for Confirm on this Mac".to_string()));
        assert_eq!(said(MacLink::ConfirmWaiting, Some("Confirm on MacBook Air")), ("hourglass", "Confirm on MacBook Air".to_string()));
        assert_eq!(said(MacLink::OlderBuild, None).0, "arrow-down-to-line");
        assert_eq!(said(MacLink::NewerBuild, None).0, "arrow-up-from-line");
    }

    #[test]
    fn time_left_is_minutes_and_seconds() {
        assert_eq!(time_left(299), "4:59");
        assert_eq!(time_left(7), "0:07");
        assert_eq!(time_left(-3), "0:00");
    }

    #[gpui::test]
    fn start_claim_and_confirm_pair_a_mac_on_the_mock(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        press(&overlay, "pair", cx);
        let link = overlay.read_with(cx, |this, _| this.macs.link.clone()).expect("Pair a Mac makes a link");
        assert!(link.link.contains("/pair/mac?code="));
        press(&overlay, "copy-link", cx);
        assert_eq!(overlay.read_with(cx, |this, _| match &this.wire {
            Wire::Mock(mock) => mock.handed.last().cloned(),
            Wire::Live(_) => None,
        }), Some(link.link), "Copy link copies the link");

        cx.update(|window, cx| overlay.update(cx, |this, cx| this.macs.paste.update(cx, |paste, cx| paste.set_value(MOCK_OTHER_LINK, window, cx))));
        press(&overlay, "claim", cx);
        assert_eq!(refused(&overlay, "claim", cx), None);
        assert!(links(&overlay, cx).contains(&("mac-office".into(), Some(MacLink::ConfirmWaiting))), "the claimed Mac waits on its Confirm there");
        assert!(overlay.read_with(cx, |this, cx| this.macs.paste.read(cx).value().is_empty()), "the box empties");

        press(&overlay, "confirm:mac-pro", cx);
        assert_eq!(refused(&overlay, "confirm:mac-pro", cx).as_deref(), Some("Mac Pro did not answer. Press Confirm again."));
        press(&overlay, "confirm:mac-pro", cx);
        assert_eq!(refused(&overlay, "confirm:mac-pro", cx), None, "a press that goes through clears what was said");
        assert!(links(&overlay, cx).contains(&("mac-pro".into(), Some(MacLink::Linked))));
    }

    #[gpui::test]
    fn a_refused_claim_is_said_as_the_gateway_gave_it(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.macs.paste.update(cx, |paste, cx| paste.set_value("hello", window, cx))));
        press(&overlay, "claim", cx);
        assert_eq!(refused(&overlay, "claim", cx).as_deref(), Some("This is not a pairing link from another Mac. Copy it again from Settings, Macs on that Mac."));
        assert_eq!(links(&overlay, cx).len(), 5, "nothing joined the list");
    }

    #[gpui::test]
    fn forget_takes_the_mac_off_the_list(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        press(&overlay, "forget:mac-work", cx);
        assert_eq!(refused(&overlay, "forget:mac-work", cx), None);
        assert!(links(&overlay, cx).iter().all(|(id, _)| id != "mac-work"));
        assert!(links(&overlay, cx).contains(&("mac-air".into(), Some(MacLink::ConfirmWaiting))));
        press(&overlay, "forget:mac-air", cx);
        assert_eq!(refused(&overlay, "forget:mac-air", cx), None);
        assert!(links(&overlay, cx).iter().all(|(id, _)| id != "mac-air"), "a pairing waiting on Confirm there is withdrawn by the same route");
    }
}
