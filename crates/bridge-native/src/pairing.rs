//! Another of his Macs on the Jobs screens (`docs/concepts/pocket.md`, *Another Mac*).
//!
//! The Phone Gateway lists the Macs (`GET /admin/macs`: this one first, then the paired, then claims
//! waiting). **A Job's `mac` is the id of the Mac it runs on; absent means this one.** The window holds
//! the list once ([`hold`]) and every row, head and menu reads it, as the write-clash marks do.
//!
//! **Nothing is drawn until a second Mac is linked, asleep or on another build** ([`shows_chips`]). A Job on another Mac is read
//! here, and the acts Fleet carries to that Mac ([`crosses`]: a message, an answer, the plan, Stop, Merge, notes, an answer read, a press, a vote, an archive, a seen mark and an answer to a part on an artifact, an answer on a Ready to try card his build here holds, and the move that brings a Job here) go from
//! here; one sent while that Mac sleeps waits in `crate::waiting_sends`. Every other act on it Fleet refuses, so
//! it is left off with [`off_sentence`], as [`refusal_to`] says for the act's route.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::{div, prelude::*, AnyElement, App, Div, FontWeight, Global, Role, SharedString};
use ipc::{Mac, MacColour, MacLink, SessionRecord};

use crate::labels::labelled;
use crate::look::{token, Look};

/// The Gateway is asked again this often, for a Mac that fell asleep or woke.
const EVERY: Duration = Duration::from_secs(30);

/// Every Mac the Gateway listed, as it last answered. Empty while it does not answer.
#[derive(Default)]
struct Held(Vec<Mac>);

impl Global for Held {}

/// Hand the window the Macs. A Gateway that does not answer hands it none, so no chip stands.
pub fn hold(macs: Vec<Mac>, cx: &mut App) {
    // An update asked of a Mac is over once the list no longer reads it frozen.
    let still: HashSet<String> = macs.iter().filter(|one| frozen_link(one)).map(|one| one.id.clone()).collect();
    if cx.try_global::<Updating>().is_some_and(|going| going.0.iter().any(|id| !still.contains(id))) {
        cx.global_mut::<Updating>().0.retain(|id| still.contains(id));
    }
    // A hidden band returns once its Mac's build changes or its link stops reading older.
    if cx.try_global::<Hidden>().is_some_and(|hidden| !hidden.0.is_empty()) {
        cx.global_mut::<Hidden>().0.retain(|id, build| macs.iter().any(|one| &one.id == id && hideable(one) && &one.build == build));
    }
    if cx.try_global::<Held>().is_some_and(|held| held.0 == macs) {
        return;
    }
    cx.set_global(Held(macs));
    cx.refresh_windows();
}

/// The Macs whose update was asked from the band and has not read linked yet.
#[derive(Default)]
struct Updating(HashSet<String>);

impl Global for Updating {}

/// The older-build bands he hid, by Mac id, with the build that Mac ran when he hid it. Held here and nowhere
/// else: a different build brings the band back.
#[derive(Default)]
struct Hidden(HashMap<String, Option<String>>);

impl Global for Hidden {}

/// Whether the band for this Mac may be hidden: only when that Mac runs an older build. A newer build is this
/// Mac's own update to take, so its band stays.
fn hideable(mac: &Mac) -> bool {
    !mac.this_mac && mac.link == Some(MacLink::OlderBuild)
}

/// The Macs last held.
pub fn macs(cx: &App) -> &[Mac] {
    cx.try_global::<Held>().map_or(&[], |held| held.0.as_slice())
}

/// Ask the Gateway for the Macs now and every half minute. Lives as long as the app.
pub fn follow(cx: &mut App) {
    cx.spawn(async move |cx| loop {
        let read = cx.update(|cx| crate::settings::gateway::macs(cx));
        let answer = read.await;
        cx.update(|cx| hold(answer.unwrap_or_default(), cx));
        cx.background_executor().timer(EVERY).await;
    })
    .detach();
}

/// A walk's Macs: the pairing scenario's two, and none for any other, so one walk never leaves chips for the next.
pub fn begin_mock(scenario: &str, cx: &mut App) {
    crate::move_to_mac::refuse_next(false);
    hold(if matches!(scenario, "jobs-pairing" | "jobs-other-mac") { mock_macs(true) } else { Vec::new() }, cx);
}

/// Studio, this Mac, and Air, which fell asleep at 09:12 that morning; or Studio alone.
pub fn mock_macs(second: bool) -> Vec<Mac> {
    let since = chrono::Local::now().date_naive().and_hms_opt(9, 12, 0).and_then(|at| at.and_local_timezone(chrono::Local).single()).map_or(0, |at| at.timestamp());
    mock_macs_with(second, Some(MacLink::Asleep { since }))
}

