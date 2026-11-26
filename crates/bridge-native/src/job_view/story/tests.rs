use chrono::{DateTime, Duration, TimeZone, Utc};
use gpui::{div, prelude::*, Role, TestAppContext, VisualTestContext, Window};
use ipc::{ArtifactState, PullRequestCommit, SessionId};

use super::mock::{self, JOB};
use super::model::{self, Built, Card, Entry, Filter, Kind, Link, Merge, Spent, Who};
use super::{Region, StoryEvent, StoryTab};
use crate::features::overlay::{Content, FeatureOverlay, OverlayEvent, Source};
use crate::features::page::FeaturePageEvent;
use crate::features::story_link::{self, StoryRequest};

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

/// The moment the mock story is read at.
fn now() -> DateTime<Utc> {
    at(17, 12)
}

fn built() -> Built {
    model::build(&mock::story(JOB), &mock::titles(), &mock::features(), now())
}

fn keys(built: &Built) -> Vec<&str> {
    built.entries.iter().map(|entry| entry.key.as_str()).collect()
}

fn entry(key: &str, kind: Kind, at: DateTime<Utc>, waiting: bool) -> Entry {
    Entry { key: key.into(), kind, at, title: key.into(), says: String::new(), waiting, when: String::new(), spent: Spent::NONE, card: Card::default() }
}

// ---- the model -------------------------------------------------------------------------------

#[test]
fn the_rows_stand_oldest_first_whatever_ask_they_belong_to_with_fleets_order_breaking_a_tie() {
    let built = built();
    assert_eq!(
        keys(&built),
        [
            "ask-ask-1",
            "ask-ask-2",
            "handed-ask-2-2026-11-14T12:00:00Z",
            "shown-sketches/votes",
            "handed-ask-3-2026-11-15T09:00:00Z",
            "shown-documents/feature-catalog",
            "ask-ask-3",
            "built-3720"
        ]
    );
    let mut flipped = mock::story(JOB);
    flipped.asks.reverse();
    let again = model::build(&flipped, &mock::titles(), &mock::features(), now());
    assert!(again.entries.windows(2).all(|pair| pair[0].at <= pair[1].at), "the same order from any order Fleet gave");
}

#[test]
fn what_he_was_shown_before_any_ask_stands_apart_and_is_not_in_the_timeline() {
    let built = built();
    assert_eq!(built.before.iter().map(|entry| entry.title.as_str()).collect::<Vec<_>>(), ["Start card with the model picker"]);
    assert!(built.entries.iter().all(|entry| entry.key != built.before[0].key));
}

#[test]
fn a_time_is_now_then_minutes_hours_and_days_ago() {
    let t = at(17, 12);
    assert_eq!(model::relative(t, t), "now");
    assert_eq!(model::relative(t, t - Duration::seconds(59)), "now");
    assert_eq!(model::relative(t, t - Duration::minutes(5)), "5m ago");
    assert_eq!(model::relative(t, t - Duration::minutes(180)), "3h ago");
    assert_eq!(model::relative(t, t - Duration::hours(48)), "2d ago");
    assert_eq!(model::relative(t, t + Duration::minutes(5)), "now", "a time ahead of the clock is now");
}

#[test]
fn a_portion_is_said_in_minutes_hours_and_days_and_a_portion_with_no_end_says_nothing() {
    let said: Vec<String> = [0, 20, 60, 125, 190, 1440, 2880].into_iter().map(model::spoken).collect();
    assert_eq!(said, ["<1m", "20m", "1h", "2h 5m", "3h 10m", "1d", "2d"]);
    assert_eq!(Spent::NONE.said(), "");
}

#[test]
fn a_portion_runs_to_the_next_row_and_the_last_runs_to_now_only_where_it_waits_on_him() {
    let mut rows = vec![entry("a", Kind::You, at(17, 8), false), entry("b", Kind::Artifact, at(17, 9), false), entry("c", Kind::Pull, at(17, 11), false)];
    model::durations(&mut rows, now());
    let minutes: Vec<Option<i64>> = rows.iter().map(|row| row.spent.minutes).collect();
    assert_eq!(minutes, [Some(60), Some(120), None], "the last has no end: nothing waits on him");
    assert_eq!(rows.iter().map(|row| row.spent.who).collect::<Vec<_>>(), [Who::Job, Who::You, Who::Job], "time after what he was shown is his");
    assert_eq!(rows.iter().map(|row| row.spent.share).collect::<Vec<_>>(), [50, 100, 0], "the bar is the share of the longest");
    let mut waiting = vec![entry("a", Kind::Artifact, at(17, 11), true)];
    model::durations(&mut waiting, now());
    assert_eq!(waiting[0].spent.minutes, Some(60), "a row that waits on him runs to now");
    assert_eq!(waiting[0].spent.share, 100);
}

#[test]
fn a_short_portion_still_draws_a_bar() {
    let mut rows = vec![entry("a", Kind::You, at(17, 0), false), entry("b", Kind::You, at(17, 1), false), entry("c", Kind::You, at(17, 11), false)];
    model::durations(&mut rows, now());
    assert_eq!(rows[0].spent.share, 10);
    let mut rows = vec![entry("a", Kind::You, at(17, 0), false), entry("b", Kind::You, at(17, 0) + Duration::minutes(1), false), entry("c", Kind::You, at(17, 11), false)];
    model::durations(&mut rows, now());
    assert_eq!(rows[0].spent.share, 4, "never less than a sliver");
}

