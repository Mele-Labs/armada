//! The claims card: each tracker item the Job waits on because another Job or a person holds it
//! (`docs/practices/protocol/a-jobs-wait-on-a-held-item.md`); claims not yet in the project's store are a badge on the
//! composer's toolbar, not a card. It stands at the thread's end with the other waits, and has no act: Fleet wakes the Job when a wait ends.

use chrono::{DateTime, Local, Utc};
use gpui::{div, prelude::*, AnyElement, Context, Hsla, Role, SharedString};
use ipc::{ClaimWait, ClaimWaitEnds, Instant, Mac};

use super::rows::{mono, tip};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// A Mac as the person knows it: the paired Mac the host's name matches, by the name he gave it, and where none
/// matches the host's name without a trailing `.local`. A paired Mac is matched with its `.local` left off too.
fn mac_called<'a>(macs: &'a [Mac], host: &'a str) -> (&'a str, Option<&'a Mac>) {
    let bare = |name: &'a str| {
        let name = name.trim();
        let cut = name.len().checked_sub(".local".len()).filter(|cut| name.is_char_boundary(*cut) && name[*cut..].eq_ignore_ascii_case(".local"));
        cut.map_or(name, |cut| &name[..cut])
    };
    match macs.iter().find(|one| bare(&one.name).eq_ignore_ascii_case(bare(host))) {
        Some(one) => (one.name.trim(), Some(one)),
        None => (bare(host), None),
    }
}

/// The line a wait reads: who holds the item, then the holding Job's title, or the item where it has none. A Job of
/// this Mac's is named as a Job, never by the Mac the person sits at.
pub(super) fn wait_said(wait: &ClaimWait, macs: &[Mac]) -> String {
    let holder = wait.holder.trim();
    let mac = wait.machine.as_deref().map(str::trim).filter(|mac| !mac.is_empty()).map(|mac| mac_called(macs, mac).0);
    let who = match (holder, mac) {
        _ if wait.this_mac => match wait.ends {
            ClaimWaitEnds::FixMerges => "the Job fixing it".to_string(),
            _ => "another Job".to_string(),
        },
        ("", Some(mac)) => mac.to_string(),
        (holder, Some(mac)) => format!("{holder}'s Mac {mac}"),
        ("", None) => match wait.ends {
            ClaimWaitEnds::ClaimEnds => "another Job".to_string(),
            ClaimWaitEnds::Unassigned => "its assignee in beads".to_string(),
            ClaimWaitEnds::FixMerges if wait.title.as_deref().is_some_and(|title| !title.trim().is_empty()) => "the Job fixing it".to_string(),
            ClaimWaitEnds::FixMerges => "main's own fix".to_string(),
        },
        (holder, None) => holder.to_string(),
    };
    let what = wait.title.as_deref().map(str::trim).filter(|title| !title.is_empty()).unwrap_or(wait.item.trim());
    if what.is_empty() { format!("Waiting on {who}") } else { format!("Waiting on {who}: {what}") }
}

/// What ends the wait, in Armada's words: Fleet sends the kind alone.
pub(super) fn ends_said(wait: &ClaimWait) -> String {
    match (wait.ends, wait.holder.trim()) {
        (ClaimWaitEnds::ClaimEnds, _) => "Carries on when the other Job's claim ends".to_string(),
        (ClaimWaitEnds::Unassigned, "") => "Carries on when the item is unassigned".to_string(),
        (ClaimWaitEnds::Unassigned, person) => format!("Carries on when {person} is unassigned"),
        (ClaimWaitEnds::FixMerges, _) => "Requeues when the fix merges".to_string(),
    }
}

/// How long the Job has waited, in its largest unit: `<1 min`, `12 min`, `2 h`, `3 d`. Text that is no instant has none.
pub(super) fn waited(since: &Instant, now: DateTime<Utc>) -> Option<String> {
    let since = DateTime::parse_from_rfc3339(since.as_str()).ok()?;
    let minutes = (now - since.with_timezone(&Utc)).num_minutes().max(0);
    Some(match minutes {
        0 => "<1 min".to_string(),
        1..=59 => format!("{minutes} min"),
        60..=1439 => format!("{} h", minutes / 60),
        _ => format!("{} d", minutes / 1440),
    })
}