/// The same Macs, with Air's link as given.
fn mock_macs_with(second: bool, air_link: Option<MacLink>) -> Vec<Mac> {
    let studio = Mac { id: "studio".into(), name: "Studio".into(), colour: MacColour::Teal, this_mac: true, ..Default::default() };
    let air = Mac { id: "air".into(), name: "Air".into(), colour: MacColour::Orange, link: air_link, ..Default::default() };
    if second { vec![studio, air] } else { vec![studio] }
}

/// A walk's scene for the Macs: `one-mac` lists only this one, `air-awake` wakes Air.
pub fn scene(name: &str, cx: &mut App) -> Result<(), String> {
    match name {
        "one-mac" => hold(mock_macs(false), cx),
        "two-macs" => hold(mock_macs(true), cx),
        "air-awake" => hold(mock_macs_with(true, Some(MacLink::Linked)), cx),
        other => return Err(format!("no pairing scene named {other}")),
    }
    Ok(())
}

/// Whether a second Mac is listed that is linked or asleep. A claim waiting on a Confirm does not count.
pub fn shows_chips(macs: &[Mac]) -> bool {
    macs.iter().any(|one| !one.this_mac && paired(one))
}

/// Whether both Macs hold a key for each other: linked, asleep, or frozen on a build that differs. A frozen Mac's
/// rows stay listed, so its chips and the move stay too, the move off with why.
pub fn paired(mac: &Mac) -> bool {
    matches!(mac.link, Some(MacLink::Linked | MacLink::Asleep { .. } | MacLink::OlderBuild | MacLink::NewerBuild))
}

/// The Mac a Refresh preview takes along: the other Mac, where one is paired, linked first.
pub fn refresh_partner(macs: &[Mac]) -> Option<&Mac> {
    crate::move_to_mac::other(macs)
}

/// What Refresh preview asks of Fleet, **decided here for the title bar's Refresh row, the Ready to try card and
/// the dock's restart key alike**: both Macs onto one preview commit where another is paired, this Mac alone where
/// none is. Two Macs that each refresh alone end on different builds and the link freezes.
pub fn refresh_act(macs: &[Mac]) -> crate::fleet::BuildAct {
    match refresh_partner(macs) {
        Some(other) => crate::fleet::BuildAct::Macs(ipc::UpdateMac { mac: Some(other.id.clone()), this_mac_too: true, build: Some(ipc::BuildSource::Preview), ..Default::default() }),
        None => crate::fleet::BuildAct::Change(ipc::ChangeFleetBuild { build: ipc::BuildSource::Preview, adopt: false, clear: None }),
    }
}

/// Whether the Gateway froze the link for a build that differs.
fn frozen_link(mac: &Mac) -> bool {
    !mac.this_mac && matches!(mac.link, Some(MacLink::OlderBuild | MacLink::NewerBuild))
}

/// A Mac whose link is frozen on a build that differs, and the one press that ends it: the same bodies Settings,
/// Macs sends (`settings/macs.rs` `update_mac`).
#[derive(Clone, Debug, PartialEq)]
pub struct Frozen {
    pub id: String,
    pub name: String,
    pub colour: MacColour,
    /// The Gateway's `why`, or what it would say.
    pub why: String,
    pub label: String,
    pub update: ipc::UpdateMac,
    /// The build the band may be hidden at: only where the other Mac is the older one. `None` is a band that never hides.
    pub hide_at: Option<Option<String>>,
}

/// The first Mac frozen on a build, if any. Asleep and linked Macs give none.
pub fn frozen(macs: &[Mac]) -> Option<Frozen> {
    frozen_unhidden(macs, &HashMap::new())
}

/// The first frozen Mac whose band is not hidden at the build it runs now.
fn frozen_unhidden(macs: &[Mac], hidden: &HashMap<String, Option<String>>) -> Option<Frozen> {
    let mac = macs.iter().find(|one| frozen_link(one) && !(hideable(one) && hidden.get(&one.id) == Some(&one.build)))?;
    let older = mac.link == Some(MacLink::OlderBuild);
    let (label, update) = if older {
        (format!("Update {}", mac.name), ipc::UpdateMac { mac: Some(mac.id.clone()), ..Default::default() })
    } else {
        ("Update this Mac".to_string(), ipc::UpdateMac { like: Some(mac.id.clone()), ..Default::default() })
    };
    let why = mac.why.clone().unwrap_or_else(|| {
        if older {
            format!("{} runs an older build. Nothing crosses until it updates.", mac.name)
        } else {
            format!("{} runs a newer build. Nothing crosses until this Mac updates.", mac.name)
        }
    });
    Some(Frozen { id: mac.id.clone(), name: mac.name.clone(), colour: mac.colour, why, label, update, hide_at: hideable(mac).then(|| mac.build.clone()) })
}