#[test]
fn the_filters_are_all_then_each_kind_the_story_holds_and_a_filter_shows_only_its_kind() {
    let built = built();
    assert_eq!(built.filters(), [Filter::All, Filter::Only(Kind::You), Filter::Only(Kind::Job), Filter::Only(Kind::Artifact), Filter::Only(Kind::Pull)]);
    assert_eq!(built.visible(Filter::All).len(), 8);
    let only = |kind| built.visible(Filter::Only(kind)).into_iter().map(|at| built.entries[at].key.clone()).collect::<Vec<_>>();
    assert_eq!(only(Kind::Pull), ["built-3720"]);
    assert_eq!(only(Kind::Artifact), ["shown-sketches/votes", "shown-documents/feature-catalog"], "the artifacts of the asks, not the group before any");
    assert!(built.before_shown(Filter::All) && built.before_shown(Filter::Only(Kind::Artifact)) && !built.before_shown(Filter::Only(Kind::You)));
    let mut bare = built.clone();
    bare.entries.retain(|entry| entry.kind != Kind::Job);
    assert!(!bare.filters().contains(&Filter::Only(Kind::Job)), "a kind with no row has no tab");
}

#[test]
fn an_artifact_says_what_he_did_in_plain_words_and_a_status_is_never_a_bare_word() {
    let built = built();
    let said = |key: &str| built.entries.iter().find(|entry| entry.key == key).map(|entry| entry.says.clone()).expect(key);
    assert_eq!(said("shown-sketches/votes"), "You sent it back with a note");
    assert_eq!(said("shown-documents/feature-catalog"), "You approved it");
    assert_eq!(built.before[0].says, "You dismissed it");
    assert_eq!(said("built-3720"), "Merged");
    assert_eq!(said("ask-ask-1"), "Open: decided kinds as sections");
    for state in [ArtifactState::Approved, ArtifactState::SentBack, ArtifactState::Dismissed, ArtifactState::Asking, ArtifactState::Reading, ArtifactState::Withdrawn] {
        assert!(model::said_of(state).split_whitespace().count() >= 2, "{state:?} is a sentence part, not a bare word");
    }
}

#[test]
fn an_asks_card_is_his_words_in_bubbles_and_the_features_it_is_filed_under() {
    let built = built();
    let card = &built.entries[built.find("ask-1").expect("the ask")].card;
    let said: Vec<(bool, &str)> = card.bubbles.iter().map(|bubble| (bubble.mine, bubble.said.as_str())).collect();
    assert_eq!(said, [(true, "every feature gets a page the agents read before they touch its files"), (false, "Showed you the document Feature catalog design"), (true, "decided kinds as sections")]);
    assert_eq!(card.filed.iter().map(|filed| filed.title.as_str()).collect::<Vec<_>>(), ["Start a Job", "Reviews"]);
    assert_eq!(card.stand.map(|stand| stand.word), Some("Open"));
}

#[test]
fn an_asks_card_alternates_his_words_and_the_jobs_in_time_order() {
    let built = built();
    let card = &built.entries[built.find("ask-3").expect("the ask")].card;
    let said: Vec<(bool, &str)> = card.bubbles.iter().map(|bubble| (bubble.mine, bubble.said.as_str())).collect();
    assert_eq!(
        said,
        [(true, "yes, per revision"), (false, "Showed you the sketch Votes per revision"), (false, "Passed it to One artifact that grows"), (true, "they show as i never voted"), (false, "Merged pull request #3720")]
    );
    assert!(card.bubbles.iter().filter(|bubble| !bubble.mine).all(|bubble| bubble.who == "Feature Catalog"));
}