/// The badge the composer's toolbar wears while the Job's claims have not reached the store, and what its tooltip says.
pub(super) const UNSENT: &str = "Not shared yet";
pub(super) const UNSENT_TIP: &str = "Other Jobs cannot see what this Job holds yet. Armada keeps trying";

impl JobView {
    /// The card, or `None` where the Job waits on no held item and every claim is sent.
    pub(super) fn claim_waits_card(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let record = self.record(id, cx)?;
        if record.claim_waits.is_empty() {
            return None;
        }
        let now = crate::mock::fixed_now().map_or_else(Utc::now, |now| now.with_timezone(&Utc));
        let waiting = look.color(token::STATUS_AWAITING_REVIEW);
        // As the column narrows (under 400px) the chip and the figure go under the words, so the words keep the width.
        let narrow = self.pane.is_some_and(|pane| (pane - look.length(token::SPACE_4) * 2.).min(self.column(look)) < gpui::px(400.));
        let rows = record.claim_waits.iter().enumerate().map(|(at, wait)| self.claim_wait_row(at, wait, now, narrow, look, cx));
        Some(
            labelled(
                div()
                    .id("claims")
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .p(look.length(token::SPACE_1))
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(waiting)
                    .bg(look.color(token::BG_SUNKEN))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .children(rows),
                Role::Region,
                "Claims",
            )
            .into_any_element(),
        )
    }

    /// One wait: the dot, who holds the item and the holding Job's title, its Mac's chip and how long; what ends it under.
    fn claim_wait_row(&self, at: usize, wait: &ClaimWait, now: DateTime<Utc>, narrow: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let macs = crate::pairing::macs(cx);
        let said = wait_said(wait, macs);
        let dot = div()
            .id(SharedString::from(format!("claim-wait-mark:{at}")))
            .flex_none()
            .size(look.length(token::SPACE_3))
            .flex()
            .items_center()
            .justify_center()
            .child(dot(look, look.color(token::STATUS_AWAITING_REVIEW)))
            .tooltip(tip("Waiting"));
        // A Job of this Mac's has no chip: the person is on that Mac. Another Mac's draws in its own colour where it is
        // paired with this one, a teammate's in the accent, as the design has it.
        let chip = wait.machine.as_deref().map(str::trim).filter(|mac| !mac.is_empty() && !wait.this_mac).map(|mac| {
            let (name, paired) = mac_called(macs, mac);
            let hue = paired.map_or_else(|| look.color(token::ACCENT), |one| crate::settings::macs::colour_of(look, one.colour));
            crate::pairing::pill_in(look, SharedString::from(format!("claim-wait-mac:{at}")), name, hue)
        });
        let since = waited(&wait.since, now).map(|figure| {
            let from = DateTime::parse_from_rfc3339(wait.since.as_str()).ok().map(|at| crate::settings::time::stamp(at.with_timezone(&Local))).unwrap_or_default();
            let drawn = div()
                .id(SharedString::from(format!("claim-wait-since:{at}")))
                .flex_none()
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_SUBTLE))
                .child(SharedString::from(figure.clone()))
                .tooltip(tip(format!("Waiting since {from}")));
            labelled(drawn, Role::Timer, figure).into_any_element()
        });
        let id = SharedString::from(format!("claim-wait:{at}"));
        let tail = (chip.is_some() || since.is_some()).then(|| div().flex_none().flex().items_center().gap(look.length(token::SPACE_3)).h(look.length(token::LEADING_SM)).children(chip).children(since).into_any_element());
        let row = line(look, id, dot.into_any_element(), &said, tail, &ends_said(wait), true, narrow);
        // A wait on a Job of this Mac's opens that Job, on a press or Enter and Space where it holds focus.
        match wait.job.clone().filter(|_| wait.this_mac) {
            Some(job) => labelled(
                row.cursor_pointer()
                    .tab_index(0)
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .tooltip(tip("Open the Job"))
                    .on_click(cx.listener(move |this, _, _, cx| this.open_job(job.clone(), cx))),
                Role::Button,
                said.clone(),
            )
            .into_any_element(),
            None => labelled(row, Role::Row, said.clone()).into_any_element(),
        }
    }
}

/// A round mark in `hue`, as the design's state dot.
fn dot(look: &Look, hue: Hsla) -> gpui::Div {
    div().flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(hue)
}

