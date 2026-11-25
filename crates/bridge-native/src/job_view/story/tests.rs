use chrono::{DateTime, TimeZone, Utc};
use gpui::{div, prelude::*, Role, TestAppContext, VisualTestContext, Window};
use ipc::{ArtifactState, SessionId};

use super::mock::{self, JOB};
use super::model::{self, decided, Built};
use super::{Region, StoryTab};
use crate::features::overlay::{Content, FeatureOverlay, OverlayEvent, Source};
use crate::features::page::FeaturePageEvent;
use crate::features::page_model::{day, Hue};
use crate::features::story_link::{self, StoryRequest};

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

fn built() -> Built {
    model::build(&mock::story(JOB), &mock::titles(), &mock::features())
}

fn row_texts(card: &model::Card) -> Vec<String> {
    card.rows.iter().map(|row| row.text.clone()).collect()
}

// ---- the model -------------------------------------------------------------------------------

#[test]
fn asks_are_numbered_in_the_order_fleet_gave_them() {
    let built = built();
    let said: Vec<(&str, &str)> = built.cards.iter().map(|card| (card.number.as_str(), card.title.as_str())).collect();
    assert_eq!(said, [("1.", "Feature catalog"), ("2.", "A reading page never shows"), ("3.", "Votes show as never cast")]);
    let mut flipped = mock::story(JOB);
    flipped.asks.reverse();
    let numbers: Vec<String> = model::build(&flipped, &mock::titles(), &mock::features()).cards.iter().map(|card| format!("{} {}", card.number, card.title)).collect();
    assert_eq!(numbers, ["1. A reading page never shows", "2. Feature catalog", "3. Votes show as never cast"], "grouped by feature, and Fleet's order kept inside a group");
}

#[test]
fn a_folded_ask_is_one_head_line_with_its_kind_what_came_of_it_and_its_state() {
    let built = built();
    let heads: Vec<(&str, String, &str, Hue)> = built.cards.iter().map(|card| (card.kind, card.outcome.clone(), card.stand.word, card.stand.hue)).collect();
    assert_eq!(heads[0], ("Change request", "Open: decided kinds as sections".to_string(), "Open", Hue::Running));
    assert_eq!(heads[1], ("Bug", "Handed to One artifact that grows".to_string(), "Handed", Hue::Handed));
    assert_eq!(heads[2], ("Bug", "Merged #3720".to_string(), "Fixed", Hue::Done));
}

#[test]
fn each_state_has_its_word_and_hue_and_an_ask_with_nothing_yet_says_where_it_stands() {
    use ipc::AskState::*;
    let said: Vec<(&str, Hue)> = [Building, Handed, Built, Backlog, Dropped].into_iter().map(|state| (model::stand(state).word, model::stand(state).hue)).collect();
    assert_eq!(said, [("Open", Hue::Running), ("Handed", Hue::Handed), ("Fixed", Hue::Done), ("Backlog", Hue::NotStarted), ("Dropped", Hue::Muted)]);
    let mut story = mock::story(JOB);
    let bare = &mut story.asks[0];
    bare.ask.said.clear();
    bare.shown.clear();
    let card = &model::build(&story, &mock::titles(), &mock::features()).cards[0];
    assert_eq!(card.outcome, "Open", "no outcome is made up: only where it stands");
}

#[test]
fn the_unfolded_ask_lists_how_it_started_what_was_said_shown_handed_and_built_in_the_order_it_happened_then_where_it_is_filed() {
    let built = built();
    assert_eq!(
        row_texts(&built.cards[2]),
        ["\"they show as i never voted\"", "\"yes, per revision\"", "Votes per revision", "Handed to One artifact that grows", "Merged #3720", "Filed under Thread and composer"]
    );
    let rows = &built.cards[2].rows;
    assert_eq!(rows[0].meta, format!("You, {}, how it started", day(at(15, 12))));
    assert_eq!(rows[1].meta, format!("You, {}, answer", day(at(13, 12))));
    assert_eq!(rows[4].meta, day(at(15, 15)));
    assert!(rows[3].tinted && rows[3].hue == Hue::Handed && rows[4].tinted && rows[4].hue == Hue::Done, "handed and merged are drawn in their hues");
    assert_eq!(rows[5].feature.as_deref(), Some("thread-and-composer"));
    assert_eq!(
        row_texts(&built.cards[0]),
        ["\"every feature gets a page the agents read before they touch its files\"", "Feature catalog design", "\"decided kinds as sections\"", "Open", "Filed under Start a Job", "Filed under Reviews"]
    );
    assert_eq!(built.cards[0].rows[1].meta.split(". ").last(), Some("Approved"), "shown, then what he decided");
    assert_eq!(built.cards[0].rows[3].hue, Hue::Running, "an open ask ends in Open in the running hue");
    assert!(!row_texts(&built.cards[2]).contains(&"Open".to_string()), "a fixed ask is not open");
    assert_eq!(built.cards[1].rows.iter().filter(|row| row.feature.is_some()).count(), 1);
}

