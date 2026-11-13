//! Another of his Macs on the Jobs screens (`docs/concepts/pocket.md`, *Another Mac*).
//!
//! The Phone Gateway lists the Macs (`GET /admin/macs`: this one first, then the paired, then claims
//! waiting). **A Job's `mac` is the id of the Mac it runs on; absent means this one.** The window holds
//! the list once ([`hold`]) and every row, head and menu reads it, as the write-clash marks do.
//!
//! **Nothing is drawn until a second Mac is linked or asleep** ([`shows_chips`]). A Job on another Mac is
//! read here and acted on there: Fleet refuses an act on it until stage 2, so each act is left off with
//! [`off_sentence`], gathered in [`Here::acts_here`].

use std::time::Duration;

use gpui::{div, prelude::*, AnyElement, App, FontWeight, Global, Role, SharedString};
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
    if cx.try_global::<Held>().is_some_and(|held| held.0 == macs) {
        return;
    }
    cx.set_global(Held(macs));
    cx.refresh_windows();
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
    hold(if scenario == "jobs-pairing" { mock_macs(true) } else { Vec::new() }, cx);
}

/// Studio, this Mac, and Air, which fell asleep at 09:12 that morning; or Studio alone.
pub fn mock_macs(second: bool) -> Vec<Mac> {
    let since = chrono::Local::now().date_naive().and_hms_opt(9, 12, 0).and_then(|at| at.and_local_timezone(chrono::Local).single()).map_or(0, |at| at.timestamp());
    let studio = Mac { id: "studio".into(), name: "Studio".into(), colour: MacColour::Teal, this_mac: true, ..Default::default() };
    let air = Mac { id: "air".into(), name: "Air".into(), colour: MacColour::Orange, link: Some(MacLink::Asleep { since }), ..Default::default() };
    if second { vec![studio, air] } else { vec![studio] }
}

/// A walk's scene for the Macs: `one-mac` lists only this one.
pub fn scene(name: &str, cx: &mut App) -> Result<(), String> {
    match name {
        "one-mac" => hold(mock_macs(false), cx),
        "two-macs" => hold(mock_macs(true), cx),
        other => return Err(format!("no pairing scene named {other}")),
    }
    Ok(())
}

/// Whether a second Mac is listed that is linked or asleep. A claim waiting on a Confirm does not count.
pub fn shows_chips(macs: &[Mac]) -> bool {
    macs.iter().any(|one| !one.this_mac && matches!(one.link, Some(MacLink::Linked | MacLink::Asleep { .. })))
}

/// What a Job's chip says: the Mac's name in its colour, and when it fell asleep if it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    pub name: String,
    pub colour: MacColour,
    pub asleep_since: Option<i64>,
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
    /// Whether an act on this Job may go to Fleet from here: only a Job on this Mac. **The one place the
    /// answer is worked out**; every button, menu, key and box asks it.
    fn acts_here(&self) -> bool;
}

impl Here for SessionRecord {
    fn acts_here(&self) -> bool {
        self.mac.is_none()
    }
}

/// Why acts on this Job are off, or `None` where they are on.
pub fn refusal(job: &SessionRecord, cx: &App) -> Option<String> {
    if job.acts_here() { None } else { off_sentence(macs(cx), job.mac.as_deref()) }
}

/// Whether a POST only reads, or opens a window here: it goes on for a Job on another Mac, as the thread and panes do.
pub fn reads(path: &str) -> bool {
    path.ends_with("/read") || path.ends_with("/view") || path.ends_with("/window/open")
}

/// The time the Mac fell asleep, in his clock.
pub fn asleep_said(since: i64) -> String {
    use chrono::{Local, TimeZone};
    let at = Local.timestamp_opt(since, 0).single().map(crate::settings::time::clock).unwrap_or_default();
    format!("Asleep since {at}")
}

/// The chip as a row and a head draw it: the Mac's name in its colour on a wash of it. A Mac that fell asleep is
/// dimmed and says since when; the name is the chip's, so a walk and VoiceOver find it by it.
pub fn pill(look: &Look, id: SharedString, chip: &Chip) -> AnyElement {
    let hue = crate::settings::macs::colour_of(look, chip.colour);
    let drawn = div()
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
        .child(SharedString::from(chip.name.clone()))
        .when_some(chip.asleep_since, |pill, since| {
            let said: SharedString = asleep_said(since).into();
            pill.opacity(0.5).tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx))
        });
    labelled(drawn, Role::Image, SharedString::from(chip.name.clone())).into_any_element()
}

/// A Mac's mark on a Jobs row: a dot in its fixed colour, its name in the tooltip, dimmed while it sleeps.
pub fn dot(look: &Look, id: SharedString, chip: &Chip) -> AnyElement {
    let hue = crate::settings::macs::colour_of(look, chip.colour);
    let said: SharedString = match chip.asleep_since {
        Some(since) => format!("{}: {}", chip.name, asleep_said(since)).into(),
        None => chip.name.clone().into(),
    };
    let drawn = div()
        .id(id)
        .flex_none()
        .size(look.length(token::SPACE_2))
        .rounded_full()
        .bg(hue)
        .when(chip.asleep_since.is_some(), |dot| dot.opacity(0.5))
        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx));
    labelled(drawn, Role::Image, SharedString::from(chip.name.clone())).into_any_element()
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
    fn the_chip_shows_only_once_a_second_mac_is_linked_or_asleep() {
        assert!(!shows_chips(&[this()]), "one Mac: nothing changes");
        let waiting = Mac { id: "x".into(), link: Some(MacLink::ClaimWaiting), ..Default::default() };
        let confirm = Mac { id: "y".into(), link: Some(MacLink::ConfirmWaiting), ..Default::default() };
        assert!(!shows_chips(&[this(), waiting, confirm]), "claims waiting do not count");
        assert!(shows_chips(&[this(), linked("air", "Air", MacColour::Orange)]));
        let asleep = Mac { id: "air".into(), link: Some(MacLink::Asleep { since: 5 }), ..Default::default() };
        assert!(shows_chips(&[this(), asleep]));
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
    fn the_refusal_names_the_mac_the_job_runs_on() {
        let macs = [this(), linked("air", "Air", MacColour::Orange)];
        assert_eq!(off_sentence(&macs, Some("air")).as_deref(), Some("Runs on Air. Act on it there."));
        assert_eq!(off_sentence(&macs, None), None);
        assert_eq!(off_sentence(&macs, Some("gone")).as_deref(), Some("Runs on another Mac. Act on it there."));
    }
}
