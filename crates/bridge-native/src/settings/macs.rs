//! Settings › Macs: this Mac, its Cloudflare tunnel, and every Mac and phone that reaches it, read off the Phone
//! Gateway (`gateway.rs`). The tunnel's rows are drawn as Classic's Funnel rows are (`PhonePairing`'s `WayIn`).
//!
//! **He opens it cold, with no walk** (the owner, 12 Nov). Each state of the tunnel shows the one press that moves
//! it on, and why where there is a why; nothing tells him to go and do something outside Bridge, and nothing asks
//! him to type. Cloudflare's sign-in page opens only when he presses for it.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use gpui::{div, prelude::*, px, svg, Animation, AnimationExt, AnyElement, App, Context, DismissEvent, Entity, FocusHandle, Focusable, FontWeight, Hsla, Keystroke, MouseButton, MouseDownEvent, Pixels, Point, Role, SharedString, Subscription, Task, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{CloudflareState, GatewaySetup, Mac, MacColour, MacLink, MacPairLink, PairedPhone, Served, UpdateMac};

use super::entries::Body;
use crate::dropdown::{Dropdown, Entry, Picked};
use super::gateway::{PhoneClaim, PhoneCode, Tunnel, NOT_RUNNING};
use super::overlay::SettingsOverlay;
use super::view::{bone, bounds_of, card, ghost, ringed, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The setup is read again this often until the tunnel runs (`RETRY_EVERY`).
const RETRY: Duration = Duration::from_secs(5);

/// The Macs and phones are read again this often while the page is shown, as a phone's claim is polled.
const LIST_EVERY: Duration = Duration::from_secs(2);

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
    /// A pairing act's refusal, said under it, by the act's key: `pair`, `claim`, `confirm:<id>`, `forget:<id>`,
    /// `update:<id>` or `update-this:<id>`.
    refused: BTreeMap<String, String>,
    /// The Macs whose Update was pressed and whose row still reads another build: the press says Updating.
    updating: BTreeSet<String>,
    /// The pairing act out; another waits for its answer.
    out: Option<String>,
    keys: BTreeMap<String, FocusHandle>,
    _poll: Option<Task<()>>,
    _reads: Vec<Task<()>>,
    /// Reads the list again while Settings is open, so a claim from another Mac draws its Confirm.
    _list: Option<Task<()>>,
    /// The link's time left, drawn again each second while it is good.
    _tick: Option<Task<()>>,
    _paste: Subscription,
    /// The card he pressed on a Mac nothing was set up on; the setup runs on from it by itself.
    lead: Option<Lead>,
    /// `POST /phone`'s refusal.
    phone_said: Option<String>,
    /// The setup pressed Set up tunnel itself, once.
    tunnel_asked: bool,
    /// Pair with a link instead was pressed.
    by_link: bool,
    /// The code Pair a phone made, until it runs out, is confirmed or declined.
    phone: Option<PhoneLink>,
    /// The phone's link ran out: Make a new link is offered in its place.
    lapsed: bool,
    /// Counts the phone code down, and asks the Gateway for a claim every other second.
    _phone: Option<Task<()>>,
}

/// A phone's pairing code with the link its camera opens, drawn as a QR code, and the phone that entered it.
struct PhoneLink {
    code: PhoneCode,
    link: String,
    qr: Qr,
    /// The phone's name, once it has entered the code.
    claim: Option<String>,
}

/// The QR code's modules, row by row, dark where `true`.
#[derive(Clone)]
struct Qr {
    width: usize,
    dark: std::sync::Arc<[bool]>,
}

impl Qr {
    /// `None` when the link is too long for a code.
    fn of(link: &str) -> Option<Qr> {
        use qrcode::{Color, EcLevel, QrCode};
        let code = QrCode::with_error_correction_level(link.as_bytes(), EcLevel::M).ok()?;
        Some(Qr { width: code.width(), dark: code.to_colors().into_iter().map(|one| one == Color::Dark).collect() })
    }
}

/// Modules of empty margin round the code; the spec asks four and phone cameras rely on it.
const QUIET: usize = 4;

/// The code is drawn about this wide; each module is a whole number of points, so no edge falls between pixels.
const QR_SIDE: f32 = 192.;

/// Phone pairing's claim poll, every this many ticks (`CLAIM_EVERY` is two seconds in Classic).
const POLL_TICKS: u32 = 2;

/// The two ways a Mac nothing is set up on starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lead {
    /// Pair with a Mac that's set up.
    Find,
    /// Set up this Mac first.
    SetUp,
}