#[test]
fn what_he_was_shown_says_what_he_decided_in_armadas_voice_and_his_votes_and_presses_carry_no_figure() {
    let mut story = mock::story(JOB);
    let shown = &mut story.asks[2].shown[0];
    assert_eq!(decided(shown), "Sent back: keep each take's votes");
    for (state, words, said) in [
        (ArtifactState::Approved, None, "Approved"),
        (ArtifactState::Approved, Some("go"), "Approved: go"),
        (ArtifactState::SentBack, None, "Sent back"),
        (ArtifactState::Dismissed, Some("ignored"), "Dismissed"),
        (ArtifactState::Asking, None, "Waiting"),
    ] {
        shown.state = state;
        shown.words = words.map(str::to_string);
        assert_eq!(decided(shown), said);
    }
    let rows = &model::build(&story, &mock::titles(), &mock::features()).cards[2].rows;
    assert_eq!(rows[2].also, "Pressed start on take-a", "a press is a muted line");
    let first = &model::build(&mock::story(JOB), &mock::titles(), &mock::features()).cards[0].rows[1];
    assert_eq!(first.also, "Picked rows", "a vote is a muted line");
    assert!(!first.also.chars().any(|c| c.is_ascii_digit()), "no count beside what it counts");
}

#[test]
fn a_hand_is_to_a_job_or_from_one_and_a_job_with_no_title_is_never_named_by_its_id() {
    let hand = |built: &Built| built.cards[1].rows.iter().find(|row| row.glyph == "arrow-right-left").map(|row| row.text.clone());
    let mut story = mock::story(JOB);
    story.asks[1].handed[0].to_title = None;
    assert_eq!(hand(&model::build(&story, &Default::default(), &mock::features())).as_deref(), Some("Handed"));
    assert_eq!(hand(&model::build(&story, &mock::titles(), &mock::features())).as_deref(), Some("Handed to One artifact that grows"), "the Jobs list names it");
    story.asks[1].handed[0].from = SessionId::carried("job-one-artifact");
    story.asks[1].handed[0].to = SessionId::carried(JOB);
    assert_eq!(hand(&model::build(&story, &mock::titles(), &mock::features())).as_deref(), Some("Taken from One artifact that grows"));
}

#[test]
fn what_he_was_shown_before_any_ask_stands_apart_unnumbered_and_a_feature_without_a_title_reads_by_its_slug() {
    let built = built();
    assert_eq!(built.before.len(), 1);
    assert_eq!(built.before[0].text, "Start card with the model picker");
    assert!(built.before[0].meta.ends_with("Dismissed"));
    let mut story = mock::story(JOB);
    story.shown_otherwise.clear();
    assert!(model::build(&story, &mock::titles(), &mock::features()).before.is_empty(), "nothing before any ask: no heading is drawn");
    let bare = model::build(&mock::story(JOB), &mock::titles(), &Default::default());
    assert_eq!(bare.cards[2].rows.last().map(|row| row.text.as_str()), Some("Filed under thread-and-composer"));
}

#[test]
fn the_done_cards_lead_is_a_line_an_ask_each_with_its_asks_state() {
    let lead = model::lead(&mock::story(JOB));
    let said: Vec<(&str, &str)> = lead.iter().map(|(stand, line)| (stand.word, line.as_str())).collect();
    assert_eq!(said, [("Open", "Feature catalog: being built"), ("Handed", "A reading page never shows: handed to One artifact that grows"), ("Fixed", "Votes show as never cast: fixed in #3720")]);
    let mut story = mock::story(JOB);
    story.lead.clear();
    assert!(model::lead(&story).is_empty());
}

