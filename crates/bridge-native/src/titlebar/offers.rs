//! The Refresh row of the Fleet build switcher: the branches Jobs offered for the preview, each named by its
//! Job's title and its line on what it holds, with where Fleet's check of it stands. The words are worked out
//! here from `ipc::PreviewOffers`; the row draws them.

use gpui::{div, prelude::*, px, AnyElement, Context, Role, SharedString, Window};
use ipc::{PreviewOffers, PreviewState};

use super::face::Build;
use super::parts::{glyph, sans, spinner, tip, Btn, Weight};
use super::{Asked, TitleBar};
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// One offer as the row draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferLine {
    /// The Job's title; the branch only where Fleet holds no offer for it, or the offer has no title.
    pub name: String,
    /// The Job's line on what the branch holds.
    pub about: Option<String>,
    /// Where the check stands. Absent for a branch Fleet holds no offer for.
    pub state: Option<PreviewState>,
    /// What failed it, on a failed offer.
    pub why: Option<String>,
}

/// What the Refresh row draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferLines {
    /// The branches Refresh brings in, in `refresh.branches` order, named by their offers. They stand above the buttons.
    pub named: Vec<OfferLine>,
    /// The offers the running build does not hold that are still being checked or failed, in offer order. They stand
    /// below the buttons, so the row says why a Job's branch is not in the set.
    pub waiting: Vec<OfferLine>,
}

/// The lines the Refresh row draws, or none while Fleet says no row.
pub fn lines(offers: &PreviewOffers) -> Option<OfferLines> {
    let refresh = offers.refresh.as_ref()?;
    let line = |branch: &str| -> OfferLine {
        match offers.offers.iter().find(|offer| offer.branch == branch) {
            Some(offer) => OfferLine {
                name: if offer.title.trim().is_empty() { offer.branch.clone() } else { offer.title.clone() },
                about: Some(offer.about.clone()).filter(|about| !about.trim().is_empty()),
                state: Some(offer.state),
                why: offer.why.clone(),
            },
            None => OfferLine { name: branch.to_string(), about: None, state: None, why: None },
        }
    };
    let named = refresh.branches.iter().map(|branch| line(branch)).collect();
    let waiting = offers
        .offers
        .iter()
        .filter(|offer| !offer.in_build && offer.state != PreviewState::Ready && !refresh.branches.contains(&offer.branch))
        .map(|offer| line(&offer.branch))
        .collect();
    Some(OfferLines { named, waiting })
}

/// Whether the switcher's own restart button stays: not while the Refresh row shows and that button would also
/// read Refresh preview, so one button of that name is drawn. Update to main stays.
pub fn restart_stays(held: &Build, row_shows: bool) -> bool {
    !(row_shows && held.restart().target == ipc::BuildSource::Preview)
}

/// What a state glyph's tooltip says: the state, and on a failed offer what failed it.
pub fn state_said(state: PreviewState, why: Option<&str>) -> String {
    match (state, why) {
        (PreviewState::Checking, _) => "Checking".into(),
        (PreviewState::Ready, _) => "Ready".into(),
        (PreviewState::Failed, Some(why)) => format!("Failed: {why}"),
        (PreviewState::Failed, None) => "Failed".into(),
    }
}

/// The hue a state is drawn in, on its glyph and its edge.
fn hue(state: Option<PreviewState>) -> Token {
    match state {
        Some(PreviewState::Checking) => token::STATUS_RUNNING,
        Some(PreviewState::Ready) => token::STATUS_COMPLETED_SUCCESS,
        Some(PreviewState::Failed) => token::STATUS_COMPLETED_FAILED,
        None => token::BORDER_DEFAULT,
    }
}

