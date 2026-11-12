//! Settings › Macs: this Mac, its Cloudflare tunnel, and every Mac and phone that reaches it, read off the Phone
//! Gateway (`gateway.rs`). The tunnel's rows are drawn as Classic's Funnel rows are (`PhonePairing`'s `WayIn`).
//!
//! **He opens it cold, with no walk** (the owner, 12 Nov). Each state of the tunnel shows the one press that moves
//! it on, and why where there is a why; nothing tells him to go and do something outside Bridge, and nothing asks
//! him to type. Cloudflare's sign-in page opens only when he presses for it.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{div, prelude::*, svg, AnyElement, App, Context, FocusHandle, FontWeight, Hsla, Keystroke, Role, SharedString, Task, Window};
use ipc::{CloudflareState, GatewaySetup, Mac, MacColour, MacLink, PairedPhone};

use super::entries::Body;
use super::gateway::Tunnel;
use super::overlay::SettingsOverlay;
use super::view::{bone, card, ghost, mono, ringed, small, tip};
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
    keys: BTreeMap<String, FocusHandle>,
    _poll: Option<Task<()>>,
    _reads: Vec<Task<()>>,
}

/// Where the keys wait while the control they were on goes: the Devices list itself.
const AT_REST: &str = "devices";

impl MacsPage {
    pub fn new() -> MacsPage {
        MacsPage { setup: None, macs: None, phones: None, asked: false, pressing: false, said: None, unpair_said: None, keys: BTreeMap::new(), _poll: None, _reads: Vec::new() }
    }

    /// Settings opened again: the Gateway is asked again the next time the page is drawn.
    pub fn reopen(&mut self) {
        self.asked = false;
        self.said = None;
        self.unpair_said = None;
    }