/// Hide the band for a Mac running an older build, until that Mac's build changes.
fn hide_frozen(frozen: &Frozen, cx: &mut App) {
    if let Some(build) = &frozen.hide_at {
        cx.default_global::<Hidden>().0.insert(frozen.id.clone(), build.clone());
        cx.refresh_windows();
    }
}

/// Ask Fleet to end the freeze: `POST /macs/update`. The band says Updating until the list reads the Mac unfrozen;
/// a refusal is a toast in Fleet's words and the press comes back.
pub fn update_frozen(frozen: &Frozen, port: Option<u16>, cx: &mut App) {
    let Some(port) = port else {
        crate::toasts::tell("Fleet is not connected", cx);
        return;
    };
    let body = match ipc::encode(&frozen.update) {
        Ok(body) => body,
        Err(why) => {
            crate::toasts::tell(format!("{why:?}"), cx);
            return;
        }
    };
    let id = frozen.id.clone();
    cx.default_global::<Updating>().0.insert(id.clone());
    cx.refresh_windows();
    cx.spawn(async move |cx| {
        let answer = cx
            .background_executor()
            .spawn(async move {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                runtime.block_on(crate::transport::post(port, "/macs/update", body))
            })
            .await;
        let said = match answer {
            Ok((status, _)) if (200..300).contains(&status) => return,
            Ok((status, bytes)) => crate::snooze::refusal_said(status, &bytes),
            Err(why) => why,
        };
        cx.update(|cx| {
            cx.default_global::<Updating>().0.remove(&id);
            cx.refresh_windows();
            crate::toasts::tell(said, cx);
        });
    })
    .detach();
}

/// The band along the top of the Jobs screen for a Mac frozen on a build that differs: its name, the Gateway's why,
/// and the press that ends the freeze. Nothing while every Mac is linked or asleep.
pub fn frozen_band(look: &Look, macs: &[Mac], port: Option<u16>, cx: &App) -> Option<AnyElement> {
    let hidden = cx.try_global::<Hidden>().map(|hidden| hidden.0.clone()).unwrap_or_default();
    let frozen = frozen_unhidden(macs, &hidden)?;
    let going =cx.try_global::<Updating>().is_some_and(|going| going.0.contains(&frozen.id));
    let ink = look.color(token::STATUS_ESCALATED);
    let hue = crate::settings::macs::colour_of(look, frozen.colour);
    let label = if going { format!("Updating {}", if frozen.update.like.is_some() { "this Mac" } else { frozen.name.as_str() }) } else { frozen.label.clone() };
    let tooltip = match (&frozen.update.like, going) {
        (Some(_), _) => format!("Update this Mac onto {}'s build", frozen.name),
        (None, _) => format!("Update {} onto this Mac's build", frozen.name),
    };
    let button = crate::titlebar::parts::Btn::new("frozen-link-update", label.clone(), crate::titlebar::parts::Weight::Primary).pending(going).disabled(going).draw(look, cx).tooltip(crate::titlebar::parts::tip(tooltip));
    let press = frozen.clone();
    let button = if going {
        button
    } else {
        button.on_click(move |_, _, cx| update_frozen(&press, port, cx))
    };
    let said: SharedString = frozen.why.clone().into();
    Some(
        labelled(
            div()
                .id("frozen-link-band")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .bg(ink.opacity(0.12))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(ink)
                .child(crate::titlebar::parts::glyph("unplug", look.length(token::SPACE_4), ink))
                .child(pill_in(look, "frozen-link-mac".into(), &frozen.name, hue))
                .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(said))
                .child(labelled(button, Role::Button, label))
                .children(frozen.hide_at.is_some().then(|| {
                    let name = frozen.name.clone();
                    let press = frozen.clone();
                    let hide = crate::titlebar::parts::Btn::new("frozen-link-hide", "Hide", crate::titlebar::parts::Weight::Ghost)
                        .draw(look, cx)
                        .tooltip(crate::titlebar::parts::tip(format!("Hide this until {name}'s build changes. {name} offers its own update when you get to it")))
                        .on_click(move |_, _, cx| hide_frozen(&press, cx));
                    labelled(hide, Role::Button, "Hide")
                })),
            Role::Group,
            SharedString::from(format!("{} runs another build", frozen.name)),
        )
        .into_any_element(),
    )
}

/// What a Job's chip says: the Mac's name in its colour, and when it fell asleep if it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    pub name: String,
    pub colour: MacColour,
    pub asleep_since: Option<i64>,
}