#[test]
fn the_features_the_job_is_on_keep_fleets_order_and_say_only_the_ways_it_touched_each() {
    let lines: Vec<(String, String)> = built().features.iter().map(|line| (line.title.clone(), line.how.clone())).collect();
    assert_eq!(
        lines,
        [
            ("Start a Job".to_string(), "wrote 12 files · named in 9 commits · 2 asks filed".to_string()),
            ("Thread and composer".to_string(), "named in 3 commits · 1 ask filed".to_string()),
            ("Reviews".to_string(), "1 ask filed".to_string()),
        ]
    );
    let one = ipc::JobFeature { slug: "a".into(), title: "A".into(), area: "x".into(), files_written: 1, commits_named: 1, asks: 0 };
    assert_eq!(model::how_touched(&one), "wrote 1 file · named in 1 commit");
}

#[test]
fn asks_group_under_the_most_touched_feature_they_touch_with_open_first_and_the_unfiled_last_with_no_heading() {
    let mut story = mock::story(JOB);
    // A closed ask under the first feature, an ask on two features (Reviews is touched least) and one under none.
    story.asks[0].open = false;
    let mut loose = story.asks[2].clone();
    loose.ask.ask_id = "ask-4".into();
    loose.ask.title = "Loose".into();
    loose.ask.features.clear();
    let mut both = story.asks[0].clone();
    both.ask.ask_id = "ask-5".into();
    both.ask.title = "On two".into();
    both.ask.features = vec!["reviews".into(), "thread-and-composer".into()];
    both.open = true;
    story.asks.extend([loose, both]);
    let built = model::build(&story, &mock::titles(), &mock::features());
    let order: Vec<(&str, Option<&str>)> = built.cards.iter().map(|card| (card.ask_id.as_str(), card.heading.as_ref().map(|heading| heading.title.as_str()))).collect();
    assert_eq!(
        order,
        [("ask-2", Some("Start a Job")), ("ask-1", None), ("ask-5", Some("Thread and composer")), ("ask-3", None), ("ask-4", None)],
        "open first in a group; a heading once per group; an ask on two sits in the more touched one; unfiled last, no heading"
    );
    assert_eq!(built.cards.iter().map(|card| card.number.as_str()).collect::<Vec<_>>(), ["1.", "2.", "3.", "4.", "5."]);
}

#[test]
fn the_outcome_is_what_was_merged_else_where_it_was_handed_else_what_he_decided_and_never_a_made_up_sentence() {
    let titles = mock::titles();
    let mine = SessionId::carried(JOB);
    let mut story = mock::story(JOB);
    let one = |story: &ipc::StoryAsk| model::outcome(story, &mine, &titles);
    assert_eq!(one(&story.asks[2]), "Merged #3720");
    story.asks[2].built.push(ipc::StoryBuilt { pull_request: "#3731".into(), merged_at: ipc::Instant::carried("2026-11-16T12:00:00Z".to_string()) });
    assert_eq!(one(&story.asks[2]), "Merged #3720, #3731");
    story.asks[2].open = true;
    assert_eq!(one(&story.asks[2]), "Merged #3720, #3731, still open");
    assert_eq!(one(&story.asks[1]), "Handed to One artifact that grows");
    let bare = &mut story.asks[0];
    assert_eq!(one(bare), "Open: decided kinds as sections", "the latest thing said or decided, while open");
    bare.ask.said = vec![mock::words("row-z", ipc::MessageKind::Answer, "yes, per revision", 20)];
    assert_eq!(one(bare), "Open: Answered: yes, per revision");
    bare.open = false;
    bare.ask.said.clear();
    bare.shown.clear();
    bare.ask.state = ipc::AskState::Dropped;
    assert_eq!(one(bare), "Dropped");
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

#[gpui::test]
fn every_ask_starts_folded_and_one_at_a_time_enter_folds_and_unfolds_where_j_and_k_are(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    assert_eq!(unfolded(&tab, cx), None, "every ask is one line until he presses it");
    assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(0), "the keys start on the first open one");
    cx.simulate_keystrokes("j");
    assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(1));
    assert_eq!(unfolded(&tab, cx), None, "moving unfolds nothing");
    cx.simulate_keystrokes("enter");
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("ask-2"));
    cx.simulate_keystrokes("enter");
    assert_eq!(unfolded(&tab, cx), None, "Enter on the open ask folds it");
    cx.simulate_keystrokes("down down down");
    assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(2), "it stops at the last");
    cx.simulate_keystrokes("enter k");
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("ask-3"));
    assert_eq!(tab.read_with(cx, |tab, _| tab.cursor), Some(1));
    tab.update(cx, |tab, cx| tab.rebuild(cx));
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("ask-3"), "reading the story again keeps what he unfolded");
    cx.simulate_keystrokes("k enter");
    assert_eq!(unfolded(&tab, cx).as_deref(), Some("ask-1"), "Enter unfolds the first");
}