#[test]
fn the_harness_wrappers_come_off_his_words() {
    let sent = r#"Sent back part "story-over-time" of "Job story over time". A is close but lets refine it Change it and put it again with add_part under the same id."#;
    assert_eq!(model::his_words(sent), "A is close but lets refine it");
    assert_eq!(model::his_words(r#"On your preview note "the card": We are working through some changes now"#), "We are working through some changes now");
    assert_eq!(model::his_words("a plain message, said as it was"), "a plain message, said as it was");
    assert_eq!(model::his_words(r#"Sent back part "a" of "b". Change it and put it again with add_part under the same id."#), "");
}

#[test]
fn an_artifacts_card_is_the_job_showing_it_then_his_words_votes_and_presses() {
    let built = built();
    let card = &built.entries.iter().find(|entry| entry.key == "shown-sketches/votes").expect("shown").card;
    let said: Vec<(bool, &str)> = card.bubbles.iter().map(|bubble| (bubble.mine, bubble.said.as_str())).collect();
    assert_eq!(said, [(false, "Showed you the sketch Votes per revision"), (true, "keep each take's votes"), (true, "Pressed start on take-a")]);
    assert_eq!(card.bubbles[0].who, "Feature Catalog", "the Job is named by its title");
    assert_eq!(card.bubbles[1].when, "", "when his note was written is not kept, so none is drawn");
    assert_eq!(card.bubbles[2].when, "3d ago");
}

#[test]
fn a_hand_names_the_job_and_the_ask_it_was_about() {
    let built = built();
    let hand = built.entries.iter().find(|entry| entry.key == "handed-ask-2-2026-11-14T12:00:00Z").expect("handed");
    assert_eq!(hand.title, "Handed to One artifact that grows");
    assert_eq!(hand.card.about, "A reading page never shows");
    assert_eq!(hand.card.job.as_ref().map(|(_, title)| title.as_str()), Some("One artifact that grows"));
    assert_eq!(hand.kind, Kind::Job);
}

fn commit(sha: &str, headline: &str, when: Option<&str>) -> PullRequestCommit {
    PullRequestCommit { sha: sha.into(), headline: headline.into(), author: None, at: when.map(str::to_string) }
}

#[test]
fn a_pull_requests_history_is_its_commits_by_when_then_its_merge() {
    let merge = Merge { number: 3720, at: at(15, 15) };
    let commits = [commit("bbbbbbb2222", "Second", Some("2026-11-15T10:00:00Z")), commit("aaaaaaa1111", "First", Some("2026-11-15T08:00:00Z")), commit("ccccccc3333", "Unstamped", None)];
    let events = model::pull_events(&merge, &commits, now());
    assert_eq!(events.iter().map(|event| event.said.as_str()).collect::<Vec<_>>(), ["Unstamped", "First", "Second", "Merged"]);
    assert_eq!(events[1].sha.as_deref(), Some("aaaaaaa"), "the short sha");
    assert_eq!(events[1].when, "2d ago");
    assert_eq!(events[1].link, Some(Link::Commit { number: 3720, sha: "aaaaaaa1111".into(), headline: "First".into() }));
    assert_eq!(events[3].link, None, "the merge goes nowhere");
}

#[test]
fn the_presses_of_an_opened_row_are_its_features_then_its_commits_then_the_pull_request() {
    let built = built();
    let ask = &built.entries[built.find("ask-1").expect("the ask")];
    assert_eq!(ask.links(), [Link::Feature("start-a-job".into()), Link::Feature("reviews".into())]);
    let pull = built.entries.iter().find(|entry| entry.kind == Kind::Pull).expect("a pull request");
    assert_eq!(pull.links_with(&[]), [Link::Pull(3720)], "before its commits are read");
    assert_eq!(pull.links_with(&[commit("abc1234", "One", None)]), [Link::Commit { number: 3720, sha: "abc1234".into(), headline: "One".into() }, Link::Pull(3720)]);
}

#[test]
fn the_keys_start_on_the_newest_row_that_waits_on_him_else_the_newest() {
    let mut built = built();
    assert_eq!(built.newest(Filter::All), Some(7));
    built.entries[3].waiting = true;
    assert_eq!(built.newest(Filter::All), Some(3));
    assert_eq!(built.newest(Filter::Only(Kind::Pull)), Some(7), "under a filter, among what it shows");
    assert_eq!(built.newest(Filter::Only(Kind::Job)), Some(4));
}

#[test]
fn the_features_the_job_is_on_keep_fleets_order_and_say_only_the_ways_it_touched_each() {
    let built = built();
    let said: Vec<(&str, &str)> = built.features.iter().map(|line| (line.title.as_str(), line.how.as_str())).collect();
    assert_eq!(said, [("Start a Job", "wrote 12 files · named in 9 commits · 2 asks filed"), ("Thread and composer", "named in 3 commits · 1 ask filed"), ("Reviews", "1 ask filed")]);
}

#[test]
fn the_done_cards_lead_is_a_line_an_ask_each_with_its_asks_state() {
    let lead = model::lead(&mock::story(JOB));
    assert_eq!(lead.iter().map(|(stand, _)| stand.word).collect::<Vec<_>>(), ["Open", "Handed", "Fixed"]);
}

// ---- the tab ---------------------------------------------------------------------------------

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        // A request an earlier test left is not this test's.
        story_link::take(cx);
    });
}

fn open(cx: &mut TestAppContext) -> (gpui::Entity<StoryTab>, &mut VisualTestContext) {
    ready(cx);
    let (tab, cx) = cx.add_window_view(|window, cx| StoryTab::mock(JOB.into(), false, window, cx));
    cx.update(|window, cx| {
        let focus = tab.read(cx).focus.clone();
        window.focus(&focus, cx)
    });
    cx.run_until_parked();
    (tab, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn unfolded(tab: &gpui::Entity<StoryTab>, cx: &mut VisualTestContext) -> Option<String> {
    tab.read_with(cx, |tab, _| tab.unfolded.clone())
}

fn cursor(tab: &gpui::Entity<StoryTab>, cx: &mut VisualTestContext) -> Option<usize> {
    tab.read_with(cx, |tab, _| tab.cursor)
}

#[gpui::test]
fn the_keys_start_on_the_newest_row_with_nothing_opened_and_enter_opens_and_closes_one_at_a_time(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    assert_eq!((unfolded(&tab, cx), cursor(&tab, cx)), (None, Some(7)));
    cx.simulate_keystrokes("k k");
    assert_eq!(cursor(&tab, cx), Some(5));
    assert_eq!(unfolded(&tab, cx), None, "moving opens nothing");
    cx.simulate_keystrokes("enter");
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("shown-documents/feature-catalog"));
    cx.simulate_keystrokes("k enter");
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("handed-ask-3-2026-11-15T09:00:00Z"), "one at a time");
    cx.simulate_keystrokes("enter");
    assert_eq!(unfolded(&tab, cx), None);
    cx.simulate_keystrokes("down down down down down");
    assert_eq!(cursor(&tab, cx), Some(8), "past the timeline it goes on into the group he was shown before any ask");
    cx.simulate_keystrokes("down");
    assert_eq!(cursor(&tab, cx), Some(8), "and stops at the last");
}

#[gpui::test]
fn a_row_that_is_opened_stays_opened_when_the_story_is_read_again(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_keystrokes("enter");
    let key = unfolded(&tab, cx);
    assert!(key.is_some());
    tab.update(cx, |tab, cx| tab.rebuild(cx));
    assert_eq!(unfolded(&tab, cx), key);
}

#[gpui::test]
fn tab_goes_from_the_rows_to_the_filters_and_arrows_there_pick_a_kind_which_opening_a_row_keeps(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_keystrokes("tab");
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Filters);
    cx.simulate_keystrokes("right");
    assert_eq!(tab.read_with(cx, |tab, _| tab.filter), Filter::Only(Kind::You));
    assert_eq!(tab.read_with(cx, |tab, _| tab.rows().len()), 3, "the three asks");
    cx.simulate_keystrokes("right right right");
    assert_eq!(tab.read_with(cx, |tab, _| tab.filter), Filter::Only(Kind::Pull));
    cx.simulate_keystrokes("right");
    assert_eq!(tab.read_with(cx, |tab, _| tab.filter), Filter::Only(Kind::Pull), "it stops at the last");
    cx.simulate_keystrokes("left left");
    assert_eq!(tab.read_with(cx, |tab, _| tab.filter), Filter::Only(Kind::Job));
    cx.simulate_keystrokes("tab enter");
    assert!(unfolded(&tab, cx).is_some());
    assert_eq!(tab.read_with(cx, |tab, _| tab.filter), Filter::Only(Kind::Job), "opening a row keeps the filter");
    assert_eq!(tab.read_with(cx, |tab, _| tab.rows().len()), 2);
    assert!(drawn(cx, Role::Tab, "Jobs") && drawn(cx, Role::Tab, "Artifacts") && drawn(cx, Role::Tab, "PRs"));
}