/// What the page draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    /// Nothing is set up, and nothing pressed: the two cards.
    Cards,
    /// This Mac's setup, its steps ticking off.
    Steps,
    /// This Mac is set up and no other Mac has turned up yet.
    Waiting,
    /// This Mac, its tunnel and every device.
    Page,
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
            updating: BTreeSet::new(),
            out: None,
            keys: BTreeMap::new(),
            _poll: None,
            _reads: Vec::new(),
            _list: None,
            _tick: None,
            _paste: sub,
            lead: None,
            phone_said: None,
            tunnel_asked: false,
            by_link: false,
            phone: None,
            lapsed: false,
            _phone: None,
        }
    }

    /// Another Mac is on the list: paired, or a pairing waiting on a Confirm.
    fn others(&self) -> bool {
        matches!(&self.macs, Some(Ok(macs)) if macs.iter().any(|mac| !mac.this_mac))
    }

    /// Nothing is set up: the Gateway does not answer, or it answers with no way in and no other Mac.
    fn bare(&self) -> bool {
        match &self.setup {
            None => false,
            Some(Err(_)) => true,
            Some(Ok(setup)) => matches!(setup.cloudflare, CloudflareState::Missing | CloudflareState::SignedOut | CloudflareState::NoTunnel { .. }) && setup.served != Served::Funnel && !self.others(),
        }
    }

    fn runs(&self) -> bool {
        matches!(self.cloudflare(), Some(CloudflareState::Running { .. }))
    }

    /// The tunnel's setup stopped: its press was refused, or the sign-in made no tunnel.
    fn stuck(&self) -> Option<&str> {
        self.said.as_deref().or(match self.cloudflare() {
            Some(CloudflareState::NoTunnel { why: Some(why) }) => Some(why.as_str()),
            _ => None,
        })
    }

    fn part(&self) -> Part {
        let Some(lead) = self.lead else { return if self.bare() { Part::Cards } else { Part::Page } };
        if self.others() {
            return Part::Page;
        }
        match lead {
            // Finding a Mac waits for one however this Mac's tunnel went: the link still pairs them.
            Lead::Find if self.phone_said.is_none() && (self.runs() || self.stuck().is_some()) => Part::Waiting,
            Lead::SetUp if self.runs() => Part::Page,
            _ => Part::Steps,
        }
    }

    /// Settings opened again: the Gateway is asked again the next time the page is drawn.
    pub fn reopen(&mut self) {
        self.asked = false;
        self.said = None;
        self.unpair_said = None;
        self.refused.clear();
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

/// The press a Mac's row carries, by its key's prefix and its words: Confirm on a claim waiting here, Update on a
/// Mac on another build, Forget on a paired Mac or one waiting for Confirm there, and none on this Mac.
fn press_of_mac(mac: &Mac) -> Option<(&'static str, &'static str)> {
    match mac.link.as_ref()? {
        MacLink::ClaimWaiting => Some(("confirm", "Confirm")),
        MacLink::OlderBuild => Some(("update", "Update")),
        MacLink::NewerBuild => Some(("update-this", "Update this Mac")),
        MacLink::Linked | MacLink::Asleep { .. } | MacLink::ConfirmWaiting => Some(("forget", "Forget")),
    }
}

/// Whether the link is frozen on a build that differs: Update replaces Forget, which moves to the row's menu.
fn frozen(mac: &Mac) -> bool {
    matches!(mac.link, Some(MacLink::OlderBuild | MacLink::NewerBuild))
}

/// What a press on one row is called for VoiceOver and for a walk, and what its tooltip says it does.
fn press_said(mac: &Mac, said: &str, going: bool) -> (String, Option<String>) {
    match (&mac.link, going) {
        (Some(MacLink::OlderBuild), false) => (format!("Update {}", mac.name), Some(format!("Update {} onto this Mac's build", mac.name))),
        (Some(MacLink::OlderBuild), true) => (format!("Updating {}", mac.name), Some(format!("Updating {} onto this Mac's build", mac.name))),
        (Some(MacLink::NewerBuild), false) => (format!("Update this Mac onto {}", mac.name), Some(format!("Update this Mac onto {}'s build", mac.name))),
        (Some(MacLink::NewerBuild), true) => (format!("Updating this Mac onto {}", mac.name), Some(format!("Updating this Mac onto {}'s build", mac.name))),
        _ => (format!("{said} {}", mac.name), None),
    }
}

/// The key that opens a row's menu from the keyboard: Shift+F10, or the menu key.
fn opens_menu(key: &Keystroke) -> bool {
    key.key == "menu" || (key.key == "f10" && key.modifiers.shift)
}

fn age_of(at: i64) -> String {
    age_said((chrono::Utc::now().timestamp() - at).max(0))
}

/// How long ago a device was seen; under a minute is just now, never "0m ago".
fn age_said(seconds: i64) -> String {
    if seconds < 60 {
        return "just now".into();
    }
    format!("{} ago", crate::merge_strip::words::span_said(seconds))
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
        self.watch_list(window, cx);
    }

    /// `GET /admin/macs` and the phones every two seconds while the page is shown, whatever the tunnel's state: the
    /// other Mac's claim arrives after the page opened, and its Confirm must be drawn without reopening Settings.
    fn watch_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.macs._list = Some(cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(LIST_EVERY).await;
            let going = this
                .update(cx, |this, cx| {
                    if this.macs_shown(cx) {
                        this.read_devices(cx);
                    }
                    this.open
                })
                .unwrap_or(false);
            if !going {
                break;
            }
        }));
    }

    fn read_devices(&mut self, cx: &mut Context<Self>) {
        let (macs, phones) = (self.wire.gateway_macs(cx), self.wire.gateway_phones(cx));
        self.macs._reads = vec![
            cx.spawn(async move |this, cx| {
                let answer = macs.await;
                let _ = this.update(cx, |this, cx| {
                    this.macs.macs = Some(answer);
                    // A row that no longer reads another build is done updating.
                    let page = &mut this.macs;
                    if let Some(Ok(macs)) = &page.macs {
                        page.updating.retain(|id| macs.iter().any(|mac| &mac.id == id && frozen(mac)));
                    }
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
                    let held = this.macs.keys.values().any(|focus| focus.is_focused(window));
                    this.macs.setup = Some(answer);
                    this.lead_on(window, cx);
                    // While a setup leads, the list is read each round too, so a Mac that turns up is drawn.
                    let leads = this.macs.lead.is_some() && !this.macs.others();
                    if leads {
                        this.read_devices(cx);
                    }
                    this.macs_keys(cx);
                    if held && !this.macs_stops().iter().any(|focus| focus.is_focused(window)) {
                        let press = if this.macs.part() == Part::Page { this.macs.cloudflare().and_then(|state| this.macs.keys.get(press_of(state)?)).cloned() } else { this.macs_stops().first().cloned() };
                        if let Some(press) = press.or_else(|| this.macs.keys.get(AT_REST).cloned()) {
                            window.focus(&press, cx);
                        }
                    }
                    cx.notify();
                    !runs || leads
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

    /// A card pressed: `POST /phone {on: true}`, then the setup reads the Gateway and runs on by itself.
    fn start_setup(&mut self, lead: Lead, window: &mut Window, cx: &mut Context<Self>) {
        let page = &mut self.macs;
        page.lead = Some(lead);
        page.phone_said = None;
        page.said = None;
        page.tunnel_asked = false;
        page.by_link = false;
        let press = self.wire.phone_on(cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match answer {
                    Ok(_) => this.read_setup(window, cx),
                    Err(refused) => this.macs.phone_said = Some(refused.said),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// The setup's next press, made for him: Set up tunnel, once the Gateway answers and no tunnel runs.
    fn lead_on(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.macs.lead.is_none() || self.macs.tunnel_asked {
            return;
        }
        if matches!(self.macs.cloudflare(), Some(CloudflareState::Missing | CloudflareState::SignedOut | CloudflareState::NoTunnel { .. } | CloudflareState::Down { .. })) {
            self.macs.tunnel_asked = true;
            self.set_up_tunnel(window, cx);
        }
    }

    /// Keys for each control, made before they are drawn.
    pub(super) fn macs_keys(&mut self, cx: &mut Context<Self>) {
        let mut names: Vec<String> = ["setup", "sign-in", "copy", "pair", "copy-link", "claim", "find", "set-up", "again", "by-link", "pair-phone", "copy-phone-link", "phone-new", "phone-confirm", "phone-decline"].into_iter().map(String::from).collect();
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
        match page.part() {
            Part::Cards => {
                stops.extend(key("find"));
                stops.extend(key("set-up"));
                return stops;
            }
            Part::Steps => {
                if matches!(page.cloudflare(), Some(CloudflareState::SigningIn { .. })) {
                    stops.extend(key("sign-in"));
                }
                if page.phone_said.is_some() || page.stuck().is_some() {
                    stops.extend(key("again"));
                }
                return stops;
            }
            Part::Waiting => {
                if !page.by_link {
                    stops.extend(key("by-link"));
                    return stops;
                }
                stops.extend(key("pair"));
                if page.link.is_some() {
                    stops.extend(key("copy-link"));
                }
                stops.push(page.paste_focus.clone());
                stops.extend(key("claim"));
                return stops;
            }
            Part::Page => {}
        }
        if page.funnel_row().is_some() {
            stops.extend(key("copy"));
        }
        stops.extend(page.cloudflare().and_then(press_of).and_then(key));
        stops.extend(key("pair"));
        stops.extend(key("pair-phone"));
        if let Some(phone) = &page.phone {
            stops.extend(key("copy-phone-link"));
            if phone.claim.is_some() {
                stops.extend(key("phone-confirm"));
                stops.extend(key("phone-decline"));
            }
        } else if page.lapsed {
            stops.extend(key("phone-new"));
        }
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
        if opens_menu(key) && !mods.platform && !mods.control && !mods.alt {
            return self.open_menu_from_keys(window, cx);
        }
        if mods.platform || mods.control || mods.alt || !matches!(key.key.as_str(), "enter" | "space") {
            return false;
        }
        let Some(name) = self.macs.keys.iter().find(|(_, focus)| focus.is_focused(window)).map(|(name, _)| name.clone()) else { return false };
        self.press_macs(&name, window, cx);
        true
    }

    /// The Mac whose Update holds the keys, by id.
    fn update_held(&self, window: &Window) -> Option<String> {
        self.macs.keys.iter().filter(|(_, focus)| focus.is_focused(window)).find_map(|(name, _)| name.strip_prefix("update-this:").or_else(|| name.strip_prefix("update:")).map(String::from))
    }

    /// Shift+F10 on a Mac whose link is frozen: its menu opens under its press.
    fn open_menu_from_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.update_held(window) else { return false };
        let Some(mac) = self.macs.macs.as_ref().and_then(|macs| macs.as_ref().ok()).and_then(|macs| macs.iter().find(|one| one.id == id)).cloned() else { return false };
        let (name, _) = press_said(&mac, "", self.macs.updating.contains(&id));
        let at = bounds_of(window, cx, Role::Button, &name).map(|bounds| bounds.bottom_left()).unwrap_or_default();
        self.open_mac_menu(id, mac.name, at, true, window, cx);
        true
    }

    /// Forget stays reachable on a row whose press is Update: a menu of one item, at the pointer or under the press.
    fn open_mac_menu(&mut self, id: String, name: String, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let back = window.focused(cx);
        let entries = vec![Entry::Act { id: "forget", label: format!("Forget {name}").into(), icon: None, danger: false }];
        let menu = cx.new(|cx| Dropdown::new(name, entries, at, gpui::Anchor::TopLeft, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: pick, .. } = picked;
            if *pick == "forget" {
                this.forget_mac(id.clone(), window, cx);
            }
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.popup = None;
            if let Some(back) = &back {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some((menu, vec![picked, dismissed]));
        cx.notify();
    }

    /// An Update that held the keys and is no longer drawn, because its Mac is linked or forgotten, hands them on:
    /// to the Mac's Forget, or to the list.
    pub(super) fn macs_settle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.update_held(window) else { return };
        let drawn = matches!(&self.macs.macs, Some(Ok(macs)) if macs.iter().any(|mac| mac.id == id && frozen(mac)));
        if drawn {
            return;
        }
        let present = matches!(&self.macs.macs, Some(Ok(macs)) if macs.iter().any(|mac| mac.id == id));
        let forget = present.then(|| self.macs.keys.get(&format!("forget:{id}"))).flatten();
        let Some(to) = forget.or_else(|| self.macs.keys.get(AT_REST)).cloned() else { return };
        window.defer(cx, move |window, cx| window.focus(&to, cx));
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
            "pair-phone" | "phone-new" => self.pair_phone(window, cx),
            "copy-phone-link" => {
                if let Some(link) = self.macs.phone.as_ref().map(|phone| phone.link.clone()) {
                    self.wire.copy(&link, cx);
                }
            }
            "phone-confirm" => self.confirm_phone(window, cx),
            "phone-decline" => self.decline_phone(window, cx),
            "find" => self.start_setup(Lead::Find, window, cx),
            "set-up" => self.start_setup(Lead::SetUp, window, cx),
            "again" => {
                if let Some(lead) = self.macs.lead {
                    self.start_setup(lead, window, cx);
                }
            }
            "by-link" => {
                self.macs.by_link = true;
                window.focus(&self.macs.paste_focus.clone(), cx);
                cx.notify();
            }
            _ => {
                if let Some(id) = name.strip_prefix("unpair:") {
                    self.unpair(id.to_string(), window, cx);
                } else if let Some(id) = name.strip_prefix("confirm:") {
                    self.confirm_mac(id.to_string(), window, cx);
                } else if let Some(id) = name.strip_prefix("forget:") {
                    self.forget_mac(id.to_string(), window, cx);
                } else if let Some(id) = name.strip_prefix("update-this:") {
                    self.update_mac(id.to_string(), true, window, cx);
                } else if let Some(id) = name.strip_prefix("update:") {
                    self.update_mac(id.to_string(), false, window, cx);
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

    /// `POST /admin/pair/start`: the code a phone's camera opens, as a link and a QR code, and its time left.
    fn pair_phone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.begin("pair-phone") {
            return;
        }
        let press = self.wire.phone_start(cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(code) = this.answered("pair-phone", answer) {
                    let link = code.link();
                    match Qr::of(&link) {
                        Some(qr) => {
                            this.macs.lapsed = false;
                            this.macs.phone = Some(PhoneLink { code, link, qr, claim: None });
                            this.watch_phone(window, cx);
                            if let Some(copy) = this.macs.keys.get("copy-phone-link").cloned() {
                                window.focus(&copy, cx);
                            }
                        }
                        None => {
                            this.macs.refused.insert("pair-phone".into(), "The link is too long to draw as a QR code.".into());
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Draws the phone code's time left each second, asks for a claim every other, and drops the code once it runs
    /// out. The keys on its controls go to the one that is left.
    fn watch_phone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.macs._phone = Some(cx.spawn_in(window, async move |this, cx| {
            let mut ticks = 0u32;
            loop {
                cx.background_executor().timer(TICK).await;
                ticks += 1;
                let next = this.update_in(cx, |this, window, cx| {
                    let Some((code, expires_at)) = this.macs.phone.as_ref().map(|phone| (phone.code.code.clone(), phone.code.expires_at)) else { return None };
                    if expires_at <= chrono::Utc::now().timestamp() {
                        let held = this.holds_phone(window);
                        this.macs.phone = None;
                        this.macs.lapsed = true;
                        if let Some(new) = held.then(|| this.macs.keys.get("phone-new").cloned()).flatten() {
                            window.focus(&new, cx);
                        }
                        cx.notify();
                        return None;
                    }
                    if this.macs_shown(cx) {
                        cx.notify();
                    }
                    let poll = (ticks % POLL_TICKS == 0).then(|| (code, this.wire.phone_pending(cx)));
                    Some(poll)
                });
                let Ok(Some(poll)) = next else { break };
                let Some((code, claims)) = poll else { continue };
                let claims = claims.await;
                let going = this.update_in(cx, |this, _, cx| {
                    let Some(phone) = this.macs.phone.as_mut().filter(|phone| phone.code.code == code) else { return false };
                    match claims {
                        Ok(claims) => {
                            this.macs.refused.remove("pair-phone");
                            phone.claim = claims.into_iter().find(|one: &PhoneClaim| one.code == code).map(|one| one.name);
                        }
                        Err(said) => {
                            this.macs.refused.insert("pair-phone".into(), said);
                        }
                    }
                    cx.notify();
                    true
                });
                if !going.unwrap_or(false) {
                    break;
                }
            }
        }));
    }

    /// One of the phone code's controls holds the keys.
    fn holds_phone(&self, window: &Window) -> bool {
        ["copy-phone-link", "phone-confirm", "phone-decline"].iter().any(|name| self.holds(name, window))
    }

    /// `POST /admin/pair/confirm`. The phone is on the list at once, the panel closes and the keys go to its Unpair.
    fn confirm_phone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(code) = self.macs.phone.as_ref().map(|phone| phone.code.code.clone()) else { return };
        if !self.begin("phone-confirm") {
            return;
        }
        let press = self.wire.phone_confirm(code, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let held = this.holds_phone(window);
                // A refusal says the code is no good any more, or that no phone entered it: either way it is dropped.
                this.macs.phone = None;
                this.macs._phone = None;
                if let Some(phone) = this.answered("phone-confirm", answer) {
                    let unpair = this.macs.keys.entry(format!("unpair:{}", phone.id)).or_insert_with(|| cx.focus_handle().tab_stop(true)).clone();
                    if held {
                        window.focus(&unpair, cx);
                    }
                    if let Some(Ok(phones)) = this.macs.phones.as_mut() {
                        if phones.iter().all(|one| one.id != phone.id) {
                            phones.push(phone);
                        }
                    }
                    this.read_devices(cx);
                } else if let Some(pair) = held.then(|| this.macs.keys.get("pair-phone").cloned()).flatten() {
                    window.focus(&pair, cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Decline: `POST /admin/pair/decline`, and the code, the link and the claim are dropped here whatever it says.
    /// Polling stops; the phone's claim lapses unconfirmed.
    fn decline_phone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(code) = self.macs.phone.take().map(|phone| phone.code.code) else { return };
        self.macs._phone = None;
        self.macs.refused.remove("phone-decline");
        if let Some(pair) = self.macs.keys.get("pair-phone").cloned() {
            window.focus(&pair, cx);
        }
        let press = self.wire.phone_decline(code, cx);
        cx.spawn(async move |this, cx| {
            let answer = press.await;
            let _ = this.update(cx, |this, cx| {
                if let Err(said) = answer {
                    this.macs.refused.insert("phone-decline".into(), said);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
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

    /// `POST /macs/update` on Fleet: the Mac moves onto this Mac's build, or this Mac onto the Mac's. The press says
    /// Updating from the press until the list reads the row linked; a refusal is said under the row, in Fleet's words.
    fn update_mac(&mut self, id: String, this_moves: bool, window: &mut Window, cx: &mut Context<Self>) {
        let act = format!("{}:{id}", if this_moves { "update-this" } else { "update" });
        if self.macs.updating.contains(&id) || !self.begin(&act) {
            return;
        }
        self.macs.updating.insert(id.clone());
        let asked = if this_moves { UpdateMac { like: Some(id.clone()), ..UpdateMac::default() } } else { UpdateMac { mac: Some(id.clone()), ..UpdateMac::default() } };
        let press = self.wire.update_mac(asked, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = press.await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.answered(&act, answer).is_none() {
                    this.macs.updating.remove(&id);
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
                        // A setup that leads opens Cloudflare's sign-in page itself.
                        if let (Tunnel::SignIn(login), Some(_)) = (&made, this.macs.lead) {
                            this.wire.open_link(login, cx);
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
        match self.macs.part() {
            Part::Cards => vec![capped(self.lead_cards(look, window, cx))],
            Part::Steps => vec![capped(self.steps_card(look, window, cx))],
            Part::Waiting => vec![capped(self.waiting_card(look, window, cx))],
            Part::Page => {
                let room = page_room(look, window, cx);
                if room < px(TWO_COLUMNS) {
                    return vec![self.this_mac_card(look), self.tunnel_card(look, window, cx), self.devices_card(look, window, cx)];
                }
                // This Mac and its tunnel stack on the left and the list of Devices fills the right. The page's own
                // stack stops at `PAGE_MAX`, so the pair is as wide as the page and stands out of it.
                let gap = look.length(token::GAP_PANEL);
                let left = div().flex_1().min_w_0().flex().flex_col().gap(gap).child(self.this_mac_card(look)).child(self.tunnel_card(look, window, cx));
                let right = div().flex_1().min_w_0().child(self.devices_card(look, window, cx));
                vec![div().flex_none().w(room).flex().items_start().gap(gap).child(left).child(right).into_any_element()]
            }
        }
    }

    /// First open, nothing set up: the two ways to start, side by side, each one press.
    fn lead_cards(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let one = |id: &'static str, icon: &'static str, title: &'static str, said: &'static str, press: AnyElement| {
            let tile = div().flex_none().size(look.length(token::SPACE_8)).rounded(look.length(token::RADIUS_MD)).bg(look.color(token::ACCENT_FAINT)).flex().items_center().justify_center().child(svg().path(icon_path(icon)).size(look.length(token::SPACE_4)).text_color(look.color(token::ACCENT)));
            let name = div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(title);
            let body = div()
                .id(id)
                .flex_1()
                .min_w(look.length(token::SPACE_12) * 5.)
                .flex()
                .flex_col()
                .items_start()
                .gap(look.length(token::SPACE_4))
                .p(look.length(token::PAD_CARD))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::SURFACE_CARD))
                .child(tile)
                .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(name).child(caption(look, said)))
                .child(press);
            labelled(body, Role::Region, title).into_any_element()
        };
        let heading = div().text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Connect your Macs");
        let cards = div()
            .flex()
            .flex_wrap()
            .gap(look.length(token::SPACE_4))
            .child(one("macs-lead-find", "link", "Pair with a Mac that's set up", "Your other Mac already runs Armada's link", self.macs_button("find", "Find my Mac", Weight::Primary, false, look, window, cx)))
            .child(one("macs-lead-set-up", "monitor", "Set up this Mac first", "The first of your Macs", self.macs_button("set-up", "Set up", Weight::Secondary, false, look, window, cx)));
        labelled(div().id("macs-connect").flex().flex_col().gap(look.length(token::SPACE_4)).child(heading).child(cards), Role::Region, "Connect your Macs").into_any_element()
    }

    /// This Mac's setup, one row a step, each ticking off as the Gateway says it is done.
    fn steps_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let page = &self.macs;
        let state = page.cloudflare();
        let gateway_on = state.is_some();
        let signed = matches!(state, Some(CloudflareState::NoTunnel { .. } | CloudflareState::Running { .. } | CloudflareState::Down { .. }));
        let signing = matches!(state, Some(CloudflareState::SigningIn { .. })) || (gateway_on && !signed && page.tunnel_asked && page.said.is_none());
        let again = || self.macs_button("again", "Try again", Weight::Ghost, false, look, window, cx);
        let mut rows = Vec::new();

        // The Gateway.
        let (mark, name, lines, press) = match (&page.phone_said, gateway_on) {
            (_, true) => (done(look, "macs-step-gateway"), "Gateway on", Vec::new(), None),
            (Some(said), false) => (failed(look, "macs-step-gateway"), "Turn the Gateway on", vec![alert(look, "macs-phone-refused", said)], Some(again())),
            (None, false) => (working(look, "macs-step-gateway"), "Turning the Gateway on", Vec::new(), None),
        };
        rows.push(step_row(look, mark, status_name(look, "macs-step-gateway-name", name), lines, press, true));

        // Cloudflare's sign-in.
        let stuck_here = !signed && page.said.is_some();
        let (mark, name, lines, press) = if signed {
            (done(look, "macs-step-sign-in"), "Signed in to Cloudflare", Vec::new(), None)
        } else if stuck_here {
            (failed(look, "macs-step-sign-in"), "Sign in to Cloudflare", page.said.iter().map(|said| alert(look, "macs-setup-refused", said)).collect(), Some(again()))
        } else if signing {
            let open = matches!(state, Some(CloudflareState::SigningIn { .. })).then(|| self.macs_button("sign-in", "Open it again", Weight::Ghost, false, look, window, cx));
            let lines = open.is_some().then(|| caption(look, "Cloudflare's sign-in page is open in your browser.").into_any_element()).into_iter().collect();
            (working(look, "macs-step-sign-in"), "Signing in to Cloudflare", lines, open)
        } else {
            (not_done(look, "macs-step-sign-in"), "Sign in to Cloudflare", Vec::new(), None)
        };
        rows.push(step_row(look, mark, status_name(look, "macs-step-sign-in-name", name), lines, press, false));

        // The tunnel.
        let (mark, name, lines, press) = match state {
            Some(CloudflareState::Running { address }) => (done(look, "macs-step-tunnel"), "Tunnel made", vec![reach_line(look, "Your other Macs reach this Mac at", address).into_any_element()], None),
            Some(CloudflareState::NoTunnel { why: Some(why) }) => (failed(look, "macs-step-tunnel"), "Make the tunnel", vec![alert(look, "macs-tunnel-why", why)], Some(again())),
            Some(CloudflareState::NoTunnel { why: None }) if page.tunnel_asked => (working(look, "macs-step-tunnel"), "Making the tunnel", Vec::new(), None),
            _ => (not_done(look, "macs-step-tunnel"), "Make the tunnel", Vec::new(), None),
        };
        rows.push(step_row(look, mark, status_name(look, "macs-step-tunnel-name", name), lines, press, false));

        let rows = div().flex().flex_col().children(rows).into_any_element();
        labelled(card(look, "settings-card-setup", "Setting up this Mac", None, vec![rows]), Role::Region, "Setting up this Mac").into_any_element()
    }

    /// This Mac is set up and no other Mac has turned up: the page reads again by itself, and a link pairs them
    /// without the other Mac's tunnel.
    fn waiting_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let page = &self.macs;
        let line = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(status_name(look, "macs-waiting", "Waiting for your other Mac")).child(caption(look, "On it, open Settings › Macs and press Set up."));
        let mut body = vec![div().flex().items_start().gap(look.length(token::SPACE_3)).child(div().flex_none().w(look.length(token::SPACE_4)).pt(look.length(token::SPACE_1)).child(working(look, "macs-waiting-mark"))).child(line).into_any_element()];
        if page.by_link {
            body.push(div().flex().child(self.macs_button("pair", "Make a link", Weight::Ghost, page.out.as_deref() == Some("pair"), look, window, cx)).into_any_element());
            body.push(self.pairing(look, window, cx));
        } else {
            body.push(div().flex().child(self.macs_button("by-link", "Pair with a link instead", Weight::GhostGlyph("link"), false, look, window, cx)).into_any_element());
        }
        labelled(card(look, "settings-card-waiting", "Your other Macs", None, body), Role::Region, "Your other Macs").into_any_element()
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
                    let line = div().id("macs-cloudflared").flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).tooltip(tip("cloudflared version".to_string())).child(SharedString::from(format!("cloudflared {version}")));
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
                let name = div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(address.clone()));
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
                step_row(look, not_done(look, "macs-tunnel-mark"), step_name(look, if funnel.is_some() { "No tunnel for your other Macs yet" } else { "No tunnel yet" }), lines, Some(self.macs_button("setup", "Set up tunnel", Weight::Primary, page.pressing, look, window, cx)), first)
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
        let mut take = |row: gpui::Div, refused: Option<AnyElement>, frozen: Option<(String, String)>, rows: &mut Vec<AnyElement>| {
            let row = div().relative().flex().flex_col().gap(look.length(token::SPACE_1)).child(row).children(refused);
            // A row on another build has an amber edge, and Forget in a menu a right press or Shift+F10 opens.
            let row = match frozen {
                Some((id, name)) => row
                    .child(div().absolute().top_0().bottom_0().left(-look.length(token::SPACE_3)).w(look.length(token::BORDER_WIDTH) * 3.).bg(look.color(token::STATUS_ESCALATED)))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.open_mac_menu(id.clone(), name.clone(), event.position, false, window, cx);
                        }),
                    ),
                None => row,
            };
            rows.push(row.when(!first, |row| row.pt(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE))).pb(look.length(token::SPACE_3)).into_any_element());
            first = false;
        };
        let refused_at = |act: &str| page.refused.get(act).map(|said| alert(look, SharedString::from(format!("macs-{act}-refused")), said));
        match (&page.macs, &page.phones) {
            (None, _) | (_, None) => {
                for _ in 0..3 {
                    take(div().flex().items_center().gap(look.length(token::SPACE_3)).child(bone(look, look.length(token::SPACE_3), look.length(token::SPACE_3))).child(bone(look, look.length(token::SPACE_6) * 5., look.length(token::LEADING_SM))), None, None, &mut rows);
                }
            }
            (Some(macs), Some(phones)) => {
                if let Some(said) = said_of(macs) {
                    rows.push(alert(look, "macs-list-refused", said));
                }
                for mac in macs.iter().flatten() {
                    let mark = mac.link.as_ref().map(|link| {
                        let (name, ink, said) = link_mark(look, link, mac.why.as_deref());
                        glyph(look, SharedString::from(format!("macs-link-{}", mac.id)), name, ink, &said)
                    });
                    let asleep = matches!(mac.link, Some(MacLink::Asleep { .. }));
                    let differs = frozen(mac);
                    let act = press_of_mac(mac).map(|(act, said)| (format!("{act}:{}", mac.id), said));
                    let press = act.as_ref().and_then(|(key, said)| if differs { self.update_button(mac, key, said, look, window, cx) } else { self.row_button(key, said, format!("{said} {}", mac.name), matches!(mac.link, Some(MacLink::ConfirmWaiting)), look, window, cx) });
                    let ink = if differs { token::STATUS_ESCALATED } else { token::FG_MUTED };
                    let row = device_row(look, chip(look, mac), &mac.name, mac.this_mac.then_some("This Mac"), mac.why.as_deref(), asleep)
                        .child(div().flex_none().children(mac.build.as_ref().map(|build| build_mark_in(look, format!("macs-build-{}", mac.id), build, look.color(ink)))))
                        .child(div().flex_none().w(look.length(token::SPACE_6)).flex().justify_center().children(mark))
                        .child(press_column(look, press));
                    take(row, act.and_then(|(key, _)| refused_at(&key)), differs.then(|| (mac.id.clone(), mac.name.clone())), &mut rows);
                }
                // A press refused on a Mac the list no longer draws is said at its foot.
                let drawn = |key: &str| macs.iter().flatten().any(|mac| press_of_mac(mac).is_some_and(|(act, _)| key == format!("{act}:{}", mac.id)));
                for (key, said) in page.refused.iter().filter(|(key, _)| (key.starts_with("confirm:") || key.starts_with("forget:")) && !drawn(key)) {
                    rows.push(alert(look, SharedString::from(format!("macs-{key}-refused")), said));
                }
                if let Some(said) = said_of(phones) {
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
                    take(row, None, None, &mut rows);
                }
            }
        }
        if let Some(said) = &page.unpair_said {
            rows.push(alert(look, "macs-unpair-refused", said));
        }
        let rest = page.keys.get(AT_REST).cloned();
        let list = labelled(div().id("macs-devices").flex().flex_col().when_some(rest, |list, rest| list.track_focus(&rest)).children(rows), Role::List, "Devices");
        let pair = self.macs_button("pair", "Pair a Mac", Weight::Ghost, page.out.as_deref() == Some("pair"), look, window, cx);
        let pair_phone = self.macs_button("pair-phone", "Pair a phone", Weight::Ghost, page.out.as_deref() == Some("pair-phone"), look, window, cx);
        let presses = div().flex().items_center().gap(look.length(token::SPACE_2)).child(pair).child(pair_phone).into_any_element();
        let body = self.phone_panel(look, window, cx).into_iter().chain([self.pairing(look, window, cx), list.into_any_element()]).collect();
        labelled(card(look, "settings-card-devices", "Devices", Some(presses), body), Role::Region, "Devices").into_any_element()
    }

    /// Pair a phone: the QR code and the link its camera opens with the time left, a claim to Confirm or Decline, or
    /// Make a new link once the code has run out. A refusal is said here, in the Gateway's words. With none of these
    /// there is no panel, so the card keeps no gap for it.
    fn phone_panel(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let page = &self.macs;
        let said = |act: &str| page.refused.get(act).filter(|said| *said != NOT_RUNNING).map(|said| alert(look, SharedString::from(format!("macs-{act}-refused")), said));
        let refused = ["pair-phone", "phone-confirm", "phone-decline"].iter().any(|act| page.refused.get(*act).is_some_and(|said| said != NOT_RUNNING));
        if page.phone.is_none() && !page.lapsed && !refused {
            return None;
        }
        let live = page.phone.as_ref().map(|phone| {
            let left = format!("{} left", time_left(phone.code.expires_at - chrono::Utc::now().timestamp()));
            let link = div().id("macs-phone-link").min_w_0().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(phone.link.clone()));
            let figure = div().id("macs-phone-left").flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip("Time left on this link")).child(SharedString::from(left.clone()));
            let facts = div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(caption(look, "Open this on your phone's camera"))
                .child(labelled(link, Role::Label, phone.link.clone()))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(self.macs_button("copy-phone-link", "Copy link", Weight::Ghost, false, look, window, cx))
                        .child(labelled(figure, Role::Label, left)),
                );
            let code = div().flex().items_start().gap(look.length(token::SPACE_4)).child(qr_mark(look, &phone.qr)).child(facts);
            let claim = phone.claim.as_ref().map(|name| {
                let name = format!("{name} wants to pair");
                labelled(
                    div()
                        .id("macs-phone-claim")
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(glyph(look, "macs-phone-claim-mark", "hourglass", look.color(token::STATUS_ESCALATED), "Waiting for Confirm on this Mac"))
                        .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(name.clone())))
                        .child(self.macs_button("phone-confirm", "Confirm", Weight::Primary, page.out.as_deref() == Some("phone-confirm"), look, window, cx))
                        .child(self.macs_button("phone-decline", "Decline", Weight::Ghost, false, look, window, cx)),
                    Role::Group,
                    name,
                )
            });
            div().flex().flex_col().gap(look.length(token::SPACE_3)).child(code).children(claim)
        });
        let lapsed = (page.phone.is_none() && page.lapsed).then(|| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .child(caption(look, "The link has run out"))
                .child(self.macs_button("phone-new", "Make a new link", Weight::Ghost, page.out.as_deref() == Some("pair-phone"), look, window, cx))
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .children(said("pair-phone"))
                .children(live)
                .children(lapsed)
                .children(said("phone-confirm"))
                .children(said("phone-decline"))
                .into_any_element(),
        )
    }

    /// The link Pair a Mac made, to copy with its time left, and the box the other Mac's link is pasted into.
    fn pairing(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let page = &self.macs;
        let refused = |act: &str| page.refused.get(act).map(|said| alert(look, SharedString::from(format!("macs-{act}-refused")), said));
        let made = page.link.as_ref().map(|made| {
            let left = time_left(made.expires_at - chrono::Utc::now().timestamp());
            // The code is at the link's end, so a narrow page wraps it rather than cutting it off.
            let link = div().id("macs-pair-link").flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(made.link.clone()));
            let figure = div().id("macs-pair-left").flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip("Time left on this link")).child(SharedString::from(left.clone()));
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

    /// Update on a row whose build differs: an amber outline on an amber wash, and while it runs, a muted Updating with
    /// a turning mark. Its tooltip says which Mac moves onto which build.
    fn update_button(&self, mac: &Mac, key: &str, said: &str, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let focus = self.macs.keys.get(key)?.clone();
        let going = self.macs.updating.contains(&mac.id);
        let off = going || !self.live_gateway();
        let (name, tip_said) = press_said(mac, said, going);
        let button = div()
            .id(SharedString::from(format!("macs-{key}")))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL_SM))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .when(going, |button| button.border_color(look.color(token::BORDER_DEFAULT)).text_color(look.color(token::FG_MUTED)))
            .when(!going, |button| button.border_color(look.color(token::STATUS_ESCALATED)).bg(look.color(token::STATUS_ESCALATED_BG)).text_color(look.color(if off { token::FG_SUBTLE } else { token::FG_DEFAULT })))
            .when(!off, |button| button.cursor_pointer())
            .when(going, |button| {
                button.child(svg().flex_none().path(icon_path("loader-circle")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)).with_animation(
                    "macs-updating",
                    Animation::new(look.duration(token::DURATION_SPIN)).repeat(),
                    |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
                ))
            })
            .child(SharedString::from(if going { "Updating" } else { said }.to_string()))
            .when_some(tip_said, |button, said| button.tooltip(tip(said)));
        let button = ringed(look, button, &focus, window).track_focus(&focus);
        let key = key.to_string();
        let button = button.when(!off, |button| button.on_click(cx.listener(move |this, _, window, cx| this.press_macs(&key, window, cx))));
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
            Weight::GhostGlyph(glyph) => super::view::ghost_with(look, id, Some(glyph), said, off),
            Weight::Secondary => super::workflow_detail::sunken_button(look, id, said, off),
            Weight::Primary => super::change_band::filled(look, id, said, off),
        };
        let button = ringed(look, button, &focus, window).track_focus(&focus).when(!off, |button| button.on_click(cx.listener(move |this, _, window, cx| this.press_macs(name, window, cx))));
        labelled(button, Role::Button, said).into_any_element()
    }
}

#[derive(Clone, Copy)]
enum Weight {
    Ghost,
    GhostGlyph(&'static str),
    Secondary,
    Primary,
}

/// The QR code: dark modules on a light ground with its quiet zone, as a camera needs in either theme. Each module is
/// a whole number of points, and the square is as wide as they add up to.
///
/// armada-allow-off-contract: a QR code is read by a camera, so it is black on white in both themes and no token is
/// both (Classic's `.armada-phone__qr`, `docs/contracts/design-system.md`).
fn qr_mark(look: &Look, qr: &Qr) -> AnyElement {
    let total = qr.width + QUIET * 2;
    let cell = (QR_SIDE / total as f32).floor().max(1.);
    let side = gpui::px(cell * total as f32);
    let (width, dark) = (qr.width, qr.dark.clone());
    let modules = gpui::canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let cell = bounds.size.width / total as f32;
            for (at, _) in dark.iter().enumerate().filter(|(_, dark)| **dark) {
                let (x, y) = ((at % width + QUIET) as f32, (at / width + QUIET) as f32);
                let origin = gpui::point(bounds.origin.x + cell * x, bounds.origin.y + cell * y);
                window.paint_quad(gpui::fill(gpui::Bounds::new(origin, gpui::size(cell, cell)), gpui::black()));
            }
        },
    )
    .size_full();
    labelled(div().id("macs-phone-qr").flex_none().size(side).rounded(look.length(token::RADIUS_MD)).overflow_hidden().bg(gpui::white()).child(modules), Role::Image, "QR code for the pairing link").into_any_element()
}

/// A read's refusal to say on the page. A Gateway that does not answer is the two cards' to show, never red words.
fn said_of<T>(read: &Result<T, String>) -> Option<&str> {
    read.as_ref().err().map(String::as_str).filter(|said| *said != NOT_RUNNING)
}

/// The page is two columns from this wide, and one column under it.
pub(super) const TWO_COLUMNS: f32 = 840.;

/// How wide the overlay leaves the page: the box is nine tenths of the room, less its padding, its border, the index
/// and the gap between the index and the page.
pub(super) fn page_room(look: &Look, window: &Window, cx: &App) -> Pixels {
    let pad = look.length(token::GAP_PANEL);
    crate::room::of(window, cx).size.width * 0.9 - pad * 3. - look.length(token::BORDER_WIDTH) * 2. - look.length(token::W_SETTINGS_INDEX)
}

/// The setup's parts stand in the settings column's width (`view.rs`'s `PAGE_MAX` on the Settings stack).
fn capped(part: AnyElement) -> AnyElement {
    div().w_full().max_w(gpui::px(720.)).child(part).into_any_element()
}

/// A setup step's name, read out as its state.
fn status_name(look: &Look, id: &'static str, said: &'static str) -> AnyElement {
    labelled(div().id(id).child(step_name(look, said)), Role::Status, said).into_any_element()
}

fn done(look: &Look, id: &'static str) -> AnyElement {
    glyph(look, id, "circle-check", look.color(token::STATUS_COMPLETED_SUCCESS), "Done")
}

fn working(look: &Look, id: &'static str) -> AnyElement {
    glyph(look, id, "loader-circle", look.color(token::STATUS_RUNNING), "Working")
}

fn failed(look: &Look, id: &'static str) -> AnyElement {
    glyph(look, id, "circle-x", look.color(token::STATUS_COMPLETED_FAILED), "Stopped")
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

/// A sentence ending on an address.
fn reach_line(look: &Look, lead: &str, address: &str) -> gpui::Div {
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .text_size(look.length(token::TEXT_SM))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(format!("{lead}\u{a0}")))
        .child(div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(address.to_string())))
}

/// A build's short hash, with a tooltip saying what the figure is.
fn build_mark(look: &Look, id: impl Into<SharedString>, build: &str) -> AnyElement {
    build_mark_in(look, id, build, look.color(token::FG_MUTED))
}

/// A build's hash in the ink given: amber on a Mac whose build differs.
fn build_mark_in(look: &Look, id: impl Into<SharedString>, build: &str, ink: Hsla) -> AnyElement {
    let said = format!("Build {build}");
    let hash = div().id(id.into()).flex_none().text_size(look.length(token::TEXT_2XS)).text_color(ink).tooltip(tip(said.clone())).child(SharedString::from(build.to_string()));
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

/// The column every row's press sits in, the same width down the list: wide enough for Update this Mac.
fn press_column(look: &Look, press: Option<AnyElement>) -> gpui::Div {
    div().flex_none().w(look.length(token::SPACE_12) * 3.).flex().justify_end().children(press)
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

    /// The page on a Gateway whose Mac mini runs an older build and whose Mac Pro runs a newer one.
    fn opened_differing(cx: &mut TestAppContext) -> (Entity<SettingsOverlay>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (overlay, cx) = cx.add_window_view(|window, cx| {
            let mut fleet = MockFleet::new();
            fleet.gateway = super::super::gateway::MockGateway::builds_differ();
            SettingsOverlay::new(Wire::Mock(Box::new(fleet)), window, cx)
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.read_macs(window, cx)));
        cx.run_until_parked();
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.macs_keys(cx)));
        (overlay, cx)
    }

    fn updates(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Vec<UpdateMac> {
        overlay.read_with(cx, |this, _| match &this.wire {
            Wire::Mock(mock) => mock.gateway.updates.clone(),
            Wire::Live(_) => Vec::new(),
        })
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
    fn a_device_seen_under_a_minute_ago_was_seen_just_now() {
        assert_eq!(age_said(0), "just now");
        assert_eq!(age_said(59), "just now");
        assert_eq!(age_said(60), "1m ago");
        assert_eq!(age_said(4 * 60), "4m ago");
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

    #[gpui::test]
    fn an_older_mac_presses_update_with_its_id_and_a_newer_one_update_this_mac_with_like(cx: &mut TestAppContext) {
        let (overlay, cx) = opened_differing(cx);
        let presses: Vec<_> = overlay.read_with(cx, |this, _| this.macs.macs.iter().flatten().flatten().map(|mac| press_of_mac(mac)).collect());
        assert_eq!(presses, [None, Some(("forget", "Forget")), Some(("update", "Update")), Some(("update-this", "Update this Mac"))]);

        press(&overlay, "update:mac-mini", cx);
        assert_eq!(updates(&overlay, cx), [UpdateMac { mac: Some("mac-mini".into()), ..UpdateMac::default() }]);
        assert!(overlay.read_with(cx, |this, _| this.macs.updating.contains("mac-mini")), "the press says Updating once it is sent");
        assert_eq!(refused(&overlay, "update:mac-mini", cx), None);
        cx.update(|_, cx| overlay.update(cx, |this, _| {
            this.open = true;
            this.picked = "bridge:macs";
        }));
        for _ in 0..2 {
            cx.executor().advance_clock(LIST_EVERY);
            cx.run_until_parked();
        }
        assert!(links(&overlay, cx).contains(&("mac-mini".into(), Some(MacLink::Linked))), "the list reads it linked once it is on this Mac's build");
        assert!(overlay.read_with(cx, |this, _| this.macs.updating.is_empty()), "and the press is done");

        press(&overlay, "update-this:mac-pro", cx);
        assert_eq!(updates(&overlay, cx).last(), Some(&UpdateMac { like: Some("mac-pro".into()), ..UpdateMac::default() }));
        assert_eq!(updates(&overlay, cx).len(), 2);
    }

    #[gpui::test]
    fn a_refused_update_is_said_under_the_row_in_fleets_words_and_the_press_comes_back(cx: &mut TestAppContext) {
        let (overlay, cx) = opened_differing(cx);
        let said = "Mac Pro did not answer, so its build is not known. Try again once it is awake.";
        overlay.update(cx, |this, _| {
            if let Wire::Mock(mock) = &mut this.wire {
                mock.gateway.refuse_update = Some(said.into());
            }
        });
        press(&overlay, "update-this:mac-pro", cx);
        assert_eq!(refused(&overlay, "update-this:mac-pro", cx).as_deref(), Some(said));
        assert!(overlay.read_with(cx, |this, _| this.macs.updating.is_empty()), "a refused press is not Updating");
        press(&overlay, "update-this:mac-pro", cx);
        assert_eq!(refused(&overlay, "update-this:mac-pro", cx), None, "a press that goes through clears what was said");
        assert!(overlay.read_with(cx, |this, _| this.macs.updating.contains("mac-pro")));
    }

    #[gpui::test]
    fn forget_on_a_frozen_row_is_one_item_in_a_menu_that_shift_f10_opens(cx: &mut TestAppContext) {
        let (overlay, cx) = opened_differing(cx);
        let held = overlay.read_with(cx, |this, _| this.macs.keys.get("update-this:mac-pro").cloned()).expect("the press has keys");
        cx.update(|window, cx| window.focus(&held, cx));
        let opened = cx.update(|window, cx| overlay.update(cx, |this, cx| this.macs_key(&Keystroke::parse("shift-f10").expect("a key"), window, cx)));
        assert!(opened, "Shift+F10 on the press is taken");
        let labels: Vec<String> = overlay.read_with(cx, |this, cx| {
            let (menu, _) = this.popup.as_ref().expect("the menu is open");
            menu.read(cx).entries.iter().filter_map(|entry| if let Entry::Act { label, .. } = entry { Some(label.to_string()) } else { None }).collect()
        });
        assert_eq!(labels, ["Forget Mac Pro"]);
        let menu = overlay.read_with(cx, |this, _| this.popup.as_ref().map(|(menu, _)| menu.clone())).expect("the menu is open");
        menu.update(cx, |_, cx| cx.emit(Picked::Entry { id: "forget", keep: false }));
        cx.run_until_parked();
        assert!(links(&overlay, cx).iter().all(|(id, _)| id != "mac-pro"), "picking it forgets the Mac");
    }

    fn ticks(seconds: u64, cx: &mut VisualTestContext) {
        for _ in 0..seconds {
            cx.executor().advance_clock(TICK);
            cx.run_until_parked();
        }
    }

    fn phone_claim(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Option<Option<String>> {
        overlay.read_with(cx, |this, _| this.macs.phone.as_ref().map(|phone| phone.claim.clone()))
    }

    fn phone_names(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Vec<String> {
        overlay.read_with(cx, |this, _| this.macs.phones.clone().and_then(Result::ok).unwrap_or_default().into_iter().map(|one| one.name).collect())
    }

    #[gpui::test]
    fn pair_a_phone_draws_its_link_then_a_claim_from_the_second_poll_and_confirm_lists_the_phone(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        assert_eq!(phone_claim(&overlay, cx), None, "nothing is drawn before the press");
        press(&overlay, "pair-phone", cx);
        let (link, width) = overlay.read_with(cx, |this, _| this.macs.phone.as_ref().map(|phone| (phone.link.clone(), phone.qr.width))).expect("Pair a phone makes a link");
        assert!(link.ends_with("/pair?code=a41f09c2"), "{link}");
        assert!(width >= 21, "the QR code has its finder patterns");
        press(&overlay, "copy-phone-link", cx);
        assert_eq!(overlay.read_with(cx, |this, _| match &this.wire {
            Wire::Mock(mock) => mock.handed.last().cloned(),
            Wire::Live(_) => None,
        }), Some(link), "Copy link copies the link");

        ticks(2, cx);
        assert_eq!(phone_claim(&overlay, cx), Some(None), "the first poll finds no claim");
        ticks(2, cx);
        assert_eq!(phone_claim(&overlay, cx), Some(Some("iPhone 15".into())), "the second finds the phone");

        press(&overlay, "phone-confirm", cx);
        assert_eq!(refused(&overlay, "phone-confirm", cx), None);
        assert_eq!(phone_claim(&overlay, cx), None, "the panel closes");
        assert_eq!(phone_names(&overlay, cx), ["iPhone", "iPad", "iPhone 15"], "the phone is in the list");
        assert!(overlay.read_with(cx, |this, _| this.macs.keys.contains_key("unpair:phone-3")), "with its Unpair");
    }

    #[gpui::test]
    fn decline_drops_the_code_and_the_claim_and_the_phone_is_not_listed(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        press(&overlay, "pair-phone", cx);
        ticks(4, cx);
        assert_eq!(phone_claim(&overlay, cx), Some(Some("iPhone 15".into())));
        press(&overlay, "phone-decline", cx);
        assert_eq!(phone_claim(&overlay, cx), None, "the code, the link and the claim are gone");
        assert_eq!(refused(&overlay, "phone-decline", cx), None);
        assert!(overlay.read_with(cx, |this, _| this.macs._phone.is_none()), "polling stops");
        assert_eq!(phone_names(&overlay, cx), ["iPhone", "iPad"], "no phone joined");
        overlay.read_with(cx, |this, _| match &this.wire {
            Wire::Mock(mock) => assert!(mock.gateway.phone_pending_is_empty(), "the Gateway lists the code no more"),
            Wire::Live(_) => {}
        });
    }

    #[gpui::test]
    fn a_decline_the_gateway_refuses_is_said_and_the_code_is_dropped_all_the_same(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        press(&overlay, "pair-phone", cx);
        cx.update(|_, cx| overlay.update(cx, |this, _| {
            if let Wire::Mock(mock) = &mut this.wire {
                mock.gateway.phone_decline("a41f09c2").expect("the code ends once");
            }
        }));
        press(&overlay, "phone-decline", cx);
        assert_eq!(phone_claim(&overlay, cx), None);
        assert_eq!(refused(&overlay, "phone-decline", cx).as_deref(), Some("This code is not good any more."));
    }

    #[gpui::test]
    fn a_link_that_runs_out_goes_and_make_a_new_link_is_offered(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        press(&overlay, "pair-phone", cx);
        cx.update(|_, cx| overlay.update(cx, |this, _| {
            if let Some(phone) = this.macs.phone.as_mut() {
                phone.code.expires_at = chrono::Utc::now().timestamp() - 1;
            }
        }));
        ticks(1, cx);
        assert_eq!(phone_claim(&overlay, cx), None, "the QR code and link go");
        assert!(overlay.read_with(cx, |this, _| this.macs.lapsed));
        assert!(overlay.read_with(cx, |this, _| this.macs.keys.get("phone-new").is_some_and(|new| this.macs_stops().contains(new))), "its press is a stop");
        press(&overlay, "phone-new", cx);
        assert_eq!(phone_claim(&overlay, cx), Some(None), "Make a new link starts again");
        assert!(overlay.read_with(cx, |this, _| !this.macs.lapsed));
    }

    #[gpui::test]
    fn a_claim_from_another_mac_after_the_page_opened_draws_its_confirm(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|_, cx| overlay.update(cx, |this, _| {
            this.open = true;
            this.picked = "bridge:macs";
        }));
        assert!(links(&overlay, cx).iter().all(|(id, _)| id != "mac-office"));
        cx.update(|_, cx| overlay.update(cx, |this, _| {
            if let Wire::Mock(mock) = &mut this.wire {
                mock.gateway.macs.push(Mac { id: "mac-office".into(), name: "Office Mac".into(), link: Some(MacLink::ClaimWaiting), ..Default::default() });
            }
        }));
        cx.executor().advance_clock(LIST_EVERY);
        cx.run_until_parked();
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.macs_keys(cx)));
        assert!(links(&overlay, cx).contains(&("mac-office".into(), Some(MacLink::ClaimWaiting))), "the list is read again with the page open");
        overlay.read_with(cx, |this, _| {
            let office = this.macs.macs.iter().flatten().flatten().find(|mac| mac.id == "mac-office").expect("listed");
            assert_eq!(press_of_mac(office), Some(("confirm", "Confirm")));
            assert!(this.macs.keys.get("confirm:mac-office").is_some_and(|confirm| this.macs_stops().contains(confirm)), "its Confirm is a stop");
        });
    }

    #[test]
    fn the_qr_code_is_square_and_a_link_too_long_for_it_is_refused() {
        let qr = Qr::of("https://studio.example.com/pair?code=a41f09c2").expect("encodes");
        assert_eq!(qr.dark.len(), qr.width * qr.width);
        assert!(qr.dark[0] && qr.dark[qr.width - 1] && qr.dark[qr.width * (qr.width - 1)], "the three finder corners are dark");
        assert!(Qr::of(&"x".repeat(5000)).is_none());
    }
}