#[gpui::test]
fn a_folded_ask_stays_folded_when_the_story_is_read_again(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_keystrokes("enter enter");
    assert_eq!(unfolded(&tab, cx), None);
    tab.update(cx, |tab, cx| tab.rebuild(cx));
    assert_eq!(unfolded(&tab, cx), None, "a read does not unfold what he folded");
}

#[gpui::test]
fn tab_goes_to_the_filed_under_rows_and_enter_there_opens_the_feature_where_esc_goes_back_to_the_asks(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_keystrokes("tab");
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Asks, "a folded ask has no feature to go to");
    cx.simulate_keystrokes("enter tab");
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Links);
    cx.simulate_keystrokes("j");
    assert_eq!(tab.read_with(cx, |tab, _| tab.link_at), 1);
    cx.simulate_keystrokes("j");
    assert_eq!(tab.read_with(cx, |tab, _| tab.link_at), 1, "two features, so it stops at the second");
    cx.simulate_keystrokes("escape");
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Asks);
    assert!(tab.read_with(cx, |tab, _| tab.overlay().is_none()));
    cx.simulate_keystrokes("tab enter");
    let overlay = tab.read_with(cx, |tab, _| tab.overlay().cloned()).expect("the feature opened over the tab");
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.depth()), 1);
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.titles().map(|(title, from)| (title.to_string(), from.to_string()))), Some(("Start a Job".to_string(), "Story of Feature Catalog".to_string())));
    cx.simulate_keystrokes("escape");
    assert!(tab.read_with(cx, |tab, _| tab.overlay().is_none()), "the last Esc closes the overlay");
    // The keys are back on the tab, still on the Filed under row; Esc goes to the heads, and a folded ask has no feature to go to.
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Links);
    cx.simulate_keystrokes("escape enter tab");
    assert_eq!(tab.read_with(cx, |tab, _| tab.region), Region::Asks, "a part with nothing in it is passed over");
}

#[gpui::test]
fn a_story_link_puts_its_ask_in_front_when_the_tab_first_draws_and_one_for_another_job_is_left_alone(cx: &mut TestAppContext) {
    ready(cx);
    cx.update(|cx| story_link::put(StoryRequest { session_id: SessionId::carried("another-job"), ask_id: "ask-3".into() }, cx));
    let (tab, cx) = cx.add_window_view(|window, cx| StoryTab::mock(JOB.into(), false, window, cx));
    assert_eq!(unfolded(&tab, cx), None, "another Job's request is not this tab's");
    assert!(cx.update(|_, cx| story_link::peek(cx)).is_some_and(|request| request.session_id.as_str() == "another-job"), "and stays for that Job's tab");
    cx.update(|_, cx| story_link::take(cx));
    cx.update(|_, cx| story_link::put(StoryRequest { session_id: SessionId::carried(JOB), ask_id: "ask-3".into() }, cx));
    let (named, cx) = cx.add_window_view(|window, cx| StoryTab::mock(JOB.into(), false, window, cx));
    assert_eq!(unfolded(&named, cx).as_deref(), Some("ask-3"));
    assert_eq!(named.read_with(cx, |tab, _| tab.cursor), Some(2));
    assert!(cx.update(|_, cx| story_link::peek(cx)).is_none(), "it was taken");
}

#[gpui::test]
fn a_long_title_gives_way_and_the_state_keeps_its_place_at_the_heads_end(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_resize(gpui::size(gpui::px(360.), gpui::px(800.)));
    for _ in 0..2 {
        tab.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    // The book of a window with no label root keeps every frame's entries, so the last is the frame just drawn.
    let bounds = |cx: &mut VisualTestContext, role: Role, name: &str| {
        cx.update(|window, cx| {
            cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds)
        })
    };
    let state = bounds(cx, Role::Image, "Fixed").expect("the state is drawn");
    assert!(state.right() <= gpui::px(360.), "the state stays in view at 360 wide: {state:?}");
}

