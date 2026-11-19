use super::model::*;
use super::{mock, UsageScreen};

fn week() -> Board {
    Board::new(mock::waste(Span::Week))
}

#[test]
fn kinds_are_listed_largest_first_and_each_kinds_jobs_too() {
    let board = week();
    assert_eq!(
        board.kinds(),
        ["subagent_context", "main_context", "cold_cache", "repeat_reads", "big_results", "fresh_starts"],
        "58M, 31M, 18M, 12M, 8M and 4M"
    );
    let titles = mock::titles();
    let lines = board.lines_of("subagent_context", &titles);
    assert_eq!(lines.iter().map(|line| (line.title.as_str(), line.units.as_str(), line.peak.as_str())).take(3).collect::<Vec<_>>(), [("Native jobs screens", "21M", "870k"), ("Docs audit", "9M", "610k"), ("Fleet speed work", "6M", "420k")]);
    let units: Vec<u64> = board.kind("subagent_context").expect("the kind").jobs.iter().map(|job| job.units).collect();
    assert!(units.windows(2).all(|pair| pair[0] >= pair[1]), "largest first");
}

#[test]
fn a_kind_that_arrives_out_of_order_is_sorted_with_ties_broken_by_title() {
    let mut waste = mock::waste(Span::Week);
    waste.kinds.reverse();
    let fresh = waste.kinds.iter().position(|kind| kind.kind == "fresh_starts").expect("a kind");
    waste.kinds[fresh].units = 31_000_000;
    waste.kinds[fresh].title = "Aardvark".into();
    let board = Board::new(waste);
    assert_eq!(board.kinds()[..3], ["subagent_context", "fresh_starts", "main_context"], "equal units: the title that sorts first leads");
}

#[test]
fn the_header_says_the_waste_and_the_whole_in_millions() {
    assert_eq!(week().header().as_deref(), Some("131M of 412M went to fixable waste"));
}

#[test]
fn a_window_with_no_burn_has_no_header_and_no_rows() {
    let board = Board::new(mock::empty(Span::Week));
    assert_eq!(board.header(), None);
    assert!(board.rows().is_empty());
    assert!(board.kinds().is_empty());
}

#[test]
fn units_read_as_millions_thousands_and_plain_figures() {
    for (units, said) in [(131_000_000, "131M"), (4_000_000, "4M"), (4_200_000, "4.2M"), (870_000, "870k"), (999_800, "1M"), (12_000, "12k"), (640, "640"), (0, "0")] {
        assert_eq!(units_said(units), said, "{units}");
    }
}

#[test]
fn a_figure_names_what_it_counted_in_its_tooltip_and_never_beside_it() {
    let rows = week().rows();
    assert_eq!(rows[0].tip, "41 requests");
    assert_eq!(rows[2].tip, "41 wakes");
    assert_eq!(times_said(1, look_of("cold_cache")), "1 wake");
    assert!(rows.iter().all(|row| !row.units.contains("request") && !row.units.contains("wake")), "the row's figure is units alone");
}

#[test]
fn a_job_with_no_title_shows_its_short_id_and_a_missing_peak_is_empty() {
    let board = week();
    let lines = board.lines_of("fresh_starts", &mock::titles());
    let untitled = lines.iter().find(|line| line.id == "5d1e9c42b7a8").expect("the Job with no title");
    assert_eq!(untitled.title, "5d1e9c42");
    assert_eq!(untitled.peak, "", "Fleet recorded none");
    let titled = board.lines_of("subagent_context", &mock::titles());
    assert_eq!((titled[0].units.as_str(), titled[0].peak.as_str()), ("21M", "870k"));
}

#[test]
fn a_kind_fleet_adds_is_still_listed() {
    let look = look_of("something_new");
    assert_eq!(look.glyph, "circle-dot");
}