#[gpui::test]
fn a_filter_goes_when_the_story_no_longer_holds_its_kind(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    tab.update(cx, |tab, cx| tab.filter_by(Filter::Only(Kind::Pull), cx));
    tab.update(cx, |tab, cx| {
        if let super::Load::Read(story) = &mut tab.story {
            story.asks[2].built.clear();
        }
        tab.rebuild(cx);
    });
    assert_eq!(tab.read_with(cx, |tab, _| tab.filter), Filter::All);
}

#[gpui::test]
fn tab_goes_to_the_presses_of_the_opened_row_and_enter_on_a_feature_opens_it_where_esc_goes_back(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_keystrokes("k k k k k k k enter");
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("ask-ask-1"));
    cx.simulate_keystrokes("tab");
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Links);
    cx.simulate_keystrokes("j");
    assert_eq!(tab.read_with(cx, |tab, _| tab.link_at), 1);
    cx.simulate_keystrokes("j");
    assert_eq!(tab.read_with(cx, |tab, _| tab.link_at), 1, "two features, so it stops at the second");
    cx.simulate_keystrokes("escape");
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Rows);
    cx.simulate_keystrokes("tab enter");
    let overlay = tab.read_with(cx, |tab, _| tab.overlay().cloned()).expect("the feature opened over the tab");
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.titles().map(|(title, from)| (title.to_string(), from.to_string()))), Some(("Start a Job".to_string(), "Story of Feature Catalog".to_string())));
    cx.simulate_keystrokes("escape");
    assert!(tab.read_with(cx, |tab, _| tab.overlay().is_none()), "the last Esc closes the overlay");
}

#[gpui::test]
fn a_story_link_puts_its_ask_in_front_when_the_tab_first_draws_whatever_the_filter_and_one_for_another_job_is_left_alone(cx: &mut TestAppContext) {
    ready(cx);
    cx.update(|cx| story_link::put(StoryRequest { session_id: SessionId::carried("another-job"), ask_id: "ask-3".into() }, cx));
    let (tab, cx) = cx.add_window_view(|window, cx| StoryTab::mock(JOB.into(), false, window, cx));
    assert_eq!(unfolded(&tab, cx), None, "another Job's request is not this tab's");
    assert!(cx.update(|_, cx| story_link::peek(cx)).is_some_and(|request| request.session_id.as_str() == "another-job"), "and stays for that Job's tab");
    cx.update(|_, cx| story_link::take(cx));
    cx.update(|_, cx| story_link::put(StoryRequest { session_id: SessionId::carried(JOB), ask_id: "ask-3".into() }, cx));
    let (named, cx) = cx.add_window_view(|window, cx| StoryTab::mock(JOB.into(), false, window, cx));
    assert_eq!(unfolded(&named, cx).as_deref(), Some("ask-ask-3"));
    assert_eq!(cursor(&named, cx), Some(6));
    assert!(cx.update(|_, cx| story_link::peek(cx)).is_none(), "it was taken");
    named.update(cx, |tab, cx| tab.filter_by(Filter::Only(Kind::Pull), cx));
    named.update(cx, |tab, cx| tab.put_ask("ask-1".into(), cx));
    assert_eq!((named.read_with(cx, |tab, _| tab.filter), unfolded(&named, cx).as_deref().map(str::to_string)), (Filter::All, Some("ask-ask-1".into())), "the ask is shown whatever the filter was");
}

