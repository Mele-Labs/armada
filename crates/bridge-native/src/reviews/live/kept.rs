//! The lists as Fleet kept them: drawn at once under their age, swapped for the fresh ones when Fleet's read of the
//! forge ends (`review_lists.changed`), and asked for again after a gap, which does not replay the event.

use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::PullReviewLists;

use super::{map, Pull, ReviewsScreen};
use crate::fleet::ReviewListsMoved;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::tip;

/// The head each pull request is at on the lists, by number. Two repositories' pull requests of one number keep the first.
pub(super) fn heads_of(lists: &PullReviewLists) -> HashMap<u64, String> {
    let mut heads = HashMap::new();
    for on in [&lists.requested, &lists.team, &lists.assigned, &lists.mine, &lists.reviewing] {
        for row in on {
            heads.entry(row.number).or_insert_with(|| row.head.clone());
        }
    }
    heads
}

/// Whether the page in front is read again for a fresh list: never for a head that did not move, so a swap of rows
/// under him does not redraw what he is reading. A page never read, or a row that names no head, is read.
pub(super) fn needs_reread(fresh: Option<&str>, held: Option<&str>) -> bool {
    match (fresh, held) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(fresh), Some(held)) => fresh.is_empty() || fresh != held,
    }
}

/// Whether lists read at `fetched_at` are older than the ones held, so an answer that was overtaken by its own
/// event does not put the rows back. A time that cannot be read is never older.
pub(super) fn overtaken(fetched_at: &str, held: Option<&str>) -> bool {
    let parse = |at: &str| chrono::DateTime::parse_from_rfc3339(at).ok();
    match (parse(fetched_at), held.and_then(parse)) {
        (Some(fresh), Some(held)) => fresh < held,
        _ => false,
    }
}

/// How long ago the lists were read, `Read 4m ago`: nothing for a read of a few seconds, or a time that cannot be read.
pub(super) fn read_age(fetched_at: &str, now: chrono::DateTime<chrono::Local>) -> Option<String> {
    let at = chrono::DateTime::parse_from_rfc3339(fetched_at).ok()?;
    let seconds = (now.with_timezone(&chrono::Utc) - at.with_timezone(&chrono::Utc)).num_seconds().max(0);
    (seconds >= SHOW_READ_AGE_AFTER).then(|| crate::merge_strip::words::read_said(seconds))
}

/// A read this young says nothing of its age: it is just now.
const SHOW_READ_AGE_AFTER: i64 = 5;


/// The banner over rows kept while the read behind them failed: Fleet's sentence, the time to try again where it named one,
/// and when the rows were read.
pub(super) fn failed_banner(failed: &ipc::ReviewListsFailed, fetched_at: Option<&str>) -> String {
    let until = failed.until.as_ref().and_then(|until| crate::settings::time::stamp_at(until.as_str()));
    let said = failed.said.trim_end_matches('.');
    let said = match until {
        Some(until) => format!("{said} until {until}"),
        None => said.to_string(),
    };
    match fetched_at.and_then(crate::settings::time::stamp_at) {
        Some(from) => format!("{said}. These are from {from}."),
        None => format!("{said}."),
    }
}

impl ReviewsScreen {
    /// Lists read, from Fleet's answer or its event: their rows swap in under what he has selected and scrolled to,
    /// which are held by pull request number and by the list's own offset, so nothing keyed by row position is reset.
    /// A page in front is read again only where its head moved. While Fleet's read of the forge is behind them the
    /// inbox stays marked, until the event lands.
    pub(super) fn take_lists(&mut self, lists: PullReviewLists, window: &mut Window, cx: &mut Context<Self>) {
        if overtaken(&lists.fetched_at, self.live.as_ref().and_then(|live| live.fetched_at.as_deref())) {
            return;
        }
        self.me = lists.me.clone();
        self.refreshed = true;
        self.inbox.refreshing = lists.refreshing;
        let (mut rows, found) = map::inbox_of(&lists);
        let heads = heads_of(&lists);
        if let Some(live) = self.live.as_mut() {
            live.inbox_said = None;
            live.inbox_banner = map::kept_banner(&lists);
            live.banner_limited = live.inbox_banner.is_some();
            live.fetched_at = Some(lists.fetched_at.clone()).filter(|at| !at.is_empty());
            for (number, row) in found {
                match live.pulls.get_mut(&number) {
                    Some(pull) => pull.row = row,
                    None => {
                        live.pulls.insert(number, Pull { row, facts: map::Facts::default(), notes: Vec::new(), finds: map::Finds::default() });
                    }
                }
            }
        }
        // A review he started here marks its row even where the lists were read before Fleet kept the run.
        for row in rows.iter_mut() {
            row.reviewing |= self.ran_here.contains(&row.number);
        }
        self.inbox.rows = rows;
        if let Some(number) = self.open {
            let held = self.live.as_ref().and_then(|live| live.pulls.get(&number)).and_then(|pull| pull.facts.head.clone());
            if needs_reread(heads.get(&number).map(String::as_str), held.as_deref()) {
                self.live_read(number, window, cx);
            }
        }
        self.tick_age(cx);
        cx.notify();
    }