#[test]
fn the_cursor_steps_down_and_up_and_stops_at_each_end() {
    let kinds = ["a", "b", "c"];
    let mut selection = Selection::default();
    selection.step(&kinds, 1);
    assert_eq!(selection.cursor.as_deref(), Some("a"), "the first move from nowhere lands on the first row");
    selection.step(&kinds, 1);
    selection.step(&kinds, 1);
    selection.step(&kinds, 1);
    assert_eq!(selection.cursor.as_deref(), Some("c"), "held at the last");
    selection.step(&kinds, -1);
    assert_eq!(selection.cursor.as_deref(), Some("b"));
    let mut up = Selection::default();
    up.step(&kinds, -1);
    assert_eq!(up.cursor.as_deref(), Some("c"), "Up from nowhere lands on the last");
    Selection::default().step(&[], 1);
}

#[test]
fn enter_opens_the_row_under_the_keys_and_again_shuts_it_and_escape_shuts_it() {
    let mut selection = Selection::default();
    selection.toggle("b");
    assert_eq!((selection.cursor.as_deref(), selection.opened.as_deref()), (Some("b"), Some("b")));
    selection.toggle("c");
    assert_eq!(selection.opened.as_deref(), Some("c"), "another row replaces it");
    selection.toggle("c");
    assert_eq!(selection.opened, None, "the same row shuts it");
    selection.toggle("a");
    assert!(selection.close());
    assert!(!selection.close(), "Esc with nothing open takes nothing");
    assert_eq!(selection.cursor.as_deref(), Some("a"), "the cursor stays where it was");
}

#[test]
fn a_new_reading_keeps_the_selection_where_its_kind_is_still_there() {
    let mut selection = Selection { cursor: Some("b".into()), opened: Some("b".into()) };
    selection.keep(&["a", "b"]);
    assert_eq!((selection.cursor.as_deref(), selection.opened.as_deref()), (Some("b"), Some("b")));
    selection.keep(&["a", "c"]);
    assert_eq!((selection.cursor.as_deref(), selection.opened.as_deref()), (Some("a"), None), "the cursor falls to the first row and the panel shuts");
    selection.keep(&[]);
    assert_eq!(selection.cursor, None);
}

#[test]
fn the_window_keys_are_day_week_and_fortnight_in_that_order() {
    assert_eq!([0, 1, 2].map(|slot| Span::at(slot).map(Span::label)), [Some("Day"), Some("Week"), Some("Fortnight")]);
    assert_eq!(Span::at(3), None);
}

#[test]
fn every_window_has_the_same_kinds_in_the_same_order() {
    let order = |span| Board::new(mock::waste(span)).kinds().into_iter().map(str::to_string).collect::<Vec<_>>();
    assert_eq!(order(Span::Day), order(Span::Week));
    assert_eq!(order(Span::Fortnight), order(Span::Week));
}

#[test]
fn the_scenarios_that_open_on_the_page_are_the_three() {
    assert!(["usage-week", "usage-loading", "usage-empty"].iter().all(|name| super::is_scenario(name)));
    assert!(!super::is_scenario("jobs-many"));
    let _ = UsageScreen::mock;
}

mod screen {
    use gpui::{AppContext as _, Entity, Role, TestAppContext, VisualTestContext};

    use crate::jobs::{JobsList, JobsScreen};
    use crate::labels::LabelBook;
    use crate::operator::screen::Front;
    use crate::operator::OperatorScreen;
    use crate::usage_page::UsageScreen;

