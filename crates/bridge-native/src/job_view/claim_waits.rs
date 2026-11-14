//! The claims card: each tracker item the Job waits on because another Job or a person holds it, and a line while
//! its claims have not reached the project's store (`docs/practices/protocol/a-jobs-wait-on-a-held-item.md`). It
//! stands at the thread's end with the other waits, and has no act: Fleet wakes the Job when a wait ends.

use chrono::{DateTime, Local, Utc};
use gpui::{div, prelude::*, AnyElement, Context, Hsla, Role, SharedString};
use ipc::{ClaimWait, ClaimWaitEnds, Instant};

use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The line a wait reads: who holds the item, then the holding Job's title, or the item where it has none.
pub(super) fn wait_said(wait: &ClaimWait) -> String {
    let holder = wait.holder.trim();
    let mac = wait.machine.as_deref().map(str::trim).filter(|mac| !mac.is_empty());
    let who = match (holder, mac) {
        ("", Some(mac)) => mac.to_string(),
        (holder, Some(mac)) => format!("{holder}'s Mac {mac}"),
        ("", None) => match wait.ends {
            ClaimWaitEnds::ClaimEnds => "another Job".to_string(),
            ClaimWaitEnds::Unassigned => "its assignee in beads".to_string(),
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

/// The line drawn while the Job's claims have not reached the store.
pub(super) const UNSENT: &str = "Claims not sent yet";
/// Under it: when they go.
pub(super) const UNSENT_UNDER: &str = "Sent when the project's store answers";

impl JobView {
    /// The card, or `None` where the Job waits on no held item and every claim is sent.
    pub(super) fn claim_waits_card(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let record = self.record(id, cx)?;
        if record.claim_waits.is_empty() && !record.claims_unsent {
            return None;
        }
        let now = crate::mock::fixed_now().map_or_else(Utc::now, |now| now.with_timezone(&Utc));
        let waiting = look.color(token::STATUS_AWAITING_REVIEW);
        // As the column narrows (under 400px) the chip and the figure go under the words, so the words keep the width.
        let narrow = self.pane.is_some_and(|pane| (pane - look.length(token::SPACE_4) * 2.).min(self.column(look)) < gpui::px(400.));
        let rows = record.claim_waits.iter().enumerate().map(|(at, wait)| self.claim_wait_row(at, wait, now, narrow, look, cx));
        let unsent = record.claims_unsent.then(|| {
            let mark = div().id("claims-unsent-mark").flex_none().child(glyph(look, "unplug", Size::Small, waiting)).tooltip(tip("Not sent"));
            labelled(line(look, "claims-unsent", mark.into_any_element(), UNSENT, None, UNSENT_UNDER, false, narrow), Role::Row, UNSENT).into_any_element()
        });
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
                    .children(rows)
                    .children(unsent),
                Role::Region,
                "Claims",
            )
            .into_any_element(),
        )
    }

    /// One wait: the dot, who holds the item and the holding Job's title, its Mac's chip and how long; what ends it under.
    fn claim_wait_row(&self, at: usize, wait: &ClaimWait, now: DateTime<Utc>, narrow: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said = wait_said(wait);
        let dot = div()
            .id(SharedString::from(format!("claim-wait-mark:{at}")))
            .flex_none()
            .size(look.length(token::SPACE_3))
            .flex()
            .items_center()
            .justify_center()
            .child(dot(look, look.color(token::STATUS_AWAITING_REVIEW)))
            .tooltip(tip("Waiting"));
        let chip = wait.machine.as_deref().map(str::trim).filter(|mac| !mac.is_empty()).map(|mac| {
            // A Mac paired with this one draws in its own colour; a teammate's in the accent, as the design has it.
            let hue = crate::pairing::macs(cx).iter().find(|one| one.name.eq_ignore_ascii_case(mac)).map_or_else(|| look.color(token::ACCENT), |one| crate::settings::macs::colour_of(look, one.colour));
            crate::pairing::pill_in(look, SharedString::from(format!("claim-wait-mac:{at}")), mac, hue)
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
        labelled(line(look, id, dot.into_any_element(), &said, tail, &ends_said(wait), true, narrow), Role::Row, said.clone()).into_any_element()
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
            since: Instant::carried("2026-11-14T10:00:00Z"),
            ends,
        }
    }

    #[test]
    fn a_wait_on_another_job_names_its_mac_and_title() {
        let one = wait("nick", Some("Home"), Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds);
        assert_eq!(wait_said(&one), "Waiting on nick's Mac Home: Fix the flaky queue test");
        assert_eq!(ends_said(&one), "Carries on when the other Job's claim ends");
    }

    #[test]
    fn a_wait_on_a_person_in_beads_names_them_and_the_item() {
        let one = wait("alice", None, None, ClaimWaitEnds::Unassigned);
        assert_eq!(wait_said(&one), "Waiting on alice: armada-zasx.3");
        assert_eq!(ends_said(&one), "Carries on when alice is unassigned");
    }

    #[test]
    fn a_wait_missing_its_title_or_mac_falls_back() {
        assert_eq!(wait_said(&wait("nick", Some("Home"), None, ClaimWaitEnds::ClaimEnds)), "Waiting on nick's Mac Home: armada-zasx.3", "no title: the item");
        assert_eq!(wait_said(&wait("nick", None, Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds)), "Waiting on nick: Fix the flaky queue test", "no Mac: the holder");
        assert_eq!(wait_said(&wait("", Some("Home"), Some("Fix the flaky queue test"), ClaimWaitEnds::ClaimEnds)), "Waiting on Home: Fix the flaky queue test", "no holder: the Mac");
        assert_eq!(wait_said(&wait(" ", Some(" "), Some(""), ClaimWaitEnds::ClaimEnds)), "Waiting on another Job: armada-zasx.3", "nothing named");
        let nobody = wait("", None, None, ClaimWaitEnds::Unassigned);
        assert_eq!(wait_said(&nobody), "Waiting on its assignee in beads: armada-zasx.3");
        assert_eq!(ends_said(&nobody), "Carries on when the item is unassigned");
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
}