#[gpui::test]
fn a_heads_last_line_sits_under_its_title_and_the_state_stays_inside_the_card(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_resize(gpui::size(gpui::px(700.), gpui::px(800.)));
    let title = "Schedule work or set up watcher scripts outside jobs, so that a person can leave a repository to be looked at on a timetable and not by hand";
    tab.update(cx, |tab, cx| {
        if let super::Load::Read(story) = &mut tab.story {
            story.asks[0].ask.title = title.into();
            story.asks[0].ask.said = vec![mock::words("row-long", ipc::MessageKind::Revision, "**Read the whole of the watcher design and decided that a timetable belongs to the repository, not to the Job that first asked for one**", 17)];
        }
        tab.rebuild(cx);
    });
    for _ in 0..2 {
        tab.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    let last = tab.read_with(cx, |tab, _| tab.built.cards[0].outcome.clone());
    assert!(!last.contains('*'), "the Last line is plain words: {last}");
    let bounds = |cx: &mut VisualTestContext, role: Role, name: &str| {
        cx.update(|window, cx| {
            cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds)
        })
    };
    let said = format!("1. {title}");
    let head = bounds(cx, Role::Label, &said).expect("the title is drawn");
    let under = bounds(cx, Role::Label, &last).expect("the Last line is drawn");
    let state = bounds(cx, Role::Image, "Open").expect("the state is drawn");
    let card = bounds(cx, Role::ListBoxOption, &said).expect("the card is drawn");
    assert!(under.top() >= head.bottom(), "the Last line is a row under the title, not beside it: title {head:?}, last {under:?}");
    assert!(head.size.height > gpui::px(24.), "a long title wraps to a second line and is not cut: {head:?}");
    assert!(state.right() <= card.right(), "the state stays inside the card: {state:?} in {card:?}");
    assert!(under.right() <= card.right(), "the Last line is cut at the card's edge: {under:?} in {card:?}");
}

#[gpui::test]
fn the_tab_draws_a_role_and_a_name_for_every_head_and_row(cx: &mut TestAppContext) {
    let (_tab, cx) = open(cx);
    cx.simulate_keystrokes("enter");
    assert!(drawn(cx, Role::Heading, "Start a Job") && drawn(cx, Role::Heading, "Thread and composer"), "a group is a heading named by its feature");
    assert!(drawn(cx, Role::Button, "1. Feature catalog"), "a head is a button named as it reads");
    assert!(drawn(cx, Role::Button, "3. Votes show as never cast"));
    assert!(drawn(cx, Role::ListBox, "Asks"));
    assert!(drawn(cx, Role::Link, "Filed under Reviews"), "a Filed under row is a link");
    assert!(drawn(cx, Role::Image, "Open"), "the state glyph is named");
}

#[gpui::test]
fn the_group_of_what_he_was_shown_before_any_ask_has_a_place_only_where_there_is_one(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 5, "the features, three asks and the group");
    tab.update(cx, |tab, cx| {
        if let super::Load::Read(story) = &mut tab.story {
            story.shown_otherwise.clear();
        }
        tab.rebuild(cx);
    });
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 4, "no group where nothing was shown before any ask");
}

#[gpui::test]
fn the_tab_draws_bones_while_it_reads_and_nothing_for_a_job_with_no_asks(cx: &mut TestAppContext) {
    ready(cx);
    let (tab, cx) = cx.add_window_view(|window, cx| StoryTab::mock(JOB.into(), false, window, cx));
    tab.update(cx, |tab, cx| {
        tab.story = super::Load::Reading;
        tab.rebuild(cx);
    });
    cx.run_until_parked();
    assert!(drawn(cx, Role::Image, "Loading"));
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 0, "no card is built while it reads");
    tab.update(cx, |tab, cx| {
        tab.story = super::Load::Read(ipc::JobStory { session_id: SessionId::carried(JOB), asks: Vec::new(), shown_otherwise: Vec::new(), lead: Vec::new(), features: Vec::new() });
        tab.rebuild(cx);
    });
    cx.run_until_parked();
    assert_eq!(tab.read_with(cx, |tab, _| tab.places()), 0, "an empty slot stays empty");
    assert!(matches!(tab.read_with(cx, |tab, _| tab.story_is_read()), true), "and it is not bones");
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
    assert_eq!(unfolded(&story, cx).as_deref(), Some("ask-3"), "at the ask the link named");
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
    assert!(drawn(cx, Role::Button, "3. Votes show as never cast"), "the tab is drawn in the pane");
}