    /// `review_lists.changed`: the read behind kept lists ended. Fresh lists swap in; a failure leaves the rows as they
    /// are, stops the mark and says why over them. A gap may have lost it, so the lists are asked for again.
    pub(super) fn lists_moved(&mut self, moved: &ReviewListsMoved, window: &mut Window, cx: &mut Context<Self>) {
        // This screen asks for every project's lists.
        if let Some(changed) = moved.changed.iter().rev().find(|changed| changed.project.is_none()) {
            if let Some(lists) = changed.lists.clone() {
                self.take_lists(lists, window, cx);
            } else if let Some(failed) = &changed.failed {
                self.inbox.refreshing = false;
                self.refreshed = true;
                if let Some(live) = self.live.as_mut() {
                    live.inbox_banner = Some(failed_banner(failed, live.fetched_at.as_deref()));
                    live.banner_limited = failed.code == ipc::REVIEW_LISTS_RATE_LIMITED;
                }
            }
            self.read_again_when_limit_lifts(window, cx);
            cx.notify();
        }
        if moved.lost {
            self.live_refresh(window, cx);
        }
    }

    /// How old the lists on screen are, for the header: nothing for a read of a few seconds.
    pub(super) fn live_read_age(&self) -> Option<String> {
        read_age(self.live.as_ref()?.fetched_at.as_deref()?, crate::snooze::now())
    }

