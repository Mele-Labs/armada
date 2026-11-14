//! The Fleet panel's Waiting section: the branches Jobs offered for the preview, each named by its Job's title
//! and its line on what it holds, with where Fleet's check of it stands, then what the preview left out and the
//! build of the picked branches. The words are worked out here from `ipc::PreviewOffers`; the section draws them.

use gpui::{div, prelude::*, px, AnyElement, Context, Role, SharedString, Window};
use ipc::{BuildSource, PreviewOffers, PreviewState, SnapshotState};

use super::face::{stage_word, Build};
use super::parts::{glyph, spinner, tip, Btn, Weight};
use super::switcher::{band_card, row, state_glyph, Row};
use super::{Asked, TitleBar};
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// One offer as the row draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferLine {
    /// The Job's title; the branch only where Fleet holds no offer for it, or the offer has no title.
    pub name: String,
    /// The branch, said on hover.
    pub branch: String,
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
                branch: offer.branch.clone(),
                about: Some(offer.about.clone()).filter(|about| !about.trim().is_empty()),
                state: Some(offer.state),
                why: offer.why.clone(),
            },
            None => OfferLine { name: branch.to_string(), branch: branch.to_string(), about: None, state: None, why: None },
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
    /// What waits to be added: the ready offers Refresh brings in, the offers still being checked or failed, the
    /// branches the preview left out, and the build of the picked branches. None where nothing waits.
    pub(super) fn waiting_section(&self, held: &Build, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let report = &held.report;
        let offers = self.face.offers.as_ref();
        let shown = offers.and_then(lines);
        let left_out: &[ipc::LeftOutBranch] = if report.on == BuildSource::Preview && report.restarting.is_none() { &report.left_out } else { &[] };
        let snapshot = report.snapshot.as_ref();
        if shown.is_none() && left_out.is_empty() && snapshot.is_none() {
            return None;
        }
        let offer_row = |at: usize, one: &OfferLine| -> AnyElement {
            let ink = look.color(hue(one.state));
            let mark = match one.state {
                Some(state) => {
                    let said: SharedString = state_said(state, one.why.as_deref()).into();
                    let drawn = match state {
                        PreviewState::Checking => spinner(("offer-spin", at), px(12.), ink, cx).into_any_element(),
                        PreviewState::Ready => glyph("check", px(12.), ink).into_any_element(),
                        PreviewState::Failed => glyph("x", px(12.), ink).into_any_element(),
                    };
                    labelled(div().id(("offer-state", at)).flex_none().tooltip(tip(said.clone())).child(drawn), Role::Image, said).into_any_element()
                }
                None => state_glyph(format!("offer-state-{at}").into(), "git-branch", look.color(token::FG_MUTED), "Not offered".into()),
            };
            row(
                look,
                "offer",
                at,
                Row {
                    mark,
                    name: one.name.clone().into(),
                    hover: (one.name != one.branch).then(|| one.branch.clone().into()),
                    about: one.about.clone().map(Into::into),
                    word: None,
                    end: None,
                },
            )
        };
        let list = |name: &'static str, rows: Vec<AnyElement>| -> Option<AnyElement> {
            (!rows.is_empty()).then(|| labelled(div().id(name).flex().flex_col().children(rows), Role::List, name).into_any_element())
        };
        let mut groups: Vec<AnyElement> = Vec::new();
        if let Some(shown) = shown.as_ref() {
            let ready = shown.named.len();
            groups.extend(list("Offered for the preview", shown.named.iter().enumerate().map(|(at, one)| offer_row(at, one)).collect()));
            groups.extend(list("Offers not ready", shown.waiting.iter().enumerate().map(|(at, one)| offer_row(ready + at, one)).collect()));
        }
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let title_of = |branch: &str| {
            offers.and_then(|offers| offers.offers.iter().find(|offer| offer.branch == branch && !offer.title.trim().is_empty())).map(|offer| offer.title.clone())
        };
        groups.extend(list(
            "Left out of the preview",
            left_out
                .iter()
                .enumerate()
                .map(|(at, one)| {
                    let name = title_of(&one.branch);
                    let moved = one.moved.then(|| state_glyph(format!("left-out-moved-{at}").into(), "git-commit-horizontal", look.color(token::FG_MUTED), "Moved since it was left out".into()));
                    row(
                        look,
                        "left-out",
                        at,
                        Row {
                            mark: state_glyph(format!("left-out-mark-{at}").into(), "circle-slash", amber, "Left out of the preview".into()),
                            hover: name.is_some().then(|| one.branch.clone().into()),
                            name: name.unwrap_or_else(|| one.branch.clone()).into(),
                            about: Some(one.why.clone().into()),
                            word: Some(("left out", amber)),
                            end: moved,
                        },
                    )
                })
                .collect(),
        ));
        groups.extend(snapshot.map(|snapshot| self.snapshot_row(held, snapshot, look, cx)));
        Some(
            labelled(
                band_card(look, "waiting", "plus", token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG, "Waiting to be added", None, None, groups),
                Role::Group,
                "Waiting to be added",
            )
            .into_any_element(),
        )
    }

    /// The build of the picked branches: being built, ready to swap onto, or failed.
    fn snapshot_row(&self, held: &Build, snapshot: &ipc::KeptSnapshot, look: &Look, cx: &Context<Self>) -> AnyElement {
        let report = &held.report;
        let (mark, word, about): (AnyElement, (&'static str, gpui::Hsla), Option<SharedString>) = match snapshot.state {
            SnapshotState::Building => {
                let ink = look.color(token::STATUS_RUNNING);
                let said = snapshot.stage.map_or("Building", stage_word);
                (
                    labelled(div().id("snapshot-mark").flex_none().tooltip(tip(said)).child(spinner("snapshot-spin", px(12.), ink, cx)), Role::Status, "Building").into_any_element(),
                    ("building", ink),
                    None,
                )
            }
            SnapshotState::Ready => {
                let ink = look.color(token::STATUS_COMPLETED_SUCCESS);
                (state_glyph("snapshot-mark".into(), "check", ink, "Ready".into()), ("ready", ink), snapshot.bridge_reopens.then(|| "Bridge reopens".into()))
            }
            SnapshotState::Failed => {
                let ink = look.color(token::STATUS_COMPLETED_FAILED);
                (state_glyph("snapshot-mark".into(), "x", ink, "Failed".into()), ("failed", ink), snapshot.failed.clone().map(Into::into))
            }
        };
        let swap = (snapshot.state == SnapshotState::Ready).then(|| {
            let off = report.swapping || report.reverting || report.restarting.is_some();
            let label = if report.swapping { "Swapping" } else { "Swap" };
            let button = Btn::new("swap-build", label, Weight::Secondary).pending(report.swapping).disabled(off).draw(look, cx).tooltip(tip("Restart Fleet on the build of the picked branches"));
            let button = if off { button } else { button.on_click(cx.listener(|this, _, _, cx| this.swap_pressed(cx))) };
            labelled(button, Role::Button, label).into_any_element()
        });
        let drawn = row(
            look,
            "snapshot",
            0,
            Row {
                mark,
                name: "Picked branches".into(),
                hover: (!snapshot.jobs.is_empty()).then(|| snapshot.jobs.join(", ").into()),
                about,
                word: if swap.is_some() { None } else { Some(word) },
                end: swap,
            },
        );
        labelled(div().id("held-by-snapshot").child(drawn), Role::List, "Held by the snapshot").into_any_element()
    }

    /// Skip: the row goes in this frame, and Fleet is asked to keep it skipped. The keys stay in the panel.
    pub(super) fn skip_offers_pressed(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
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
                branch: "armada/session-1".into(),
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
        assert_eq!(shown.named[1], OfferLine { name: "armada/gone".into(), branch: "armada/gone".into(), about: None, state: None, why: None });
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
    fn a_state_is_said_and_a_failed_one_says_why() {
        assert_eq!(state_said(PreviewState::Checking, None), "Checking");
        assert_eq!(state_said(PreviewState::Ready, None), "Ready");
        assert_eq!(state_said(PreviewState::Failed, Some("error[E0308]")), "Failed: error[E0308]");
        assert_eq!(state_said(PreviewState::Failed, None), "Failed");
    }
}