    fn ready(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
            crate::titlebar::bind(cx);
        });
    }

    fn redraw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
        cx.update(|window, cx| cx.try_global::<LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).is_some())
    }

    fn jobs_screen(scenario: &str, window: &mut gpui::Window, cx: &mut gpui::App) -> Entity<JobsScreen> {
        let records = crate::mock::jobs(scenario).expect("the fixture decodes");
        let list = cx.new(|cx| JobsList::mock(&records, cx));
        cx.new(|cx| JobsScreen::new(list, None, window, cx))
    }

    /// The operator's screen with Usage in front, on a scenario.
    fn page<'a>(cx: &'a mut TestAppContext, scenario: &'static str) -> (Entity<OperatorScreen>, &'a mut VisualTestContext) {
        ready(cx);
        cx.add_window_view(move |window, cx| {
            let jobs = jobs_screen(scenario, window, cx);
            OperatorScreen::mock_usage(jobs, window, cx)
        })
    }

    /// The kind whose panel is open. The label book keeps every frame a test draws, so a panel shut is read from the page and not from the book.
    fn opened(screen: &Entity<OperatorScreen>, cx: &mut VisualTestContext) -> Option<String> {
        let page: Entity<UsageScreen> = screen.read_with(cx, |screen, _| screen.usage_for_test().expect("the screen holds the page"));
        page.read_with(cx, |page, _| page.selection.opened.clone())
    }

    #[gpui::test]
    fn the_page_opens_in_front_with_its_header_and_a_row_a_kind(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-week");
        redraw(cx);
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Usage);
        assert!(drawn(cx, Role::Heading, "131M of 412M went to fixable waste"));
        assert!(drawn(cx, Role::Row, "Subagents ran past 300k context"));
        assert!(drawn(cx, Role::Row, "Fresh starts and compactions"));
        assert_eq!(opened(&screen, cx), None, "nothing is open until a row is");
    }

    #[gpui::test]
    fn enter_opens_a_kinds_jobs_and_escape_shuts_them(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-week");
        redraw(cx);
        cx.simulate_keystrokes("enter");
        redraw(cx);
        assert!(drawn(cx, Role::Table, "Jobs hit by Subagents ran past 300k context"), "the keys start on the first row");
        cx.simulate_keystrokes("j space");
        redraw(cx);
        assert!(drawn(cx, Role::Table, "Jobs hit by Main thread ran past 300k context"), "j and Space move on and open the next");
        assert_eq!(opened(&screen, cx).as_deref(), Some("main_context"), "the next kind took the panel");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert_eq!(opened(&screen, cx), None, "Esc shuts the panel");
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Usage, "and stays on the page");
        cx.simulate_keystrokes("escape");
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Usage, "Esc with nothing open leaves it where it is");
    }

    #[gpui::test]
    fn d_w_and_f_read_the_day_the_week_and_the_fortnight_and_keep_the_open_kind(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-week");
        redraw(cx);
        cx.simulate_keystrokes("down enter d");
        redraw(cx);
        assert!(drawn(cx, Role::Heading, "26M of 82M went to fixable waste"), "the day's figures");
        assert!(drawn(cx, Role::Table, "Jobs hit by Main thread ran past 300k context"), "the panel stays on its kind");
        cx.simulate_keystrokes("f");
        redraw(cx);
        assert!(drawn(cx, Role::Heading, "249M of 783M went to fixable waste"), "the fortnight's figures");
        assert_eq!(opened(&screen, cx).as_deref(), Some("main_context"));
    }

    #[gpui::test]
    fn the_loading_page_draws_shapes_and_the_empty_window_draws_only_its_switch(cx: &mut TestAppContext) {
        let (_, cx) = page(cx, "usage-loading");
        redraw(cx);
        assert!(drawn(cx, Role::Image, "Loading"));
        assert!(!drawn(cx, Role::Row, "Subagents ran past 300k context"));
        let (_, cx) = page(cx, "usage-empty");
        redraw(cx);
        assert!(drawn(cx, Role::RadioGroup, "Window"));
        assert!(!drawn(cx, Role::Heading, "0 of 0 went to fixable waste"));
        assert!(!drawn(cx, Role::Row, "Cold cache after a pause"));
        assert!(!drawn(cx, Role::Image, "Loading"));
    }

    #[gpui::test]
    fn the_palette_finds_usage_by_name_and_by_its_aliases(cx: &mut TestAppContext) {
        ready(cx);
        let (screen, cx) = cx.add_window_view(|window, cx| {
            let jobs = jobs_screen("usage-week", window, cx);
            OperatorScreen::mock_reviews(jobs, window, cx)
        });
        redraw(cx);
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Jobs);
        cx.simulate_keystrokes("cmd-k");
        cx.simulate_input("burn");
        cx.simulate_keystrokes("enter");
        redraw(cx);
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Usage, "`burn` is one of Classic's names for it");
    }
}