    /// The age in the header is drawn again every so often, since nothing else moves it.
    fn tick_age(&mut self, cx: &mut Context<Self>) {
        if self.live.as_ref().is_none_or(|live| live.tasks.contains_key("age")) {
            return;
        }
        let task = cx.spawn(async move |this, cx| {
            while let Ok(wait) = this.update(cx, |this, cx| this.wait(crate::reviews::AGE_TICK, cx)) {
                wait.await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        if let Some(live) = self.live.as_mut() {
            live.tasks.insert("age".into(), task);
        }
    }

    /// The age in the header, beside the mark that says a read is behind the lists.
    pub(in crate::reviews) fn read_age_mark(&self, look: &Look) -> Option<AnyElement> {
        let age = self.live_read_age()?;
        Some(
            labelled(
                div().id("inbox-read-age").text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip("When Fleet last read the forge's lists")).child(SharedString::from(age.clone())),
                Role::Label,
                age,
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};

    use super::*;
    use crate::fleet::FleetConnection;
    use crate::reviews::ReviewsScreen;

    const LIMIT: &str = "The forge is limiting us";

    fn at(stamp: &str) -> chrono::DateTime<chrono::Local> {
        chrono::DateTime::parse_from_rfc3339(stamp).unwrap().with_timezone(&chrono::Local)
    }

    #[test]
    fn the_lists_say_how_old_they_are_once_a_few_seconds_have_gone() {
        let read = "2026-11-18T10:00:00Z";
        assert_eq!(read_age(read, at("2026-11-18T10:00:03Z")), None, "a read of seconds is just now, and says nothing");
        assert_eq!(read_age(read, at("2026-11-18T10:00:40Z")), Some("Read 40s ago".into()));
        assert_eq!(read_age(read, at("2026-11-18T10:04:10Z")), Some("Read 4m ago".into()));
        assert_eq!(read_age(read, at("2026-11-18T12:05:00Z")), Some("Read 2h 05m ago".into()));
        assert_eq!(read_age("", at("2026-11-18T10:04:10Z")), None);
        assert_eq!(read_age(read, at("2026-11-18T09:59:00Z")), None, "a clock behind Fleet's says nothing rather than a negative age");
    }

    #[test]
    fn a_page_is_read_again_only_where_its_head_moved() {
        assert!(!needs_reread(Some("abc"), Some("abc")));
        assert!(needs_reread(Some("def"), Some("abc")));
        assert!(needs_reread(Some("abc"), None), "a page never read is read");
        assert!(needs_reread(Some(""), Some("abc")), "a row that names no head says nothing of the page's");
        assert!(!needs_reread(None, Some("abc")), "a pull request that left the lists keeps the page he has");
    }

    #[test]
    fn an_answer_older_than_the_lists_held_is_overtaken() {
        let held = Some("2026-11-18T10:00:30Z");
        assert!(overtaken("2026-11-18T10:00:00Z", held));
        assert!(!overtaken("2026-11-18T10:00:30Z", held));
        assert!(!overtaken("2026-11-18T10:01:00Z", held));
        assert!(!overtaken("2026-11-18T10:00:00Z", None));
        assert!(!overtaken("", held));
    }

    #[test]
    fn a_failed_read_says_its_sentence_and_when_the_rows_kept_were_read() {
        let from = crate::settings::time::stamp_at("2026-11-18T09:40:00Z").unwrap();
        let until = crate::settings::time::stamp_at("2026-11-18T10:02:00Z").unwrap();
        let failed = |said: &str, until: Option<&str>| ipc::ReviewListsFailed { code: "fleet.review_lists_rate_limited".into(), said: said.into(), until: until.map(ipc::Instant::carried) };
        assert_eq!(failed_banner(&failed(LIMIT, Some("2026-11-18T10:02:00Z")), Some("2026-11-18T09:40:00Z")), format!("{LIMIT} until {until}. These are from {from}."));
        assert_eq!(failed_banner(&failed("The forge did not answer.", None), Some("2026-11-18T09:40:00Z")), format!("The forge did not answer. These are from {from}."));
        assert_eq!(failed_banner(&failed("The forge did not answer.", None), None), "The forge did not answer.");
    }

    fn row(number: u64, head: &str) -> ipc::PullReviewRow {
        ipc::PullReviewRow {
            manifest_id: ipc::ManifestId::carried("ledger-api"),
            repository: "harbor/ledger-api".into(),
            number,
            title: format!("Pull {number}"),
            branch: "mira/fix".into(),
            address: String::new(),
            draft: false,
            checks: ipc::PullReviewChecks::Passed,
            review: ipc::PullRequestReviewState::None,
            head: head.into(),
            author: "mira".into(),
            updated_at: "2026-11-18T09:00:00Z".into(),
            asked_at: None,
            team: None,
            holder: None,
            seen: None,
            my_review: None,
            inbox: ipc::PullInbox::NeedsFirstLook,
            reviewing: false,
        }
    }

    fn lists(rows: Vec<ipc::PullReviewRow>, fetched_at: &str, refreshing: bool) -> PullReviewLists {
        PullReviewLists {
            me: "nick".into(),
            fetched_at: fetched_at.into(),
            requested: rows,
            team: Vec::new(),
            assigned: Vec::new(),
            mine: Vec::new(),
            reviewing: Vec::new(),
            silent: Vec::new(),
            rate_limited: None,
            refreshing,
        }
    }

    fn screen(cx: &mut TestAppContext) -> (Entity<ReviewsScreen>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let fleet = cx.update(|cx| cx.new(FleetConnection::idle));
        cx.add_window_view(|window, cx| ReviewsScreen::live(&fleet, window, cx))
    }

    fn take(reviews: &Entity<ReviewsScreen>, lists: PullReviewLists, cx: &mut gpui::VisualTestContext) {
        cx.update(|window, cx| reviews.update(cx, |reviews, cx| reviews.take_lists(lists, window, cx)));
    }

    fn moved(reviews: &Entity<ReviewsScreen>, changed: Vec<ipc::ReviewListsChanged>, lost: bool, cx: &mut gpui::VisualTestContext) {
        cx.update(|window, cx| reviews.update(cx, |reviews, cx| reviews.lists_moved(&ReviewListsMoved { changed, lost }, window, cx)));
    }

    fn numbers(reviews: &ReviewsScreen) -> Vec<u64> {
        reviews.inbox.rows.iter().map(|row| row.number).collect()
    }

    #[gpui::test]
    fn kept_rows_draw_at_once_and_the_fresh_ones_swap_in_under_his_selection_and_scroll(cx: &mut TestAppContext) {
        let (reviews, cx) = screen(cx);
        // Enough rows that the list scrolls.
        let kept: Vec<_> = (100..160).map(|number| row(number, "h")).collect();
        take(&reviews, lists(kept.clone(), "2026-11-18T09:00:00Z", true), cx);
        reviews.read_with(cx, |reviews, _| {
            assert_eq!(reviews.inbox.rows.len(), 60, "the kept rows are drawn at once");
            assert!(reviews.inbox.refreshing, "and the read behind them is said until its event lands");
        });
        // He has the 31st row selected, its page open, and the list scrolled down.
        reviews.update(cx, |reviews, _| {
            reviews.focused = Some(130);
            reviews.open = Some(130);
            reviews.inbox_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(-300.)));
        });
        cx.run_until_parked();
        let held = reviews.read_with(cx, |reviews, _| reviews.inbox_scroll.offset().y);
        assert!(held < gpui::px(0.), "the list is scrolled: {held:?}");
        // The fresh lists put a new pull request above them all and drop none.
        let mut rows = vec![row(9, "z9")];
        rows.extend(kept);
        moved(&reviews, vec![ipc::ReviewListsChanged::read(None, lists(rows, "2026-11-18T09:05:00Z", false))], false, cx);
        cx.run_until_parked();
        reviews.read_with(cx, |reviews, _| {
            assert_eq!(reviews.inbox.rows.len(), 61, "the fresh rows are in");
            assert!(!reviews.inbox.refreshing, "and the mark is gone");
            assert_eq!((reviews.focused, reviews.open), (Some(130), Some(130)), "his selection is held by number, whatever place it moved to");
            assert_eq!(reviews.inbox_scroll.offset().y, held, "and the scroll is where he left it");
        });
    }

    #[gpui::test]
    fn a_failed_read_keeps_the_rows_stops_the_mark_and_says_why(cx: &mut TestAppContext) {
        let (reviews, cx) = screen(cx);
        take(&reviews, lists(vec![row(2, "b2")], "2026-11-18T09:00:00Z", true), cx);
        let failed = ipc::ReviewListsFailed { code: "fleet.review_lists_unreadable".into(), said: "The forge did not answer.".into(), until: None };
        moved(&reviews, vec![ipc::ReviewListsChanged::failed(None, failed)], false, cx);
        reviews.read_with(cx, |reviews, _| {
            assert_eq!(numbers(reviews), [2], "the rows kept stay");
            assert!(!reviews.inbox.refreshing);
            let banner = reviews.live_inbox_banner().expect("a banner");
            assert!(banner.starts_with("The forge did not answer. These are from "), "{banner}");
            assert!(!reviews.live.as_ref().unwrap().banner_limited, "only the rate limit reads again by itself");
        });
        // The next read that lands clears it.
        take(&reviews, lists(vec![row(2, "b2")], "2026-11-18T09:10:00Z", false), cx);
        reviews.read_with(cx, |reviews, _| assert_eq!(reviews.live_inbox_banner(), None));
    }

    #[gpui::test]
    fn an_answer_overtaken_by_its_event_does_not_put_the_old_rows_back(cx: &mut TestAppContext) {
        let (reviews, cx) = screen(cx);
        moved(&reviews, vec![ipc::ReviewListsChanged::read(None, lists(vec![row(9, "z9")], "2026-11-18T09:05:00Z", false))], false, cx);
        take(&reviews, lists(vec![row(2, "b2")], "2026-11-18T09:00:00Z", true), cx);
        reviews.read_with(cx, |reviews, _| {
            assert_eq!(numbers(reviews), [9]);
            assert!(!reviews.inbox.refreshing, "no mark waits on an event that already came");
        });
    }

    #[gpui::test]
    fn the_lists_say_how_old_they_are_and_a_gap_asks_for_them_again(cx: &mut TestAppContext) {
        let (reviews, cx) = screen(cx);
        let long_ago = (chrono::Utc::now() - chrono::Duration::minutes(4)).to_rfc3339();
        take(&reviews, lists(vec![row(2, "b2")], &long_ago, true), cx);
        reviews.read_with(cx, |reviews, _| assert_eq!(reviews.live_read_age().as_deref(), Some("Read 4m ago")));
        let just_now = chrono::Utc::now().to_rfc3339();
        take(&reviews, lists(vec![row(2, "b2")], &just_now, false), cx);
        reviews.read_with(cx, |reviews, _| assert_eq!(reviews.live_read_age(), None, "a read of seconds says nothing of its age"));
        // An event alone asks nothing more of Fleet.
        moved(&reviews, vec![ipc::ReviewListsChanged::read(None, lists(vec![row(2, "b2")], &just_now, false))], false, cx);
        reviews.read_with(cx, |reviews, _| assert_eq!(reviews.live_inbox_said(), None));
        // A gap may have lost the event, which Fleet does not replay: the lists are asked for again, and with no
        // Fleet behind this screen the ask says so.
        moved(&reviews, Vec::new(), true, cx);
        reviews.read_with(cx, |reviews, _| assert_eq!(reviews.live_inbox_said().as_deref(), Some("Fleet is not connected")));
    }

    #[test]
    fn heads_come_from_every_list_and_the_first_row_of_a_number_wins() {
        let mut read = lists(vec![row(1, "a1"), row(2, "b2")], "", false);
        read.mine = vec![row(2, "other")];
        read.reviewing = vec![row(3, "c3")];
        let heads = heads_of(&read);
        assert_eq!((heads.get(&1).map(String::as_str), heads.get(&2).map(String::as_str), heads.get(&3).map(String::as_str)), (Some("a1"), Some("b2"), Some("c3")));
    }
}