#[gpui::test]
fn the_tab_draws_a_role_and_a_name_for_every_row_and_the_opened_card(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_resize(gpui::size(gpui::px(900.), gpui::px(2400.)));
    cx.simulate_keystrokes("k k k k k k k enter");
    let names: Vec<String> = tab.read_with(cx, |tab, _| tab.built.entries.iter().map(|entry| if entry.says.is_empty() { format!("{}, {}", entry.title, entry.when) } else { format!("{}, {}, {}", entry.title, entry.says, entry.when) }).collect());
    for _ in 0..2 {
        tab.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    let missing: Vec<&String> = names.iter().filter(|name| !drawn(cx, Role::Button, name)).collect();
    assert!(missing.is_empty(), "a row is a button named as it reads: {missing:?}");
    assert!(drawn(cx, Role::ListBox, "Story"));
    assert!(drawn(cx, Role::Tab, "All") && drawn(cx, Role::TabList, "Story filter"));
    assert!(drawn(cx, Role::Image, "You"), "the glyph of a row is named by who it is with");
    assert!(drawn(cx, Role::Link, "Filed under Reviews"), "a Filed under row is a link");
    assert!(drawn(cx, Role::Status, "Open, A Job is building it"), "the pill says what it means");
    assert!(drawn(cx, Role::Heading, "Shown to you"), "the leftover group is headed Shown to you");
}

#[gpui::test]
fn the_group_before_any_ask_has_places_only_where_there_is_one_and_follows_the_filter(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 10, "eight rows, the heading and one row");
    tab.update(cx, |tab, cx| tab.filter_by(Filter::Only(Kind::You), cx));
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 3, "only the asks");
    tab.update(cx, |tab, cx| tab.filter_by(Filter::All, cx));
    tab.update(cx, |tab, cx| {
        if let super::Load::Read(story) = &mut tab.story {
            story.shown_otherwise.clear();
        }
        tab.rebuild(cx);
    });
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 8);
}

#[gpui::test]
fn opening_a_pull_request_asks_for_its_commits_once_and_a_press_on_a_commit_or_the_pull_request_goes_up(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    let heard: std::rc::Rc<std::cell::RefCell<Vec<String>>> = Default::default();
    let hear = heard.clone();
    let _sub = cx.update(|_, cx| {
        cx.subscribe(&tab, move |_, event: &StoryEvent, _| {
            hear.borrow_mut().push(match event {
                StoryEvent::ReadPull { number } => format!("read {number}"),
                StoryEvent::OpenPull { number, commit } => format!("open {number} {:?}", commit.as_ref().map(|(sha, _)| sha.clone())),
                StoryEvent::OpenJob(id) => format!("job {id}"),
                _ => "other".into(),
            })
        })
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(*heard.borrow(), ["read 3720"]);
    tab.update(cx, |tab, cx| tab.set_commits(3720, vec![commit("abc1234def", "Count votes per revision", Some("2026-11-15T10:00:00Z"))], cx));
    cx.run_until_parked();
    assert!(drawn(cx, Role::Button, "Count votes per revision"), "the commit is a button");
    assert!(drawn(cx, Role::Button, "Open pull request"));
    cx.simulate_keystrokes("enter");
    assert_eq!(unfolded(&tab, cx), None, "closed");
    cx.simulate_keystrokes("enter");
    assert_eq!(*heard.borrow(), ["read 3720"], "its commits are held, so they are not asked for again");
    cx.simulate_keystrokes("tab enter");
    assert_eq!(heard.borrow().last().map(String::as_str), Some("open 3720 Some(\"abc1234def\")"), "the first press is the commit");
    cx.simulate_keystrokes("j enter");
    assert_eq!(heard.borrow().last().map(String::as_str), Some("open 3720 None"));
}

#[gpui::test]
fn a_hand_offers_the_job_it_went_to(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    let heard: std::rc::Rc<std::cell::RefCell<Vec<String>>> = Default::default();
    let hear = heard.clone();
    let _sub = cx.update(|_, cx| {
        cx.subscribe(&tab, move |_, event: &StoryEvent, _| {
            if let StoryEvent::OpenJob(id) = event {
                hear.borrow_mut().push(id.clone());
            }
        })
    });
    cx.simulate_keystrokes("k k k enter");
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("handed-ask-3-2026-11-15T09:00:00Z"));
    assert!(drawn(cx, Role::Button, "Open One artifact that grows"));
    cx.simulate_keystrokes("tab enter");
    assert_eq!(*heard.borrow(), ["job-one-artifact"]);
}

#[gpui::test]
fn the_tab_draws_bones_while_it_reads_and_nothing_for_a_job_with_no_story(cx: &mut TestAppContext) {
    ready(cx);
    let (tab, cx) = cx.add_window_view(|window, cx| StoryTab::mock(JOB.into(), false, window, cx));
    tab.update(cx, |tab, cx| {
        tab.story = super::Load::Reading;
        tab.rebuild(cx);
    });
    cx.run_until_parked();
    assert!(drawn(cx, Role::Image, "Loading"));
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 0, "no row is built while it reads");
    tab.update(cx, |tab, cx| {
        tab.story = super::Load::Read(ipc::JobStory { session_id: SessionId::carried(JOB), asks: Vec::new(), shown_otherwise: Vec::new(), lead: Vec::new(), features: Vec::new() });
        tab.rebuild(cx);
    });
    cx.run_until_parked();
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 0, "an empty slot stays empty");
    assert_eq!(tab.read_with(cx, |tab, _| tab.built.filters()), [Filter::All], "no filters over nothing, so none are drawn");
    assert!(tab.read_with(cx, |tab, _| tab.story_is_read()), "and it is not bones");
}

