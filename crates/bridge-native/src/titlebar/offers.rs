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

/// The lines the Refresh row draws, or none while Fleet says no row.
///
/// First the branches Refresh brings in, named by their offers, then the offers the running build does not
/// hold that are still being checked or failed, in offer order, so the row says why a Job's branch is not in
/// the set.
pub fn lines(offers: &PreviewOffers) -> Option<Vec<OfferLine>> {
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
    let named = refresh.branches.iter().map(|branch| line(branch));
    let waiting = offers
        .offers
        .iter()
        .filter(|offer| !offer.in_build && offer.state != PreviewState::Ready && !refresh.branches.contains(&offer.branch))
        .map(|offer| line(&offer.branch));
    Some(named.chain(waiting).collect())
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
    /// The Refresh row, stacked in the switcher: one line for each offer it names, then Refresh preview and Skip.
    pub(super) fn offers_section(&self, held: &Build, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let offers = self.face.offers.as_ref()?;
        let id = offers.refresh.as_ref()?.id.clone();
        let lines = lines(offers)?;
        let busy = held.report.restarting.is_some() || held.report.reverting || held.report.swapping;
        let rows: Vec<AnyElement> = lines
            .iter()
            .enumerate()
            .map(|(at, one)| {
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
                        .truncate()
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .tooltip(tip(said.clone()))
                        .child(said)
                });
                let row = div()
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
                            .child(
                                div()
                                    .truncate()
                                    .font_family(sans(look))
                                    .text_size(look.length(token::TEXT_SM))
                                    .line_height(look.length(token::LEADING_SM))
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .child(one.name.clone()),
                            )
                            .children(about),
                    );
                // A hairline between rows, apart from the row so the edge keeps its state's hue.
                div()
                    .flex()
                    .flex_col()
                    .children((at > 0).then(|| div().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::BORDER_SUBTLE))))
                    .child(row)
                    .into_any_element()
            })
            .collect();
        let refresh = {
            let button = Btn::new("refresh-offers", "Refresh preview", Weight::Primary).icon("rotate-cw").disabled(busy).draw(look, cx).flex_1();
            let button = if busy { button } else { button.on_click(cx.listener(|this, _, window, cx| this.refresh_offers_pressed(window, cx))) };
            labelled(button.tooltip(tip("Merge the offered branches and restart on the preview")), Role::Button, "Refresh preview")
        };
        let skip = {
            let button = Btn::new("skip-offers", "Skip", Weight::Secondary).draw(look, cx);
            let button = button.on_click(cx.listener(move |this, _, window, cx| this.skip_offers_pressed(id.clone(), window, cx)));
            labelled(button.tooltip(tip("Hide this row until the offers change")), Role::Button, "Skip")
        };
        Some(
            div()
                .id("fleet-offers")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    div()
                        .id("offers-list")
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .bg(look.color(token::BG_SUNKEN))
                        .children(rows),
                    Role::List,
                    "Offered for the preview",
                ))
                .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(refresh).child(skip))
                .into_any_element(),
        )
    }

    /// Refresh preview: the switcher's own preview switch, asking first where Jobs would be adopted.
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
        let named = lines(&offers(&json)).expect("a row");
        assert_eq!(
            named,
            vec![OfferLine {
                name: "Quiet the write-clash notices".into(),
                about: Some("About Quiet the write-clash notices".into()),
                state: Some(PreviewState::Ready),
                why: None
            }]
        );
        assert!(named.iter().all(|line| !line.name.contains("armada/")));
    }

    #[test]
    fn a_branch_with_no_offer_or_no_title_is_named_by_the_branch() {
        let json = format!(r#"{{"offers":[{}],"refresh":{{"id":"r1","branches":["armada/a","armada/gone"]}}}}"#, row("armada/a", "", "ready", ""));
        let named = lines(&offers(&json)).expect("a row");
        assert_eq!(named[0].name, "armada/a");
        assert_eq!(named[1], OfferLine { name: "armada/gone".into(), about: None, state: None, why: None });
    }

    #[test]
    fn no_refresh_is_no_row_and_the_offers_still_waiting_follow_the_branches_it_brings_in() {
        let ready = row("armada/a", "Alpha", "ready", "");
        let checking = row("armada/b", "Beta", "checking", "");
        let failed = row("armada/c", "Gamma", "failed", r#","why":"merge conflict in crates/fleet/src/building.rs""#);
        let held = r#"{"branch":"armada/d","tip":"abc","session_id":"s-d","title":"Delta","about":"x","offered_at":"2026-11-13T09:00:00Z","state":"failed","in_build":true}"#;
        let none = format!(r#"{{"offers":[{ready},{checking}]}}"#);
        assert_eq!(lines(&offers(&none)), None);
        let json = format!(r#"{{"offers":[{checking},{ready},{failed},{held}],"refresh":{{"id":"r1","branches":["armada/a"]}}}}"#);
        let named: Vec<String> = lines(&offers(&json)).expect("a row").into_iter().map(|line| line.name).collect();
        assert_eq!(named, ["Alpha", "Beta", "Gamma"]);
    }

    #[test]
    fn a_state_is_said_and_a_failed_one_says_why() {
        assert_eq!(state_said(PreviewState::Checking, None), "Checking");
        assert_eq!(state_said(PreviewState::Ready, None), "Ready");
        assert_eq!(state_said(PreviewState::Failed, Some("error[E0308]")), "Failed: error[E0308]");
        assert_eq!(state_said(PreviewState::Failed, None), "Failed");
    }
}