    fn cloudflare(&self) -> Option<&CloudflareState> {
        self.setup.as_ref().and_then(|setup| setup.as_ref().ok()).map(|setup| &setup.cloudflare)
    }
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

/// A Unix time as the page says it: the clock today, else the day and the clock.
fn clock_of(at: i64) -> String {
    use chrono::{Local, TimeZone};
    let Some(at) = Local.timestamp_opt(at, 0).single() else { return String::new() };
    if at.date_naive() == Local::now().date_naive() { at.format("%H:%M").to_string() } else { at.format("%-d %b %H:%M").to_string() }
}

fn age_of(at: i64) -> String {
    format!("{} ago", crate::merge_strip::words::span_said((chrono::Utc::now().timestamp() - at).max(0)))
}

fn colour_of(look: &Look, colour: MacColour) -> Hsla {
    look.color(match colour {
        MacColour::Blue => token::SKETCH_BLUE,
        MacColour::Teal => token::SKETCH_TEAL,
        MacColour::Green => token::SKETCH_GREEN,
        MacColour::Orange => token::SKETCH_ORANGE,
    })
}

/// How a link stands, as its glyph, its ink and its tooltip.
fn link_mark(look: &Look, link: &MacLink) -> (&'static str, Hsla, String) {
    match link {
        MacLink::Linked => ("link", look.color(token::STATUS_COMPLETED_SUCCESS), "Linked".into()),
        MacLink::Asleep { since } => ("moon", look.color(token::FG_MUTED), format!("Asleep since {}", clock_of(*since))),
        MacLink::OlderBuild => ("arrow-down-to-line", look.color(token::STATUS_ESCALATED), "Runs an older build. Linked again once it updates.".into()),
        MacLink::NewerBuild => ("arrow-up-from-line", look.color(token::STATUS_ESCALATED), "Runs a newer build. Linked again once this Mac updates.".into()),
        MacLink::ClaimWaiting => ("hourglass", look.color(token::STATUS_ESCALATED), "Waiting for Confirm on this Mac".into()),
    }
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
        let mut names: Vec<String> = ["setup", "sign-in", "copy"].into_iter().map(String::from).collect();
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
        stops.extend(page.cloudflare().and_then(press_of).and_then(key));
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
                if let Some(CloudflareState::Running { address }) = self.macs.cloudflare() {
                    let address = address.clone();
                    self.wire.copy(&address, cx);
                }
            }
            _ => {
                if let Some(id) = name.strip_prefix("unpair:") {
                    self.unpair(id.to_string(), window, cx);
                }
            }
        }
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
                body.extend(self.tunnel_rows(&setup.cloudflare, look, window, cx));
                if let Some(version) = &setup.cloudflared {
                    let line = div().id("macs-cloudflared").flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).tooltip(tip("cloudflared version".to_string())).child(SharedString::from(format!("cloudflared {version}")));
                    body.push(div().flex().child(labelled(line, Role::Label, "cloudflared version")).into_any_element());
                }
            }
        }
        let rows = div().flex().flex_col().gap(look.length(token::SPACE_3)).children(body).into_any_element();
        labelled(card(look, "settings-card-tunnel", "Tunnel", None, vec![rows]), Role::Region, "Tunnel").into_any_element()
    }

    fn tunnel_rows(&self, state: &CloudflareState, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        let page = &self.macs;
        let refused: Vec<AnyElement> = page.said.iter().map(|said| alert(look, "macs-setup-refused", said)).collect();
        let row = match state {
            CloudflareState::Running { address } => {
                let line = div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child("Phones and your other Macs reach this Mac at\u{a0}")
                    .child(div().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(address.clone())));
                let line = labelled(div().id("macs-running").child(line), Role::Status, format!("Reached at {address}"));
                step_row(look, glyph(look, "macs-running-mark", "check", look.color(token::STATUS_COMPLETED_SUCCESS), "Connected"), line.into_any_element(), Vec::new(), Some(self.macs_button("copy", "Copy address", Weight::Ghost, false, look, window, cx)), true)
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
                step_row(look, glyph(look, "macs-signing-mark", "hourglass", look.color(token::STATUS_ESCALATED), "Waiting for the sign-in"), step_name(look, "Signing in to Cloudflare"), lines, Some(self.macs_button("sign-in", "Open Cloudflare sign-in", Weight::Primary, false, look, window, cx)), true)
            }
            CloudflareState::Missing | CloudflareState::SignedOut | CloudflareState::NoTunnel { .. } => {
                let mut lines = vec![caption(look, "Phones and your other Macs reach this Mac through it.").into_any_element()];
                // The last try's failure, as Fleet wrote it, with Set up tunnel beside it.
                if let CloudflareState::NoTunnel { why: Some(why) } = state {
                    lines.push(alert(look, "macs-tunnel-why", why));
                }
                lines.extend(refused);
                step_row(look, not_done(look, "macs-tunnel-mark"), step_name(look, "No tunnel yet"), lines, Some(self.macs_button("setup", "Set up tunnel", Weight::Primary, page.pressing, look, window, cx)), true)
            }
        };
        vec![row]
    }

    fn devices_card(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let page = &self.macs;
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut first = true;
        let mut take = |row: gpui::Div, rows: &mut Vec<AnyElement>| {
            rows.push(row.when(!first, |row| row.pt(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE))).pb(look.length(token::SPACE_3)).into_any_element());
            first = false;
        };
        match (&page.macs, &page.phones) {
            (None, _) | (_, None) => {
                for _ in 0..3 {
                    take(div().flex().items_center().gap(look.length(token::SPACE_3)).child(bone(look, look.length(token::SPACE_3), look.length(token::SPACE_3))).child(bone(look, look.length(token::SPACE_6) * 5., look.length(token::LEADING_SM))), &mut rows);
                }
            }
            (Some(macs), Some(phones)) => {
                if let Err(said) = macs {
                    rows.push(alert(look, "macs-list-refused", said));
                }
                for mac in macs.iter().flatten() {
                    let mark = mac.link.as_ref().map(|link| {
                        let (name, ink, said) = link_mark(look, link);
                        glyph(look, SharedString::from(format!("macs-link-{}", mac.id)), name, ink, &said)
                    });
                    let row = device_row(look, chip(look, mac), &mac.name, mac.this_mac.then_some("This Mac"))
                        .children(mac.build.as_ref().map(|build| build_mark(look, format!("macs-build-{}", mac.id), build)))
                        .child(div().flex_none().w(look.length(token::SPACE_6)).flex().justify_center().children(mark));
                    take(row, &mut rows);
                }
                if let Err(said) = phones {
                    rows.push(alert(look, "macs-phones-refused", said));
                }
                for phone in phones.iter().flatten() {
                    let seen = phone.last_seen_at.map_or("Never seen".to_string(), |at| format!("Seen {}", age_of(at)));
                    let icon = svg().flex_none().path(icon_path("smartphone")).size(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 2.).text_color(look.color(token::FG_MUTED)).into_any_element();
                    let unpair = self.unpair_button(phone, look, window, cx);
                    let row = device_row(look, icon, &phone.name, None).child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(seen))).children(unpair);
                    take(row, &mut rows);
                }
            }
        }
        if let Some(said) = &page.unpair_said {
            rows.push(alert(look, "macs-unpair-refused", said));
        }
        let rest = page.keys.get(AT_REST).cloned();
        let list = labelled(div().id("macs-devices").flex().flex_col().when_some(rest, |list, rest| list.track_focus(&rest)).children(rows), Role::List, "Devices");
        labelled(card(look, "settings-card-devices", "Devices", None, vec![list.into_any_element()]), Role::Region, "Devices").into_any_element()
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

/// A build's short hash, with a tooltip saying what the figure is.
fn build_mark(look: &Look, id: impl Into<SharedString>, build: &str) -> AnyElement {
    let said = format!("Build {build}");
    let hash = div().id(id.into()).flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip(said.clone())).child(SharedString::from(build.to_string()));
    labelled(hash, Role::Label, said).into_any_element()
}

fn not_done(look: &Look, id: &'static str) -> AnyElement {
    glyph(look, id, "circle-dashed", look.color(token::FG_SUBTLE), "Not done yet")
}

fn alert(look: &Look, id: &'static str, said: &str) -> AnyElement {
    labelled(small(look, said.to_string(), look.color(token::STATUS_COMPLETED_FAILED)).id(id), Role::Alert, said.to_string()).into_any_element()
}

/// A Mac's colour, as a small filled square beside its name.
fn chip(look: &Look, mac: &Mac) -> AnyElement {
    let said = format!("{} colour", mac.colour.name());
    labelled(div().id(SharedString::from(format!("macs-chip-{}", mac.id))).flex_none().size(look.length(token::SPACE_3)).rounded(look.length(token::RADIUS_SM)).bg(colour_of(look, mac.colour)), Role::Image, said).into_any_element()
}

/// A Devices row: its mark, its name, and a quiet word after it; the caller adds what goes at the right.
fn device_row(look: &Look, mark: AnyElement, name: &str, aside: Option<&'static str>) -> gpui::Div {
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
                .items_baseline()
                .gap(look.length(token::SPACE_2))
                .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(name.to_string())))
                .children(aside.map(|aside| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(aside))),
        )
}