#[gpui::test]
fn the_columns_of_every_row_line_up_and_a_long_title_wraps_inside_its_row(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_resize(gpui::size(gpui::px(520.), gpui::px(800.)));
    tab.update(cx, |tab, cx| {
        if let super::Load::Read(story) = &mut tab.story {
            story.asks[0].ask.title = "Schedule work or set up watcher scripts outside jobs, so that a person can leave a repository to be looked at on a timetable and not by hand".into();
        }
        tab.rebuild(cx);
    });
    for _ in 0..2 {
        tab.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    let bounds: Vec<gpui::Bounds<gpui::Pixels>> = cx.update(|window, cx| {
        cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().filter(|one| one.role == Role::ListBoxOption).map(|one| one.bounds).take(5).collect()
    });
    assert!(bounds.len() >= 3, "rows are drawn");
    assert!(bounds.iter().all(|row| row.right() <= gpui::px(520.)), "no row spills out of the tab: {bounds:?}");
    let stamps: Vec<gpui::Pixels> = cx.update(|window, cx| {
        cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().take(40).filter(|one| one.role == Role::Image && one.name.starts_with("Time on")).map(|one| one.bounds.right()).collect::<Vec<_>>()
    });
    assert!(stamps.windows(2).all(|pair| pair[0] == pair[1]), "the time column is one column: {stamps:?}");
}

// ---- the overlay -----------------------------------------------------------------------------

/// A window root that clears the label book each frame, as the app's does, so what is no longer drawn is gone from it.
struct Root(gpui::Entity<FeatureOverlay>);

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(self.0.clone()))
    }
}

fn overlay(cx: &mut TestAppContext) -> (gpui::Entity<FeatureOverlay>, &mut VisualTestContext) {
    ready(cx);
    let held: std::rc::Rc<std::cell::RefCell<Option<gpui::Entity<FeatureOverlay>>>> = Default::default();
    let keep = held.clone();
    let (_root, cx) = cx.add_window_view(move |window, cx| {
        let overlay = cx.new(|cx| FeatureOverlay::on_page(Source::Mock, crate::features::mock::PROJECT.into(), "Story of Feature Catalog".into(), "start-a-job".into(), "Start a Job".into(), window, cx));
        *keep.borrow_mut() = Some(overlay.clone());
        Root(overlay)
    });
    cx.run_until_parked();
    let overlay = held.borrow().clone().expect("the overlay");
    (overlay, cx)
}

fn kinds(overlay: &gpui::Entity<FeatureOverlay>, cx: &mut VisualTestContext) -> (usize, &'static str) {
    overlay.read_with(cx, |overlay, _| {
        (
            overlay.depth(),
            match overlay.top() {
                Some(Content::Page(_)) => "page",
                Some(Content::Story(_)) => "story",
                None => "none",
            },
        )
    })
}

#[gpui::test]
fn a_pages_story_link_replaces_what_the_overlay_draws_and_each_esc_goes_back_one(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    assert_eq!(kinds(&overlay, cx), (1, "page"));
    let page = overlay.read_with(cx, |overlay, _| match overlay.top() {
        Some(Content::Page(page)) => page.clone(),
        _ => panic!("a page"),
    });
    page.update(cx, |_, cx| cx.emit(FeaturePageEvent::Story { session_id: SessionId::carried(JOB), ask_id: "ask-3".into() }));
    cx.run_until_parked();
    assert_eq!(kinds(&overlay, cx), (2, "story"), "the Story is the top of the one stack");
    assert!(drawn(cx, Role::Heading, "Story of Feature Catalog"), "the title line names it");
    assert!(drawn(cx, Role::Button, "Back"), "one Back");
    assert_eq!(kinds(&overlay, cx), (2, "story"), "the page under it is not the top: no second layer");
    let story = overlay.read_with(cx, |overlay, _| match overlay.top() {
        Some(Content::Story(story)) => story.clone(),
        _ => panic!("a story"),
    });
    assert_eq!(unfolded(&story, cx).as_deref(), Some("ask-ask-3"), "at the ask the link named");
    // A Filed under row inside it opens a page on the same stack.
    cx.simulate_keystrokes("tab enter");
    assert_eq!(kinds(&overlay, cx), (3, "page"));
    cx.simulate_keystrokes("escape");
    assert_eq!(kinds(&overlay, cx), (2, "story"), "one Esc goes back one");
    cx.simulate_keystrokes("escape");
    assert_eq!(kinds(&overlay, cx), (2, "story"), "the Story's keys were on its Filed under row, which Esc leaves first");
    cx.simulate_keystrokes("escape");
    assert_eq!(kinds(&overlay, cx), (1, "page"));
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.titles().map(|(title, from)| (title.to_string(), from.to_string()))), Some(("Start a Job".to_string(), "Story of Feature Catalog".to_string())));
}

#[gpui::test]
fn back_goes_back_one_and_the_last_layer_closes_the_overlay(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    let closed = std::rc::Rc::new(std::cell::Cell::new(false));
    let seen = closed.clone();
    let _sub = cx.update(|_, cx| {
        cx.subscribe(&overlay, move |_, event: &OverlayEvent, _| {
            let OverlayEvent::Closed = event;
            seen.set(true);
        })
    });
    overlay.update_in(cx, |overlay, window, cx| overlay.push_story(SessionId::carried(JOB), "ask-1".into(), window, cx));
    assert_eq!(kinds(&overlay, cx), (2, "story"));
    cx.run_until_parked();
    let back = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Button, "Back").map(|drawn| drawn.bounds.center()));
    cx.simulate_click(back.expect("Back is drawn"), gpui::Modifiers::none());
    assert_eq!(kinds(&overlay, cx), (1, "page"));
    assert!(!closed.get());
    cx.simulate_keystrokes("escape");
    assert!(closed.get(), "the last Esc closes");
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.depth()), 0);
}

// ---- the done card's lead --------------------------------------------------------------------