/// Why another Mac's Jobs cannot be reached: it is asleep, or its link is frozen on a build that differs. Nothing crosses
/// either way, so what it last said is stale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    Asleep { since: i64 },
    OlderBuild,
    NewerBuild,
}

impl Why {
    /// What the row of that Mac says at its right, and the quiet line of its Jobs.
    pub fn said(self) -> String {
        match self {
            Why::Asleep { since } => asleep_said(since),
            Why::OlderBuild => "On an older build".to_string(),
            Why::NewerBuild => "On a newer build".to_string(),
        }
    }
}

/// Whether this Mac's Jobs cannot be reached, and why. **The one predicate** the list's fold, the needs jumps, the Dock
/// count, the banners and a Job's panel ask. This Mac is never one.
pub fn unreachable(mac: &Mac) -> Option<Why> {
    match mac.link {
        _ if mac.this_mac => None,
        Some(MacLink::Asleep { since }) => Some(Why::Asleep { since }),
        Some(MacLink::OlderBuild) => Some(Why::OlderBuild),
        Some(MacLink::NewerBuild) => Some(Why::NewerBuild),
        _ => None,
    }
}

/// A paired Mac whose Jobs cannot be reached: they stop pulling at him here and stand together at the foot of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unreachable {
    pub id: String,
    pub name: String,
    pub colour: MacColour,
    pub why: Why,
    /// The Mac's own `why`, the tooltip of a frozen link.
    pub tip: Option<String>,
}

/// The Macs whose Jobs cannot be reached, from the ones the window holds.
pub fn unreachable_of(macs: &[Mac]) -> Vec<Unreachable> {
    macs.iter()
        .filter_map(|one| {
            let why = unreachable(one)?;
            let tip = (!matches!(why, Why::Asleep { .. })).then(|| one.why.clone()).flatten();
            Some(Unreachable { id: one.id.clone(), name: one.name.clone(), colour: one.colour, why, tip })
        })
        .collect()
}

/// Whether `mac`, the id a Job's record names (none is this Mac), is a Mac whose Jobs cannot be reached.
pub fn is_unreachable(macs: &[Mac], mac: Option<&str>) -> bool {
    mac.is_some_and(|id| macs.iter().any(|one| one.id == id && unreachable(one).is_some()))
}

/// Call `on` whenever the window's Macs change.
pub fn observe<V: 'static>(cx: &mut gpui::Context<V>, on: impl Fn(&mut V, &mut gpui::Context<V>) + 'static) -> gpui::Subscription {
    cx.observe_global::<Held>(on)
}

/// The chip for a Job on `mac`, where `None` is this Mac. No chip while one Mac is listed, and none for a
/// Mac the list does not hold.
pub fn chip_of(macs: &[Mac], mac: Option<&str>) -> Option<Chip> {
    if !shows_chips(macs) {
        return None;
    }
    let one = match mac {
        None => macs.iter().find(|one| one.this_mac),
        Some(id) => macs.iter().find(|one| one.id == id),
    }?;
    let asleep_since = match one.link {
        Some(MacLink::Asleep { since }) => Some(since),
        _ => None,
    };
    Some(Chip { name: one.name.clone(), colour: one.colour, asleep_since })
}

/// The chip for the Job `mac` names, from the window's Macs.
pub fn chip(mac: Option<&str>, cx: &App) -> Option<Chip> {
    chip_of(macs(cx), mac)
}

/// The sentence on every act left off, and the placeholder of a read-only box.
pub fn off_sentence(macs: &[Mac], mac: Option<&str>) -> Option<String> {
    let id = mac?;
    let name = macs.iter().find(|one| one.id == id).map_or("another Mac", |one| one.name.as_str());
    Some(format!("Runs on {name}. Act on it there."))
}

/// What a Job's record says about where it can be acted on.
pub trait Here {
    /// Whether every act on this Job may go to Fleet from here: only a Job on this Mac. An act Fleet carries to
    /// another Mac goes from here too ([`crosses`]); [`refusal_to`] asks both.
    fn acts_here(&self) -> bool;
}

impl Here for SessionRecord {
    fn acts_here(&self) -> bool {
        self.mac.is_none()
    }
}

/// Why acts on this Job that do not cross to its Mac are off, or `None` where they are on.
pub fn refusal(job: &SessionRecord, cx: &App) -> Option<String> {
    if job.acts_here() { None } else { off_sentence(macs(cx), job.mac.as_deref()) }
}

/// Why the act posted to `path` is off for this Job, or `None` where it goes. **The one place the answer is worked
/// out**; every button, menu, key and box asks it, or [`refusal`] for an act that names no route.
pub fn refusal_to(job: &SessionRecord, path: &str, cx: &App) -> Option<String> {
    if goes(path) { None } else { refusal(job, cx) }
}