// ---- the features the Job is on ------------------------------------------------------------

#[gpui::test]
fn the_features_the_job_is_on_head_the_tab_a_row_each_in_fleets_order_above_the_asks(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    for _ in 0..2 {
        tab.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    let bounds = |cx: &mut VisualTestContext, role: Role, name: &str| {
        cx.update(|window, cx| {
            cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds)
        })
    };
    let heading = bounds(cx, Role::Heading, "Features this Job is on").expect("the section is drawn");
    let rows: Vec<gpui::Bounds<gpui::Pixels>> = ["Start a Job, wrote 12 files · named in 9 commits · 2 asks filed", "Thread and composer, named in 3 commits · 1 ask filed", "Reviews, 1 ask filed"]
        .into_iter()
        .map(|name| bounds(cx, Role::Link, name).unwrap_or_else(|| panic!("{name} is drawn")))
        .collect();
    let first_ask = bounds(cx, Role::Button, "1. Feature catalog").expect("the first ask is drawn");
    assert!(heading.bottom() <= rows[0].top() && rows[0].bottom() <= rows[1].top() && rows[1].bottom() <= rows[2].top(), "heading, then each row in order: {heading:?} {rows:?}");
    assert!(rows[2].bottom() <= first_ask.top(), "the section is above the asks: {rows:?} {first_ask:?}");
}

#[gpui::test]
fn a_job_on_no_feature_draws_no_section(cx: &mut TestAppContext) {
    ready(cx);
    // Read with no features from the first frame, so the label book holds nothing an earlier frame drew.
    let (tab, cx) = cx.add_window_view(|window, cx| {
        let mut tab = StoryTab::mock(JOB.into(), false, window, cx);
        if let super::Load::Read(story) = &mut tab.story {
            story.features.clear();
        }
        tab.rebuild(cx);
        tab
    });
    cx.update(|window, cx| {
        let focus = tab.read(cx).focus.clone();
        window.focus(&focus, cx)
    });
    cx.run_until_parked();
    assert_eq!(tab.read_with(cx, |tab, _| (tab.head(), tab.places())), (0, 4), "three asks and the group, and no place for the section");
    assert!(!drawn(cx, Role::Heading, "Features this Job is on"), "an empty slot stays empty");
    cx.simulate_keystrokes("k");
    assert_eq!(tab.read_with(cx, |tab, _| (tab.region, tab.cursor)), (Region::Asks, Some(0)), "Up at the first ask stays there");
}

#[gpui::test]
fn up_from_the_first_ask_reaches_the_features_and_enter_on_one_opens_its_page_over_the_tab(cx: &mut TestAppContext) {
    let (tab, cx) = open(cx);
    cx.simulate_keystrokes("k");
    assert_eq!(tab.read_with(cx, |tab, _| (tab.region, tab.feature_at)), (Region::Features, 2), "Up from the first ask is the last feature");
    cx.simulate_keystrokes("k k k");
    assert_eq!(tab.read_with(cx, |tab, _| tab.feature_at), 0, "it stops at the first");
    cx.simulate_keystrokes("j j j");
    assert_eq!(tab.read_with(cx, |tab, _| (tab.region, tab.cursor)), (Region::Asks, Some(0)), "Down from the last feature is the first ask");
    cx.simulate_keystrokes("k k k enter");
    let overlay = tab.read_with(cx, |tab, _| tab.overlay().cloned()).expect("the feature opened over the tab");
    assert_eq!(overlay.read_with(cx, |overlay, _| overlay.titles().map(|(title, from)| (title.to_string(), from.to_string()))), Some(("Start a Job".to_string(), "Story of Feature Catalog".to_string())));
    cx.simulate_keystrokes("escape");
    assert!(tab.read_with(cx, |tab, _| tab.overlay().is_none()));
    assert_eq!(tab.read_with(cx, |tab, _| (tab.region, tab.feature_at)), (Region::Features, 0), "the keys come back to the row");
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
    tab.read_with(cx, |tab, _| tab.built.cards.iter().flat_map(|card| &card.rows).filter(|row| row.feature.is_some()).map(|row| row.text.clone()).collect())
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