struct Lead(String);

impl Render for Lead {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        use crate::look::ActiveLook as _;
        let look = cx.look().clone();
        div().size_full().children(crate::job_view::done::lead_lines(&self.0, &look, cx))
    }
}

#[gpui::test]
fn the_done_card_leads_with_a_line_an_ask_and_nothing_where_the_story_was_not_asked_for(cx: &mut TestAppContext) {
    ready(cx);
    let (view, cx) = cx.add_window_view(|_, _| Lead(JOB.into()));
    assert!(!drawn(cx, Role::Group, "What he asked for"), "nobody asked for the story yet");
    cx.update(|_, cx| super::lead::shown(JOB, cx));
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert!(drawn(cx, Role::Group, "What he asked for"));
    for line in ["Feature catalog: being built", "A reading page never shows: handed to One artifact that grows", "Votes show as never cast: fixed in #3720"] {
        assert!(drawn(cx, Role::ListItem, line), "{line}");
    }
}

fn story_moved(job: &str) -> crate::fleet::StoryMoved {
    crate::fleet::StoryMoved(vec![ipc::StoryChanged { session_id: SessionId::carried(job) }])
}

#[gpui::test]
fn a_story_that_moved_is_read_again_for_the_tab_of_that_job_and_for_no_other(cx: &mut TestAppContext) {
    ready(cx);
    let fleet = cx.new(crate::fleet::FleetConnection::idle);
    let (tab, cx) = cx.add_window_view({
        let fleet = fleet.clone();
        move |window, cx| StoryTab::live(&fleet, JOB.into(), false, window, cx)
    });
    cx.run_until_parked();
    let before = tab.read_with(cx, |tab, _| tab.reads);
    fleet.update(cx, |_, cx| cx.emit(story_moved("another-job")));
    assert_eq!(tab.read_with(cx, |tab, _| tab.reads), before, "another Job's story is not this tab's");
    fleet.update(cx, |_, cx| cx.emit(story_moved(JOB)));
    assert_eq!(tab.read_with(cx, |tab, _| tab.reads), before + 1, "this Job's story is read again");
    fleet.update(cx, |_, cx| cx.emit(crate::fleet::StoryMoved(vec![ipc::StoryChanged { session_id: SessionId::carried("another-job") }, ipc::StoryChanged { session_id: SessionId::carried(JOB) }])));
    assert_eq!(tab.read_with(cx, |tab, _| tab.reads), before + 2, "one batch naming both reads it once");
}

#[gpui::test]
fn a_story_that_moved_is_read_again_for_the_done_card_of_a_job_it_asked_for_and_for_no_other(cx: &mut TestAppContext) {
    ready(cx);
    let fleet = cx.new(crate::fleet::FleetConnection::idle);
    cx.update(|cx| super::lead::shown_on(JOB, Some(fleet.clone()), cx));
    let before = cx.update(|cx| super::lead::reads_in_tests(cx));
    assert_eq!(before, [JOB], "it was read once on coming on screen");
    cx.update(|cx| fleet.update(cx, |_, cx| cx.emit(story_moved("another-job"))));
    assert_eq!(cx.update(|cx| super::lead::reads_in_tests(cx)), [JOB], "a Job no card asked for is not read");
    cx.update(|cx| fleet.update(cx, |_, cx| cx.emit(story_moved(JOB))));
    assert_eq!(cx.update(|cx| super::lead::reads_in_tests(cx)), [JOB, JOB], "the Job named is read again");
}

// ---- a fixed tab of the Job's panes -----------------------------------------------------------

/// The Job's panes as a Job kept them before Story was a kind, kept under its key, and the Job
/// on screen on the mock Fleet.
fn job_with_kept_panes(kept: super::super::pane_tree::Arrangement, cx: &mut TestAppContext) -> (gpui::Entity<super::JobView>, &mut VisualTestContext) {
    use super::super::pane_store;
    ready(cx);
    cx.update(|cx| {
        pane_store::install(pane_store::MemoryStore::default(), cx);
        pane_store::write(&pane_store::job_key(JOB), &kept, cx);
    });
    let jobs = crate::mock::jobs("job-for-you").expect("the fixture decodes");
    let threads = crate::mock::threads("job-for-you").expect("the threads decode");
    let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
    let (view, cx) = cx.add_window_view({
        let list = list.clone();
        move |window, cx| super::JobView::new(list, Some(threads), window, cx)
    });
    list.update(cx, |list, cx| list.open(JOB.into(), cx));
    cx.run_until_parked();
    (view, cx)
}

/// The default panes as a Job that opened Plan and Files kept them before Story was a kind, with Story at the end of the
/// column's top where `story`, and the thread's pane and the top's id.
fn kept_with(story: bool) -> (super::super::pane_tree::Arrangement, String, String) {
    use super::super::pane_tree::{add_tab, close_tab, default_arrangement, holder_of, panes_of, Extras};
    let base = default_arrangement();
    let (thread, top) = (panes_of(&base.root)[0].id.clone(), panes_of(&base.root)[1].id.clone());
    let own = holder_of(&base, "story").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "story")).map(|tab| tab.id.clone()).expect("the default has Story");
    let closed = close_tab(&base, &own).into_owned();
    let mut kept = add_tab(&add_tab(&closed, &top, "plan", Extras::default()), &top, "files", Extras::default()).into_owned();
    if story {
        kept = add_tab(&kept, &top, "story", Extras::default()).into_owned();
    }
    // Kept before Story was a kind, so it was never offered.
    kept.offered.clear();
    (kept, thread, top)
}