/// Whether a POST only reads, or opens a window here: it goes on for a Job on another Mac, as the thread and panes do.
pub fn reads(path: &str) -> bool {
    path.ends_with("/read") || path.ends_with("/view") || path.ends_with("/window/open")
}

/// Whether Fleet carries a POST on a Job another Mac holds to that Mac, or keeps it until that Mac speaks again
/// (`docs/practices/protocol/his-other-macs-fleets-half.md`, *Acting across Macs*). A start and a dispatch cross
/// too, but name their Mac in the body, so they are not acts on a Job held there.
pub fn crosses(path: &str) -> bool {
    const ROUTES: [&str; 19] = [
        "/sessions/message",
        "/sessions/ask/answer",
        "/sessions/waiting/answer",
        "/sessions/waiting/answer_parts",
        "/sessions/plan/approve",
        "/sessions/plan/changes",
        "/sessions/sketch/answer",
        "/sessions/document/answer",
        "/sessions/stop",
        "/sessions/artifacts/press",
        "/sessions/artifacts/vote",
        "/sessions/artifacts/seen",
        "/sessions/artifacts/archive",
        "/sessions/artifacts/restore",
        "/sessions/artifacts/part/answer",
        "/sessions/notes",
        "/sessions/question/read",
        "/sessions/ready_to_try/answer",
        crate::move_to_mac::ROUTE,
    ];
    ROUTES.contains(&path) || (path.starts_with("/pull_requests/") && path.ends_with("/merge"))
}

/// Whether a POST goes to Fleet whichever Mac holds the Job on screen: it reads, it crosses, or it takes back a
/// send this Mac keeps.
pub fn goes(path: &str) -> bool {
    reads(path) || crosses(path) || path == crate::waiting_sends::TAKE_BACK
}

/// The time the Mac fell asleep, in his clock.
pub fn asleep_said(since: i64) -> String {
    use chrono::{Local, TimeZone};
    let at = Local.timestamp_opt(since, 0).single().map(crate::settings::time::clock).unwrap_or_default();
    format!("Asleep since {at}")
}

/// The line a Job's panel shows in place of what it could not read while its Mac cannot be reached, and the Mac's
/// own words for a frozen link as its tooltip: "Air · Asleep since 9:12", "Air · On an older build". None for a Mac that can be.
pub fn unreachable_line_of(mac: Option<&str>, cx: &App) -> Option<(SharedString, Option<SharedString>)> {
    let one = macs(cx).iter().find(|one| Some(one.id.as_str()) == mac)?;
    let why = unreachable(one)?;
    let tip = (!matches!(why, Why::Asleep { .. })).then(|| one.why.clone().map(SharedString::from)).flatten();
    Some((format!("{} \u{b7} {}", one.name, why.said()).into(), tip))
}

/// One muted line in a panel's place, named for VoiceOver by what it says.
pub fn quiet_line(look: &Look, said: SharedString, tip: Option<SharedString>) -> AnyElement {
    labelled(
        div()
            .id("asleep-line")
            .size_full()
            .p(look.length(token::SPACE_4))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(said.clone())
            .when_some(tip, |line, tip| line.tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tip.clone()).build(window, cx))),
        Role::Status,
        said,
    )
    .into_any_element()
}

/// The chip as a row and a head draw it: the Mac's name in its colour on a wash of it. A Mac that fell asleep is
/// dimmed and says since when; the name is the chip's, so a walk and VoiceOver find it by it.
pub fn pill(look: &Look, id: SharedString, chip: &Chip) -> AnyElement {
    let hue = crate::settings::macs::colour_of(look, chip.colour);
    let drawn = pill_drawn(look, id, &chip.name, hue)
        .when_some(chip.asleep_since, |pill, since| {
            let said: SharedString = asleep_said(since).into();
            pill.opacity(0.5).tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx))
        });
    labelled(drawn, Role::Image, SharedString::from(chip.name.clone())).into_any_element()
}

/// A chip for a Mac the window may not list, such as a teammate's, in the hue given.
pub fn pill_in(look: &Look, id: SharedString, name: &str, hue: gpui::Hsla) -> AnyElement {
    labelled(pill_drawn(look, id, name, hue), Role::Image, SharedString::from(name.to_string())).into_any_element()
}

/// A Mac's chip beside a name that matters more: it gives way to the name, stops at a width of its own, ends in an
/// ellipsis, and the whole Mac name is its tooltip.
pub fn pill_capped(look: &Look, id: SharedString, name: &str, hue: gpui::Hsla) -> AnyElement {
    let said = SharedString::from(name.to_string());
    div()
        .id(SharedString::from(format!("{id}-slot")))
        .flex()
        .min_w_0()
        .max_w(look.length(token::SPACE_12) * 2.25)
        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx))
        .child(pill_in(look, id, name, hue))
        .into_any_element()
}