impl TitleBar {
    /// The Refresh row, stacked in the switcher: the branches Refresh brings in, then Refresh preview and Skip, then
    /// after a hairline the offers still being checked or failed.
    pub(super) fn offers_section(&self, held: &Build, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let offers = self.face.offers.as_ref()?;
        let id = offers.refresh.as_ref()?.id.clone();
        let shown = lines(offers)?;
        let busy = held.report.restarting.is_some() || held.report.reverting || held.report.swapping;
        let restart = held.restart();
        let refreshing = restart.pending && restart.target == ipc::BuildSource::Preview;
        // One row of an offer; `at` numbers the rows down the whole section, so each keeps its own ids.
        let draw = |at: usize, one: &OfferLine| -> AnyElement {
            let ink = look.color(hue(one.state));
            let mark = one.state.map(|state| {
                let said: SharedString = state_said(state, one.why.as_deref()).into();
                let drawn = match state {
                    PreviewState::Checking => spinner(("offer-spin", at), px(12.), ink, cx).into_any_element(),
                    PreviewState::Ready => glyph("check", px(12.), ink).into_any_element(),
                    PreviewState::Failed => glyph("x", px(12.), ink).into_any_element(),
                };
                labelled(
                    div().id(("offer-state", at)).flex_none().h(look.length(token::LEADING_SM)).flex().items_center().tooltip(tip(said.clone())).child(drawn),
                    Role::Image,
                    said,
                )
            });
            let about = one.about.clone().map(|about| {
                let said: SharedString = about.into();
                div()
                    .id(("offer-about", at))
                    .min_w_0()
                    // Two lines, then an ellipsis; the whole line is in the tooltip.
                    .line_clamp(2)
                    .text_ellipsis()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .tooltip(tip(said.clone()))
                    .child(said)
            });
            div()
                .id(("offer", at))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                // The state's hue on the row's edge.
                .border_l(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(ink)
                .children(mark)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        // The title keeps the row's width and wraps, where it would be cut short.
                        .child(
                            div()
                                .min_w_0()
                                .font_family(sans(look))
                                .text_size(look.length(token::TEXT_SM))
                                .line_height(look.length(token::LEADING_SM))
                                .text_color(look.color(token::FG_DEFAULT))
                                .child(one.name.clone()),
                        )
                        .children(about),
                )
                .into_any_element()
        };
        // A list of rows in its own frame, a hairline between them apart from the rows so each edge keeps its hue.
        let list = |first: usize, rows: &[OfferLine], name: &'static str| -> AnyElement {
            let rows: Vec<AnyElement> = rows
                .iter()
                .enumerate()
                .map(|(within, one)| {
                    div()
                        .flex()
                        .flex_col()
                        .children((within > 0).then(|| div().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::BORDER_SUBTLE))))
                        .child(draw(first + within, one))
                        .into_any_element()
                })
                .collect();
            labelled(
                div()
                    .id(name)
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .children(rows),
                Role::List,
                name,
            )
            .into_any_element()
        };
        let refresh = {
            let label = if refreshing { "Refreshing preview" } else { "Refresh preview" };
            let button = Btn::new("refresh-offers", label, Weight::Primary).icon("rotate-cw").pending(refreshing).disabled(busy).draw(look, cx).flex_1();
            let button = if busy { button } else { button.on_click(cx.listener(|this, _, window, cx| this.refresh_offers_pressed(window, cx))) };
            labelled(button.tooltip(tip("Merge the offered branches and restart on the preview")), Role::Button, label)
        };
        let skip = {
            let button = Btn::new("skip-offers", "Skip", Weight::Secondary).draw(look, cx);
            let button = button.on_click(cx.listener(move |this, _, window, cx| this.skip_offers_pressed(id.clone(), window, cx)));
            labelled(button.tooltip(tip("Hide this row until the offers change")), Role::Button, "Skip")
        };
        let ready_count = shown.named.len();
        Some(
            div()
                .id("fleet-offers")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .children((!shown.named.is_empty()).then(|| list(0, &shown.named, "Offered for the preview")))
                .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(refresh).child(skip))
                // Below the buttons, after a hairline; none, and no hairline, where nothing waits.
                .children((!shown.waiting.is_empty()).then(|| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .child(div().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::BORDER_SUBTLE)))
                        .child(list(ready_count, &shown.waiting, "Offers not ready"))
                }))
                .into_any_element(),
        )
    }

    /// Refresh preview: the switcher's own preview switch.
    fn refresh_offers_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.change_build(ipc::BuildSource::Preview, window, cx);
    }

    /// Skip: the row goes in this frame, and Fleet is asked to keep it skipped. The keys stay in the panel.
    fn skip_offers_pressed(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let skipped = id.clone();
        self.answer(
            move |face| {
                if let Some(offers) = face.offers.as_mut() {
                    if offers.refresh.as_ref().is_some_and(|refresh| refresh.id == skipped) {
                        offers.refresh = None;
                    }
                }
            },
            cx,
        );
        if self.popup.is_some() {
            let focus = self.popup_focus.clone();
            window.focus(&focus, cx);
        }
        cx.emit(Asked::SkipOffers(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offers(json: &str) -> PreviewOffers {
        ipc::decode::<PreviewOffers>("PreviewOffers", json.as_bytes()).expect("offers")
    }

    fn row(branch: &str, title: &str, state: &str, extra: &str) -> String {
        format!(
            r#"{{"branch":"{branch}","tip":"abc","session_id":"s-{branch}","title":"{title}","about":"About {title}","offered_at":"2026-11-13T09:00:00Z","state":"{state}","in_build":false{extra}}}"#
        )
    }

    #[test]
    fn a_branch_is_named_by_its_jobs_title_and_about_and_never_by_its_branch() {
        let json = format!(r#"{{"offers":[{}],"refresh":{{"id":"r1","branches":["armada/session-1"]}}}}"#, row("armada/session-1", "Quiet the write-clash notices", "ready", ""));
        let shown = lines(&offers(&json)).expect("a row");
        assert_eq!(
            shown.named,
            vec![OfferLine {
                name: "Quiet the write-clash notices".into(),
                about: Some("About Quiet the write-clash notices".into()),
                state: Some(PreviewState::Ready),
                why: None
            }]
        );
        assert!(shown.waiting.is_empty());
        assert!(shown.named.iter().all(|line| !line.name.contains("armada/")));
    }

    #[test]
    fn a_branch_with_no_offer_or_no_title_is_named_by_the_branch() {
        let json = format!(r#"{{"offers":[{}],"refresh":{{"id":"r1","branches":["armada/a","armada/gone"]}}}}"#, row("armada/a", "", "ready", ""));
        let shown = lines(&offers(&json)).expect("a row");
        assert_eq!(shown.named[0].name, "armada/a");
        assert_eq!(shown.named[1], OfferLine { name: "armada/gone".into(), about: None, state: None, why: None });
    }

    #[test]
    fn the_ready_branches_stand_apart_from_the_offers_still_waiting_in_offer_order() {
        let ready = row("armada/a", "Alpha", "ready", "");
        let checking = row("armada/b", "Beta", "checking", "");
        let failed = row("armada/c", "Gamma", "failed", r#","why":"merge conflict in crates/fleet/src/building.rs""#);
        let held = r#"{"branch":"armada/d","tip":"abc","session_id":"s-d","title":"Delta","about":"x","offered_at":"2026-11-13T09:00:00Z","state":"failed","in_build":true}"#;
        let none = format!(r#"{{"offers":[{ready},{checking}]}}"#);
        assert_eq!(lines(&offers(&none)), None);
        let json = format!(r#"{{"offers":[{checking},{ready},{failed},{held}],"refresh":{{"id":"r1","branches":["armada/a"]}}}}"#);
        let shown = lines(&offers(&json)).expect("a row");
        let names = |lines: &[OfferLine]| lines.iter().map(|line| line.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&shown.named), ["Alpha"]);
        assert_eq!(names(&shown.waiting), ["Beta", "Gamma"]);
        let only_ready = format!(r#"{{"offers":[{ready}],"refresh":{{"id":"r1","branches":["armada/a"]}}}}"#);
        assert!(lines(&offers(&only_ready)).expect("a row").waiting.is_empty(), "nothing waits, so nothing is drawn below the buttons");
    }

    #[test]
    fn the_switchers_restart_gives_way_only_where_it_would_also_read_refresh_preview() {
        let held = |json: &str| super::super::face::build_of(&ipc::decode::<ipc::FleetBuildReport>("FleetBuildReport", json.as_bytes()).expect("a report"));
        let on_preview = held(r#"{"on":"preview"}"#);
        assert_eq!(on_preview.restart().label, "Refresh preview");
        assert!(!restart_stays(&on_preview, true), "with the row, only the row's button reads Refresh preview");
        assert!(restart_stays(&on_preview, false), "with no row, the switcher's own stays");
        let on_main = held(r#"{"on":"main","position":{"ahead":0,"behind":3}}"#);
        assert_eq!(on_main.restart().label, "Update to main");
        assert!(restart_stays(&on_main, true), "Update to main is not a Refresh preview, and stays");
        let going = held(r#"{"on":"main","restarting":"preview"}"#);
        assert!(!restart_stays(&going, true), "a restart onto the preview shows its pending word on the row's button");
    }

    #[test]
    fn a_state_is_said_and_a_failed_one_says_why() {
        assert_eq!(state_said(PreviewState::Checking, None), "Checking");
        assert_eq!(state_said(PreviewState::Ready, None), "Ready");
        assert_eq!(state_said(PreviewState::Failed, Some("error[E0308]")), "Failed: error[E0308]");
        assert_eq!(state_said(PreviewState::Failed, None), "Failed");
    }
}