fn top_kinds(view: &gpui::Entity<super::JobView>, top: &str, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |view, _| {
        let arrangement = view.arrangement_of(JOB).expect("the Job is placed");
        super::super::pane_tree::pane_by_id(arrangement, top).map(|pane| pane.tabs.iter().map(|tab| tab.kind.clone()).collect()).unwrap_or_default()
    })
}

#[gpui::test]
fn an_arrangement_kept_without_story_opens_with_story_after_issues(cx: &mut TestAppContext) {
    let (kept, thread, top) = kept_with(false);
    let (view, cx) = job_with_kept_panes(kept, cx);
    assert_eq!(top_kinds(&view, &thread, cx), ["thread"]);
    assert_eq!(top_kinds(&view, &top, cx), ["artifacts", "pulls", "issues", "story", "plan", "files"], "his own tabs stay after it");
}

#[gpui::test]
fn an_arrangement_kept_with_story_keeps_its_own_place_for_it(cx: &mut TestAppContext) {
    let (kept, thread, top) = kept_with(true);
    let (view, cx) = job_with_kept_panes(kept, cx);
    assert_eq!(top_kinds(&view, &top, cx), ["artifacts", "pulls", "issues", "plan", "files", "story"]);
    assert_eq!(top_kinds(&view, &thread, cx), ["thread"]);
}

#[gpui::test]
fn a_story_link_for_the_job_opens_its_story_tab_in_front_and_puts_its_ask_in_front(cx: &mut TestAppContext) {
    let (kept, _, top) = kept_with(false);
    cx.update(|cx| story_link::put(StoryRequest { session_id: SessionId::carried(JOB), ask_id: "ask-3".into() }, cx));
    let (view, cx) = job_with_kept_panes(kept, cx);
    // `ready` took the request an earlier test left, so it is put again once the Job is open and its panes are read.
    cx.update(|_, cx| story_link::put(StoryRequest { session_id: SessionId::carried(JOB), ask_id: "ask-3".into() }, cx));
    view.update_in(cx, |view, _, cx| view.open_for_link(JOB, cx));
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let front = view.read_with(cx, |view, _| {
        let arrangement = view.arrangement_of(JOB).expect("placed");
        super::super::pane_tree::pane_by_id(arrangement, &top).and_then(|pane| pane.active.clone().and_then(|id| pane.tabs.iter().find(|tab| tab.id == id).map(|tab| tab.kind.clone())))
    });
    assert_eq!(front.as_deref(), Some("story"), "the Story tab is in front of the column");
    assert!(cx.update(|_, cx| story_link::peek(cx)).is_none(), "the tab took the request");
    assert!(drawn(cx, Role::TabList, "Story filter"), "the tab is drawn in the pane");
}


// ---- the library's names ----------------------------------------------------------------------

/// A live tab of a Job of the mock project `armada`, on a connection with no link behind it, and the Job's id.
fn live_in_project(fleet: &gpui::Entity<crate::fleet::FleetConnection>, at: usize, cx: &mut TestAppContext) -> (gpui::Entity<StoryTab>, String) {
    let record = crate::mock::jobs_of("jobs-projects").expect("the fixture decodes").into_iter().filter(|one| one.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()) == Some("armada")).nth(at).expect("a Job in the project");
    let job = record.id.as_str().to_string();
    fleet.update(cx, |fleet, _| fleet.hold_job(record));
    let named = job.clone();
    let fleet = fleet.clone();
    let tab = cx.add_window(move |window, cx| StoryTab::live(&fleet, named, false, window, cx)).root(cx).expect("the tab");
    (tab, job)
}

fn filed_under(tab: &gpui::Entity<StoryTab>, cx: &mut TestAppContext) -> Vec<String> {
    tab.read_with(cx, |tab, _| tab.built.entries.iter().flat_map(|entry| &entry.card.filed).map(|filed| format!("Filed under {}", filed.title)).collect())
}

#[gpui::test]
fn the_story_draws_before_the_librarys_names_and_takes_them_when_they_land_for_every_tab_of_the_project(cx: &mut TestAppContext) {
    ready(cx);
    let fleet = cx.new(crate::fleet::FleetConnection::idle);
    let (first, job) = live_in_project(&fleet, 0, cx);
    first.update(cx, |tab, cx| tab.took(Ok(mock::story(&job)), cx));
    let slug = mock::story(&job).asks.iter().flat_map(|one| one.ask.features.clone()).next().expect("an ask filed under a feature");
    let title = mock::features().get(&slug).cloned().expect("the mock library holds it");
    assert!(filed_under(&first, cx).contains(&format!("Filed under {slug}")), "the story draws with the feature named by its slug");
    let reads = first.read_with(cx, |tab, _| tab.reads);
    cx.update(|cx| super::library::landed_in_tests("armada", 1, crate::features::mock::library(), cx));
    cx.run_until_parked();
    assert!(filed_under(&first, cx).contains(&format!("Filed under {title}")), "the title lands on the drawn story");
    assert_eq!(first.read_with(cx, |tab, _| tab.areas.clone()), mock::areas());
    assert_eq!(first.read_with(cx, |tab, _| tab.reads), reads, "the story is not read again for them");
    // Another Job's tab of the project has them as its story is read.
    let (second, other) = live_in_project(&fleet, 1, cx);
    second.update(cx, |tab, cx| tab.took(Ok(mock::story(&other)), cx));
    assert!(filed_under(&second, cx).contains(&format!("Filed under {title}")));
}