fn pill_drawn(look: &Look, id: SharedString, name: &str, hue: gpui::Hsla) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .max_w_full()
        .truncate()
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .bg(hue.opacity(0.16))
        .text_color(hue)
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .child(SharedString::from(name.to_string()))
}

/// The Mac's dot in its fixed colour, dimmed while it sleeps.
fn swatch(look: &Look, chip: &Chip) -> Div {
    div()
        .flex_none()
        .size(look.length(token::SPACE_2))
        .rounded_full()
        .bg(crate::settings::macs::colour_of(look, chip.colour))
        .when(chip.asleep_since.is_some(), |dot| dot.opacity(0.5))
}

/// A Mac's mark on a Jobs row: a dot in its fixed colour, dimmed while it sleeps. It carries no tooltip of its own:
/// the row's hover card says the Mac and its sleep, and hovering the dot opens that card.
pub fn dot(look: &Look, id: SharedString, chip: &Chip) -> AnyElement {
    labelled(swatch(look, chip).id(id), Role::Image, SharedString::from(chip.name.clone())).into_any_element()
}

/// The Mac's line in a row's hover card: the dot, then the name, and since when it sleeps.
pub fn card_line(look: &Look, chip: &Chip) -> Div {
    let said = match chip.asleep_since {
        Some(since) => format!("{} \u{b7} {}", chip.name, asleep_said(since)),
        None => chip.name.clone(),
    };
    div().flex().items_center().gap(look.length(token::SPACE_2)).child(swatch(look, chip)).child(SharedString::from(said))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linked(id: &str, name: &str, colour: MacColour) -> Mac {
        Mac { id: id.into(), name: name.into(), colour, link: Some(MacLink::Linked), ..Default::default() }
    }

    fn this() -> Mac {
        Mac { id: "studio".into(), name: "Studio".into(), colour: MacColour::Teal, this_mac: true, ..Default::default() }
    }

    #[test]
    fn refresh_preview_moves_both_macs_where_another_is_paired_and_this_one_where_none_is() {
        use crate::fleet::BuildAct;
        let alone = BuildAct::Change(ipc::ChangeFleetBuild { build: ipc::BuildSource::Preview, adopt: false, clear: None });
        assert_eq!(refresh_act(&[this()]), alone);
        let waiting = Mac { id: "x".into(), link: Some(MacLink::ClaimWaiting), ..Default::default() };
        assert_eq!(refresh_act(&[this(), waiting]), alone, "a claim waiting is no Mac to take along");
        let both = BuildAct::Macs(ipc::UpdateMac { mac: Some("air".into()), this_mac_too: true, build: Some(ipc::BuildSource::Preview), ..Default::default() });
        assert_eq!(refresh_act(&[this(), linked("air", "Air", MacColour::Orange)]), both);
        let asleep = Mac { id: "air".into(), link: Some(MacLink::Asleep { since: 5 }), ..Default::default() };
        assert_eq!(refresh_act(&[this(), asleep]), both);
        assert_eq!(refresh_act(&[]), alone);
    }

    #[test]
    fn the_both_form_is_the_wire_body_fleet_reads() {
        let both = refresh_act(&[this(), linked("air", "Air", MacColour::Orange)]);
        assert_eq!(both.route(), "/macs/update");
        assert_eq!(both.body().expect("it has a body").expect("it encodes"), r#"{"mac":"air","this_mac_too":true,"build":"preview"}"#);
    }

    fn frozen_mac(link: MacLink, why: Option<&str>) -> Mac {
        Mac { id: "air".into(), name: "Air".into(), colour: MacColour::Orange, link: Some(link), why: why.map(Into::into), ..Default::default() }
    }

    #[test]
    fn a_frozen_mac_offers_the_update_settings_sends() {
        let older = frozen(&[this(), frozen_mac(MacLink::OlderBuild, Some("Air runs an older build."))]).expect("an older build is frozen");
        assert_eq!((older.label.as_str(), older.why.as_str()), ("Update Air", "Air runs an older build."));
        assert_eq!(older.update, ipc::UpdateMac { mac: Some("air".into()), ..Default::default() });
        let newer = frozen(&[this(), frozen_mac(MacLink::NewerBuild, None)]).expect("a newer build is frozen");
        assert_eq!(newer.label, "Update this Mac");
        assert_eq!(newer.update, ipc::UpdateMac { like: Some("air".into()), ..Default::default() });
        assert!(newer.why.contains("Air runs a newer build"), "a Gateway that says nothing gets the sentence");
    }

    #[test]
    fn a_linked_or_sleeping_mac_is_not_frozen() {
        assert_eq!(frozen(&[this(), linked("air", "Air", MacColour::Orange)]), None);
        assert_eq!(frozen(&[this(), frozen_mac(MacLink::Asleep { since: 5 }, None)]), None);
        assert_eq!(frozen(&[this()]), None);
    }

    fn with_build(link: MacLink, build: &str) -> Mac {
        Mac { build: Some(build.into()), ..frozen_mac(link, None) }
    }

    fn hidden_at(build: &str) -> HashMap<String, Option<String>> {
        HashMap::from([("air".to_string(), Some(build.to_string()))])
    }

    #[test]
    fn an_older_build_band_stays_hidden_until_the_other_macs_build_changes() {
        let macs = [this(), with_build(MacLink::OlderBuild, "aaa")];
        assert!(frozen_unhidden(&macs, &HashMap::new()).is_some_and(|band| band.hide_at == Some(Some("aaa".into()))), "it can be hidden at the build it runs");
        assert_eq!(frozen_unhidden(&macs, &hidden_at("aaa")), None, "hidden at that build, it stays gone");
        let moved = [this(), with_build(MacLink::OlderBuild, "bbb")];
        assert!(frozen_unhidden(&moved, &hidden_at("aaa")).is_some(), "a different build brings it back");
    }

    #[test]
    fn a_band_for_this_mac_being_older_never_hides() {
        let macs = [this(), with_build(MacLink::NewerBuild, "aaa")];
        let band = frozen_unhidden(&macs, &HashMap::new()).expect("a newer build is frozen");
        assert_eq!((band.label.as_str(), band.hide_at), ("Update this Mac", None), "no hide on this Mac's own update");
        assert!(frozen_unhidden(&macs, &hidden_at("aaa")).is_some(), "even a stale hide does not remove it");
    }

    #[gpui::test]
    fn the_hide_press_removes_the_band_and_a_new_build_returns_it(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        cx.update(|cx| hold(vec![this(), with_build(MacLink::OlderBuild, "aaa")], cx));
        let (_probe, cx) = cx.add_window_view(|_, _| Probe);
        assert!(drawn(cx, Role::Button, "Hide"), "an older build offers Hide");
        let band = frozen(&[this(), with_build(MacLink::OlderBuild, "aaa")]).expect("frozen");
        cx.update(|_, cx| hide_frozen(&band, cx));
        // The label book keeps what an earlier frame drew, so ask the band itself.
        let has_band = |cx: &mut gpui::VisualTestContext| {
            cx.update(|_, cx| {
                use crate::look::ActiveLook;
                let look = cx.look().clone();
                frozen_band(&look, &macs(cx).to_vec(), None, cx).is_some()
            })
        };
        assert!(!has_band(cx), "hidden");
        cx.update(|_, cx| hold(vec![this(), with_build(MacLink::OlderBuild, "bbb")], cx));
        assert!(has_band(cx), "a new build brings it back");
    }

    #[gpui::test]
    fn the_newer_build_band_draws_no_hide(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        cx.update(|cx| hold(vec![this(), with_build(MacLink::NewerBuild, "aaa")], cx));
        let (_probe, cx) = cx.add_window_view(|_, _| Probe);
        assert!(drawn(cx, Role::Button, "Update this Mac"));
        assert!(!drawn(cx, Role::Button, "Hide"));
    }

    /// A parent that draws the band as the Jobs screen does.
    struct Probe;

    impl gpui::Render for Probe {
        fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
            use crate::look::ActiveLook;
            let look = cx.look();
            div().size_full().children(frozen_band(look, macs(cx), None, cx))
        }
    }

    fn drawn(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some()))
    }

    #[gpui::test]
    fn the_band_draws_for_a_frozen_mac_with_its_why_and_one_press_and_not_for_a_linked_one(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        cx.update(|cx| hold(vec![this(), frozen_mac(MacLink::OlderBuild, Some("Air runs an older build."))], cx));
        let (_probe, cx) = cx.add_window_view(|_, _| Probe);
        assert!(drawn(cx, Role::Group, "Air runs another build"), "the band is drawn");
        assert!(drawn(cx, Role::Button, "Update Air"), "with the one press");
    }

    #[gpui::test]
    fn no_band_draws_while_every_mac_is_linked(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        cx.update(|cx| hold(vec![this(), linked("air", "Air", MacColour::Orange)], cx));
        let (_probe, cx) = cx.add_window_view(|_, _| Probe);
        assert!(!drawn(cx, Role::Group, "Air runs another build"));
        assert!(!drawn(cx, Role::Button, "Update Air"));
    }

    #[test]
    fn the_chip_shows_only_once_a_second_mac_is_linked_or_asleep() {
        assert!(!shows_chips(&[this()]), "one Mac: nothing changes");
        let waiting = Mac { id: "x".into(), link: Some(MacLink::ClaimWaiting), ..Default::default() };
        let confirm = Mac { id: "y".into(), link: Some(MacLink::ConfirmWaiting), ..Default::default() };
        assert!(!shows_chips(&[this(), waiting, confirm]), "claims waiting do not count");
        assert!(shows_chips(&[this(), linked("air", "Air", MacColour::Orange)]));
        let asleep = Mac { id: "air".into(), link: Some(MacLink::Asleep { since: 5 }), ..Default::default() };
        assert!(shows_chips(&[this(), asleep]));
        for link in [MacLink::NewerBuild, MacLink::OlderBuild] {
            let frozen = Mac { id: "air".into(), link: Some(link), ..Default::default() };
            assert!(shows_chips(&[this(), frozen]), "a Mac on another build is still paired, and its rows still listed");
        }
    }

    #[test]
    fn a_job_with_no_mac_takes_this_macs_name_and_colour() {
        let macs = [this(), linked("air", "Air", MacColour::Orange)];
        assert_eq!(chip_of(&macs, None), Some(Chip { name: "Studio".into(), colour: MacColour::Teal, asleep_since: None }));
        assert_eq!(chip_of(&macs, Some("air")), Some(Chip { name: "Air".into(), colour: MacColour::Orange, asleep_since: None }));
        assert_eq!(chip_of(&macs, Some("gone")), None, "a Mac the list does not hold has no chip");
        assert_eq!(chip_of(&[this()], None), None);
    }

    #[test]
    fn an_asleep_macs_chip_carries_when_it_fell_asleep() {
        let asleep = Mac { id: "air".into(), name: "Air".into(), colour: MacColour::Orange, link: Some(MacLink::Asleep { since: 99 }), ..Default::default() };
        assert_eq!(chip_of(&[this(), asleep], Some("air")).and_then(|chip| chip.asleep_since), Some(99));
    }

    #[test]
    fn acts_are_here_only_for_a_job_with_no_mac() {
        let mut job = crate::mock::jobs("jobs-many").expect("the fixture decodes").remove(0);
        job.mac = None;
        assert!(job.acts_here());
        job.mac = Some("air".into());
        assert!(!job.acts_here());
    }

    #[test]
    fn a_post_that_only_reads_is_not_an_act() {
        assert!(reads("/sessions/judge/read"));
        assert!(reads("/sessions/window/open"));
        assert!(!reads("/sessions/message"));
        assert!(!reads("/sessions/waiting/answer"));
    }

    #[test]
    fn the_acts_fleet_carries_to_another_mac_cross_and_no_other() {
        for path in [
            "/sessions/message",
            "/sessions/ask/answer",
            "/sessions/waiting/answer",
            "/sessions/waiting/answer_parts",
            "/sessions/plan/approve",
            "/sessions/plan/changes",
            "/sessions/sketch/answer",
            "/sessions/document/answer",
            "/sessions/stop",
            "/sessions/artifacts/press",
            "/sessions/artifacts/vote",
            "/sessions/artifacts/seen",
            "/sessions/artifacts/archive",
            "/sessions/artifacts/restore",
            "/sessions/artifacts/part/answer",
            "/sessions/notes",
            "/sessions/question/read",
            "/sessions/ready_to_try/answer",
            "/pull_requests/armada/3548/merge",
        ] {
            assert!(crosses(path), "{path} crosses");
        }
        for path in ["/sessions/pause", "/sessions/snooze", "/sessions/rename", "/sessions/close", "/sessions/judge", "/sessions/waiting/dismiss", "/sessions/tune", "/sessions/start", "/sessions/dispatch", "/pull_requests/armada/3548/refresh", "/sessions/message/edit", "/sessions/artifacts/edit"] {
            assert!(!crosses(path), "{path} stays");
        }
        assert!(goes("/sessions/stop") && goes("/sessions/judge/read") && goes("/queued/take_back"));
        assert!(!goes("/sessions/pause"));
    }

    #[test]
    fn the_refusal_names_the_mac_the_job_runs_on() {
        let macs = [this(), linked("air", "Air", MacColour::Orange)];
        assert_eq!(off_sentence(&macs, Some("air")).as_deref(), Some("Runs on Air. Act on it there."));
        assert_eq!(off_sentence(&macs, None), None);
        assert_eq!(off_sentence(&macs, Some("gone")).as_deref(), Some("Runs on another Mac. Act on it there."));
    }
}