/// A line of the card: its mark, its words, then what stands at its end kept whole at the right, or under the words
/// where narrow; under it, muted, a second line, after a dot in the running hue where the Job runs again then.
#[allow(clippy::too_many_arguments)]
fn line(look: &Look, id: impl Into<SharedString>, mark: AnyElement, said: &str, tail: Option<AnyElement>, under: &str, runs: bool, narrow: bool) -> gpui::Stateful<gpui::Div> {
    let id: SharedString = id.into();
    let mut tail = tail;
    // Each mark sits on its line's first line of text, so a long line wraps under its words.
    let lead = |mark: AnyElement| div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(mark);
    let top = div()
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_3))
        .text_color(look.color(token::FG_DEFAULT))
        .child(lead(mark))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).child(SharedString::from(said.to_string())).children(if narrow { tail.take() } else { None }))
        .children(tail);
    let next = div()
        .id(SharedString::from(format!("{id}-next")))
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_3))
        .text_color(look.color(token::FG_MUTED))
        .child(lead({
            let place = div().id(SharedString::from(format!("{id}-next-mark"))).flex_none().size(look.length(token::SPACE_3)).flex().items_center().justify_center();
            if runs { place.child(dot(look, look.color(token::STATUS_RUNNING))).tooltip(tip("Runs again")).into_any_element() } else { place.into_any_element() }
        }))
        .child(div().flex_1().min_w_0().child(SharedString::from(under.to_string())));
    div()
        .id(id)
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .py(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_SM))
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .child(top)
        .child(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(holder: &str, machine: Option<&str>, title: Option<&str>, ends: ClaimWaitEnds) -> ClaimWait {
        ClaimWait {
            item: "armada-zasx.3".into(),
            holder: holder.into(),
            machine: machine.map(str::to_string),
            title: title.map(str::to_string),
            this_mac: false,
            job: None,
            since: Instant::carried("2026-11-14T10:00:00Z"),
            ends,
        }
    }

    #[test]
    fn a_wait_on_another_job_names_its_mac_and_title() {
        let one = wait("nick", Some("Home"), Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds);
        assert_eq!(wait_said(&one, &[]), "Waiting on nick's Mac Home: Fix the flaky queue test");
        assert_eq!(ends_said(&one), "Carries on when the other Job's claim ends");
    }

    #[test]
    fn a_wait_on_a_person_in_beads_names_them_and_the_item() {
        let one = wait("alice", None, None, ClaimWaitEnds::Unassigned);
        assert_eq!(wait_said(&one, &[]), "Waiting on alice: armada-zasx.3");
        assert_eq!(ends_said(&one), "Carries on when alice is unassigned");
    }

    #[test]
    fn a_wait_missing_its_title_or_mac_falls_back() {
        assert_eq!(wait_said(&wait("nick", Some("Home"), None, ClaimWaitEnds::ClaimEnds), &[]), "Waiting on nick's Mac Home: armada-zasx.3", "no title: the item");
        assert_eq!(wait_said(&wait("nick", None, Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds), &[]), "Waiting on nick: Fix the flaky queue test", "no Mac: the holder");
        assert_eq!(wait_said(&wait("", Some("Home"), Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds), &[]), "Waiting on Home: Fix the flaky queue test", "no holder: the Mac");
        assert_eq!(wait_said(&wait(" ", Some(" "), Some(""), ClaimWaitEnds::ClaimEnds), &[]), "Waiting on another Job: armada-zasx.3", "nothing named");
        let nobody = wait("", None, None, ClaimWaitEnds::Unassigned);
        assert_eq!(wait_said(&nobody, &[]), "Waiting on its assignee in beads: armada-zasx.3");
        assert_eq!(ends_said(&nobody), "Carries on when the item is unassigned");
    }

    #[test]
    fn a_wait_on_a_failing_tests_fix_names_the_fixer_or_main_and_requeues() {
        let elsewhere = ClaimWait { item: "queue::settles".into(), ..wait("nick", Some("Home"), Some("Fix the flaky test settles"), ClaimWaitEnds::FixMerges) };
        assert_eq!(wait_said(&elsewhere, &[]), "Waiting on nick's Mac Home: Fix the flaky test settles");
        assert_eq!(ends_said(&elsewhere), "Requeues when the fix merges");
        let unnamed = ClaimWait { item: "queue::settles".into(), ..wait("", None, Some("Fix the flaky test settles"), ClaimWaitEnds::FixMerges) };
        assert_eq!(wait_said(&unnamed, &[]), "Waiting on the Job fixing it: Fix the flaky test settles");
        let main = ClaimWait { item: "queue::settles".into(), ..wait("", None, None, ClaimWaitEnds::FixMerges) };
        assert_eq!(wait_said(&main, &[]), "Waiting on main's own fix: queue::settles");
    }

    #[test]
    fn a_wait_on_a_job_of_this_mac_names_the_job_not_the_mac() {
        let here = |ends, holder: &str| ClaimWait { this_mac: true, job: Some("5aaa472e".into()), ..wait(holder, Some("Nicks-MacBook-Pro-2026.local"), Some("Fix the flaky test settles"), ends) };
        let fixing = here(ClaimWaitEnds::FixMerges, "nick");
        assert_eq!(wait_said(&fixing, &[]), "Waiting on the Job fixing it: Fix the flaky test settles");
        assert_eq!(wait_said(&here(ClaimWaitEnds::ClaimEnds, "nick"), &[]), "Waiting on another Job: Fix the flaky test settles");
        let untitled = ClaimWait { title: None, ..fixing };
        assert_eq!(wait_said(&untitled, &[]), "Waiting on the Job fixing it: armada-zasx.3", "no title: the item");
        for said in [wait_said(&untitled, &[]), wait_said(&here(ClaimWaitEnds::ClaimEnds, ""), &[])] {
            assert!(!said.contains("local") && !said.contains("Mac"), "{said}");
        }
    }

    #[test]
    fn a_wait_on_another_mac_names_the_paired_mac_and_drops_a_local_host() {
        let paired = [Mac { id: "studio".into(), name: "Studio".into(), colour: ipc::MacColour::Teal, ..Mac::default() }];
        let there = wait("nick", Some("studio.local"), Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds);
        assert_eq!(wait_said(&there, &paired), "Waiting on nick's Mac Studio: Fix the flaky queue test", "the paired name, not the host");
        assert_eq!(mac_called(&paired, "STUDIO.LOCAL").1.map(|one| one.colour), Some(ipc::MacColour::Teal), "the chip finds the same Mac");
        let unpaired = wait("nick", Some("Nicks-MacBook-Pro-2026.local"), Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds);
        assert_eq!(wait_said(&unpaired, &paired), "Waiting on nick's Mac Nicks-MacBook-Pro-2026: Fix the flaky queue test", "no paired Mac: the host without .local");
        assert_eq!(mac_called(&[], "Home").0, "Home", "a name with no .local stays whole");
    }

    #[test]
    fn the_since_figure_counts_in_its_largest_unit() {
        let at = |text: &str| DateTime::parse_from_rfc3339(text).expect("an instant").with_timezone(&Utc);
        let since = Instant::carried("2026-11-14T10:00:00Z");
        assert_eq!(waited(&since, at("2026-11-14T10:00:40Z")).as_deref(), Some("<1 min"));
        assert_eq!(waited(&since, at("2026-11-14T10:12:30Z")).as_deref(), Some("12 min"));
        assert_eq!(waited(&since, at("2026-11-14T12:05:00Z")).as_deref(), Some("2 h"));
        assert_eq!(waited(&since, at("2026-11-17T10:00:00Z")).as_deref(), Some("3 d"));
        assert_eq!(waited(&since, at("2026-11-14T09:00:00Z")).as_deref(), Some("<1 min"), "a clock behind Fleet's counts none");
        assert_eq!(waited(&Instant::carried("not a time"), at("2026-11-14T10:00:00Z")), None);
    }

    #[gpui::test]
    fn claims_not_sent_are_a_badge_by_the_context_ring_and_draw_no_claims_card(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::{drawn, rooted_job};
        let (view, cx) = rooted_job(cx, "claim-waits", "mock-01", gpui::size(gpui::px(1400.), gpui::px(900.)), |_| {});
        let (unsent, waits) = view.read_with(cx, |view, cx| view.record("mock-01", cx).map(|job| (job.claims_unsent, job.claim_waits.len())).expect("the Job is held"));
        assert_eq!((unsent, waits), (true, 0), "the Job's claims are unsent and it waits on no item");
        assert!(!drawn(cx, Role::Region, "Claims"), "with no wait there is no claims card");
        assert!(drawn(cx, Role::Status, UNSENT), "the badge stands in the composer's toolbar");
    }
}
