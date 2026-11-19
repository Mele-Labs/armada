use super::model::*;
use super::{mock, UsageScreen};

fn week() -> Board {
    Board::new(mock::waste(Span::Week), Span::Week)
}

#[test]
fn kinds_are_listed_largest_first_and_each_kinds_jobs_too() {
    let board = week();
    assert_eq!(
        board.kinds(),
        ["subagent_context", "main_context", "cold_cache", "repeat_reads", "big_results", "fresh_starts"],
        "58, 31, 18, 12, 8 and 4"
    );
    let titles = mock::titles();
    let lines = board.lines_of("subagent_context", &titles, &[]);
    assert_eq!(lines.iter().map(|line| (line.title.as_str(), line.share.as_str(), line.peak.as_str())).take(3).collect::<Vec<_>>(), [("Native jobs screens", "5.1%", "870k"), ("Docs audit", "2.2%", "610k"), ("Fleet speed work", "1.5%", "420k")], "21, 9 and 6 of 412");
    let units: Vec<f64> = board.kind("subagent_context").expect("the kind").jobs.iter().map(|job| job.units).collect();
    assert!(units.windows(2).all(|pair| pair[0] >= pair[1]), "largest first");
}

#[test]
fn a_kind_that_arrives_out_of_order_is_sorted_with_ties_broken_by_title() {
    let mut waste = mock::waste(Span::Week);
    waste.kinds.reverse();
    let fresh = waste.kinds.iter().position(|kind| kind.kind == "fresh_starts").expect("a kind");
    waste.kinds[fresh].units = 31.;
    waste.kinds[fresh].title = "Aardvark".into();
    let board = Board::new(waste, Span::Week);
    assert_eq!(board.kinds()[..3], ["subagent_context", "fresh_starts", "main_context"], "equal units: the title that sorts first leads");
}

#[test]
fn the_header_says_what_share_of_the_burn_was_waste_a_person_can_fix() {
    assert_eq!(week().header().as_deref(), Some("32% of this week's burn was waste you can fix"), "131 of 412");
    let day = Board::new(mock::waste(Span::Day), Span::Day);
    assert_eq!(day.header().as_deref(), Some("32% of today's burn was waste you can fix"));
    let fortnight = Board::new(mock::waste(Span::Fortnight), Span::Fortnight);
    assert_eq!(fortnight.header().as_deref(), Some("32% of this fortnight's burn was waste you can fix"));
}

#[test]
fn a_window_with_no_burn_has_no_header_and_no_rows() {
    let board = Board::new(mock::empty(Span::Week), Span::Week);
    assert_eq!(board.header(), None);
    assert!(board.rows().is_empty());
    assert!(board.kinds().is_empty());
}

#[test]
fn a_share_is_a_percent_with_a_decimal_under_ten() {
    for (share, said) in [(0.152, "15%"), (0.318, "32%"), (0.0999, "10%"), (0.0996, "10%"), (0.0949, "9.5%"), (0.083, "8.3%"), (0.055, "5.5%"), (0.0102, "1.0%"), (0.0004, "0.0%"), (0., "0%"), (1., "100%")] {
        assert_eq!(share_said(share), said, "{share}");
    }
    assert_eq!((share_exact(0.123), share_exact(0.031), share_exact(0.5)), ("12.3%".to_string(), "3.1%".to_string(), "50.0%".to_string()), "a change keeps its decimal so before and since read against each other");
}

fn east(hours: i32) -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(hours * 3600).expect("an offset")
}

#[test]
fn a_date_is_the_day_and_the_month_in_the_macs_own_zone() {
    assert_eq!(date_in("2026-11-17T14:20:00Z", 0, &east(0)), "17 Nov");
    assert_eq!(date_in("2026-11-17T23:20:00Z", 0, &east(1)), "18 Nov", "late in the evening in UTC is the next day east of it");
    assert_eq!(date_in("2026-11-18T03:00:00Z", 0, &east(-5)), "17 Nov", "and the day before west of it");
    assert_eq!(date_in("2026-01-03T00:00:00Z", 0, &east(0)), "3 Jan");
    assert_eq!(date_in("2026-11-30T22:00:00Z", 1, &east(0)), "1 Dec", "a day later crosses the month");
    assert_eq!(date_in("2026-11-18T03:00:00Z", 1, &east(-5)), "18 Nov", "a day later is counted in the Mac's zone");
    assert_eq!(date_in("yesterday", 0, &east(0)), "yesterday", "a stamp that cannot be read is said as it came");
}

#[test]
fn a_context_size_keeps_its_token_form() {
    for (tokens, said) in [(870_000, "870k"), (1_200_000, "1.2M"), (999_800, "1M"), (640, "640")] {
        assert_eq!(tokens_said(tokens), said, "{tokens}");
    }
}

#[test]
fn the_header_and_every_figure_share_the_windows_whole_burn_on_what_fleet_really_served() {
    let mut waste = mock::waste(Span::Week);
    waste.total_units = 9038.2;
    waste.waste_units = 1361.6;
    waste.kinds[0].units = 1115.8;
    let board = Board::new(waste, Span::Week);
    assert_eq!(board.header().as_deref(), Some("15% of this week's burn was waste you can fix"));
    assert_eq!(board.rows()[0].share, "12%", "1,116 of 9,038");
    assert_eq!(board.lines_of("subagent_context", &mock::titles(), &[])[0].share, "0.2%", "21 of 9,038");
}

#[test]
fn a_cause_is_drawn_from_the_slot_folder_or_the_repository_root_and_long_ones_lose_their_middle() {
    let root = "/Users/nickmele/Development/armada";
    let roots = [root.to_string()];
    let slotted = |n: u8, rest: &str| format!("{root}/.armada/slots/slot-{n}/{rest}");
    assert_eq!(cause_said(&slotted(7, "crates/ipc/src/lib.rs"), &roots), "crates/ipc/src/lib.rs");
    assert_eq!(cause_said(&format!("{root}/docs/practices/native-bridge.md"), &roots), "docs/practices/native-bridge.md");
    assert_eq!(cause_said("/elsewhere/.armada/slots/slot-12/CLAUDE.md", &[]), "CLAUDE.md", "a slot folder is enough without a root");
    assert_eq!(cause_said("mcp__armada-fleet__waiting_for", &roots), "mcp__armada-fleet__waiting_for", "a tool is as it is");
    let long = cause_said(&slotted(3, "crates/bridge-native/src/job_view/story/model.rs"), &roots);
    assert_eq!(long.chars().count(), 40);
    assert!(long.contains('…') && long.ends_with("story/model.rs"), "{long}");
}

#[test]
fn only_the_file_and_tool_kinds_carry_a_cause_column_and_it_names_the_largest_cause() {
    let board = week();
    let roots = mock::roots();
    let repeat = board.lines_of("repeat_reads", &mock::titles(), &roots);
    assert!(repeat[0].cause.ends_with("story/model.rs") && repeat[0].cause.contains('…'), "the largest Job's largest file: {}", repeat[0].cause);
    assert!(repeat[0].cause_tip.starts_with("/Users/") && repeat[0].cause_tip.ends_with("story/model.rs"), "the tooltip holds the whole name");
    let big = board.lines_of("big_results", &mock::titles(), &roots);
    assert!(big.iter().any(|line| line.cause == "mcp__armada-fleet__get_job_lifecycles"));
    assert!(board.lines_of("subagent_context", &mock::titles(), &roots).iter().all(|line| line.cause.is_empty() && line.cause_tip.is_empty()));
    assert!(shows_causes("repeat_reads") && shows_causes("big_results") && !shows_causes("cold_cache"));
}

#[test]
fn a_figure_names_what_it_counted_in_its_tooltip_and_never_beside_it() {
    let rows = week().rows();
    assert_eq!(rows[0].tip, "41 requests");
    assert_eq!(rows[2].tip, "41 wakes");
    assert_eq!(times_said(1, look_of("cold_cache")), "1 wake");
    assert!(rows.iter().all(|row| row.share.ends_with('%')), "the row's figure is a share alone");
}

#[test]
fn a_job_with_no_title_shows_its_short_id_and_a_missing_peak_is_empty() {
    let board = week();
    let lines = board.lines_of("fresh_starts", &mock::titles(), &[]);
    let untitled = lines.iter().find(|line| line.id == "5d1e9c42b7a8").expect("the Job with no title");
    assert_eq!(untitled.title, "5d1e9c42");
    assert_eq!(untitled.peak, "", "Fleet recorded none");
    let titled = board.lines_of("subagent_context", &mock::titles(), &[]);
    assert_eq!((titled[0].share.as_str(), titled[0].peak.as_str()), ("5.1%", "870k"));
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
    selection.keep(&["a", "b"], &["a", "b"]);
    assert_eq!((selection.cursor.as_deref(), selection.opened.as_deref()), (Some("b"), Some("b")));
    selection.keep(&["a", "c"], &["a", "c"]);
    assert_eq!((selection.cursor.as_deref(), selection.opened.as_deref()), (Some("a"), None), "the cursor falls to the first row and the panel shuts");
    selection.keep(&[], &[]);
    assert_eq!(selection.cursor, None);
    let mut on_setting = Selection { cursor: Some("s".into()), opened: Some("b".into()) };
    on_setting.keep(&["s", "b"], &["b"]);
    assert_eq!((on_setting.cursor.as_deref(), on_setting.opened.as_deref()), (Some("s"), Some("b")), "a setting's row is somewhere the keys rest, and no panel's kind");
}

#[test]
fn the_window_keys_are_day_week_and_fortnight_in_that_order() {
    assert_eq!([0, 1, 2].map(|slot| Span::at(slot).map(Span::label)), [Some("Day"), Some("Week"), Some("Fortnight")]);
    assert_eq!(Span::at(3), None);
}

#[test]
fn every_window_has_the_same_kinds_in_the_same_order() {
    let order = |span| Board::new(mock::waste(span), span).kinds().into_iter().map(str::to_string).collect::<Vec<_>>();
    assert_eq!(order(Span::Day), order(Span::Week));
    assert_eq!(order(Span::Fortnight), order(Span::Week));
}

#[test]
fn a_window_press_moves_the_switch_at_once_and_keeps_the_old_rows_until_the_answer() {
    let mut held = Held::new(Span::Week);
    held.arrived(Span::Week, Ok(mock::waste(Span::Week)));
    assert!(matches!(held.shown(), Shown::Read { pending: false, .. }));
    assert!(held.pick(Span::Day), "the press is taken");
    assert_eq!(held.span(), Span::Day, "the switch is on Day in the same frame");
    match held.shown() {
        Shown::Read { board, pending } => {
            assert!(pending, "marked pending");
            assert_eq!(board.span(), Span::Week, "the rows are still the week's");
            assert_eq!(board.header().as_deref(), Some("32% of this week's burn was waste you can fix"));
        }
        other => panic!("the old rows stay, not {other:?}"),
    }
    held.arrived(Span::Day, Ok(mock::waste(Span::Day)));
    match held.shown() {
        Shown::Read { board, pending } => {
            assert!(!pending, "the answer ends the wait");
            assert_eq!(board.span(), Span::Day);
            assert_eq!(board.header().as_deref(), Some("32% of today's burn was waste you can fix"));
        }
        other => panic!("the day's rows, not {other:?}"),
    }
    assert!(!held.pick(Span::Day), "a press on the window shown asks nothing");
}

#[test]
fn an_answer_for_a_window_the_switch_has_left_is_dropped() {
    let mut held = Held::new(Span::Week);
    held.arrived(Span::Week, Ok(mock::waste(Span::Week)));
    held.pick(Span::Day);
    held.pick(Span::Fortnight);
    held.arrived(Span::Day, Ok(mock::waste(Span::Day)));
    match held.shown() {
        Shown::Read { board, pending } => assert_eq!((board.span(), pending), (Span::Week, true), "the late day changed nothing"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_read_that_fails_shows_fleets_reason_and_a_later_answer_clears_it() {
    let mut held = Held::new(Span::Week);
    assert_eq!(held.shown(), Shown::Loading, "nothing asked yet is the skeleton");
    held.arrived(Span::Week, Err("Fleet answered 500".into()));
    assert_eq!(held.shown(), Shown::Failed("Fleet answered 500"));
    assert!(held.failed() && held.board().is_none());
    held.arrived(Span::Week, Ok(mock::waste(Span::Week)));
    assert!(matches!(held.shown(), Shown::Read { pending: false, .. }), "the next read puts it right");
}

#[test]
fn a_failed_window_switch_is_failed_but_a_failed_refresh_keeps_its_rows() {
    let mut held = Held::new(Span::Week);
    held.arrived(Span::Week, Ok(mock::waste(Span::Week)));
    held.arrived(Span::Week, Err("Fleet is not connected".into()));
    assert!(matches!(held.shown(), Shown::Read { pending: false, .. }), "the minute's read failed: the rows are still the week's");
    held.pick(Span::Day);
    held.arrived(Span::Day, Err("Fleet answered 500".into()));
    assert_eq!(held.shown(), Shown::Failed("Fleet answered 500"), "rows for another window are not drawn under the Day switch");
    assert!(held.pick(Span::Week), "the way out is any window");
    assert!(matches!(held.shown(), Shown::Read { pending: false, .. }), "back on the week, the rows held are its own");
}

fn changing() -> Changing {
    let mut changing = Changing::default();
    changing.arrived(Span::Week, Span::Week, Ok(mock::adjustments(&[])));
    changing
}

const CACHE: &str = "tokens.subagentCacheTtl";

#[test]
fn a_row_a_setting_with_its_value_in_force_its_suggestion_what_it_cuts_and_what_it_saves() {
    let board = week();
    let rows = changing().rows(Some(&board));
    assert_eq!(rows.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(), ["Subagent cache length", "Compact a Job at", "Stop an idle Job after"], "Fleet's order");
    assert_eq!(
        rows.iter().map(|row| (row.now.as_str(), row.choices[row.chosen].as_str(), row.cuts.as_str(), row.saves.as_str())).collect::<Vec<_>>(),
        [
            ("5 min", "1 hour", "Cold cache after a pause", "5.5%"),
            ("400k", "200k", "Main thread ran past 300k context", "8.3%"),
            ("55 min", "55 min", "Fresh starts and compactions", "-"),
        ],
        "the dropdown is preset to the suggestion, and to the value in force where there is none"
    );
    assert_eq!(rows[0].choices, ["5 min", "1 hour"]);
}

#[test]
fn what_a_setting_cuts_is_spelled_from_its_id_where_the_waste_is_not_read_yet() {
    assert_eq!(changing().rows(None)[0].cuts, "Cold cache");
}

#[test]
fn apply_is_on_only_where_the_dropdown_stands_on_another_value_than_the_one_in_force() {
    let mut changing = changing();
    let can = |changing: &Changing| changing.rows(None).iter().map(|row| row.can_apply).collect::<Vec<_>>();
    assert_eq!(can(&changing), [true, true, false], "two are preset to a suggestion, one stands on what is in force");
    changing.choose(CACHE, 0);
    assert_eq!(can(&changing), [false, true, false], "back on the value in force");
    changing.choose("timeouts.sessionQuietSeconds", 0);
    assert_eq!(can(&changing), [false, true, true]);
    assert_eq!(changing.apply(CACHE), None, "a press with nothing to change sends nothing");
}

#[test]
fn a_press_on_apply_shows_the_new_value_pending_at_once_and_success_leaves_it_in_force() {
    let mut changing = changing();
    assert_eq!(changing.apply(CACHE), Some(ipc::SettingValue::Text("1h".into())), "the value the dropdown stands on");
    let row = &changing.rows(None)[0];
    assert_eq!((row.now.as_str(), row.pending, row.can_apply), ("1 hour", true, false), "the new value, waiting on Fleet, and no second press");
    assert_eq!(changing.apply(CACHE), None, "a second press while it is on its way sends nothing");
    changing.saved(CACHE, false);
    let row = &changing.rows(None)[0];
    assert_eq!((row.now.as_str(), row.pending, row.note.clone()), ("1 hour", false, None), "in force until the reading says so");
    changing.arrived(Span::Week, Span::Week, Ok(mock::adjustments(&[(CACHE.to_string(), ipc::SettingValue::Text("1h".into()))])));
    let row = &changing.rows(None)[0];
    assert_eq!((row.now.as_str(), row.choices[row.chosen].as_str(), row.can_apply, row.saves.as_str()), ("1 hour", "1 hour", false, "-"), "read back: nothing left to apply and nothing left to save");
}

#[test]
fn a_refused_change_puts_the_old_value_back_and_says_why_and_a_new_pick_clears_it() {
    let mut changing = changing();
    changing.apply(CACHE);
    changing.refused(CACHE, "settings.json was not written".into());
    let row = &changing.rows(None)[0];
    assert_eq!((row.now.as_str(), row.pending), ("5 min", false), "the value in force is back");
    assert_eq!(row.note, Some(Note::Failed("settings.json was not written".into())));
    assert!(row.can_apply && row.choices[row.chosen] == "1 hour", "the dropdown stays where it was, so a press tries again");
    changing.choose(CACHE, 0);
    assert_eq!(changing.rows(None)[0].note, None, "a new pick clears the reason");
    changing.refused(CACHE, "late".into());
    assert_eq!(changing.rows(None)[0].note, None, "a refusal for a change that is not on its way says nothing");
}

#[test]
fn a_setting_that_applies_after_a_restart_says_so_until_fleet_starts_again() {
    let mut changing = changing();
    let quiet = "timeouts.sessionQuietSeconds";
    changing.choose(quiet, 1);
    assert_eq!(changing.apply(quiet), Some(ipc::SettingValue::Integer(1800)));
    changing.saved(quiet, true);
    assert_eq!(changing.rows(None)[2].note, Some(Note::Restart));
    changing.arrived(Span::Week, Span::Week, Ok(mock::adjustments(&[(quiet.to_string(), ipc::SettingValue::Integer(1800))])));
    assert_eq!(changing.rows(None)[2].note, Some(Note::Restart), "a reading does not end the wait");
    changing.restarted();
    assert_eq!(changing.rows(None)[2].note, None);
}

#[test]
fn a_reading_for_a_window_the_switch_has_left_is_dropped_and_a_refused_one_keeps_the_last() {
    let mut changing = changing();
    let mut other = mock::adjustments(&[]);
    other.settings.truncate(1);
    changing.arrived(Span::Day, Span::Fortnight, Ok(other.clone()));
    assert_eq!(changing.keys().len(), 3, "the late day changed nothing");
    changing.arrived(Span::Fortnight, Span::Fortnight, Err("Fleet answered 404".into()));
    assert_eq!(changing.keys().len(), 3, "a refused read leaves the rows where they are");
    changing.arrived(Span::Fortnight, Span::Fortnight, Ok(other));
    assert_eq!((changing.keys().len(), changing.span()), (1, Some(Span::Fortnight)));
    assert!(Changing::default().rows(None).is_empty() && Changing::default().keys().is_empty(), "nothing read, nothing drawn");
}

fn changes(zone: &chrono::FixedOffset) -> Vec<ChangeRow> {
    changing().changes_in(Some(&week()), zone)
}

#[test]
fn a_change_with_a_share_since_says_what_was_changed_the_share_before_and_after_and_how_far_it_moved() {
    let row = changes(&east(0)).remove(1);
    assert_eq!((row.change.as_str(), row.date.as_str()), ("Subagent cache length: 5 min → 1 hour", "17 Nov"));
    assert_eq!((row.before.as_str(), row.kind.as_str()), ("12.3% of your burn", "Cold cache after a pause"));
    assert_eq!(
        row.after,
        After::Since {
            share: "3.1%".into(),
            moved: Moved::Down,
            chip: "down 9.2 points".into(),
            tip: "Cold cache after a pause: 12.3% of your burn in the week before, 3.1% since".into()
        }
    );
}

#[test]
fn the_chip_is_the_passed_hue_with_a_falling_glyph_for_a_fall_and_the_warning_hue_with_a_rising_one_for_a_rise() {
    use crate::look::token;
    assert_eq!(Moved::Down.mark(), (Some("trending-down"), token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG));
    assert_eq!(Moved::Up.mark(), (Some("trending-up"), token::STATUS_AWAITING_REVIEW, token::STATUS_AWAITING_REVIEW_BG));
    assert_eq!(Moved::Same.mark().0, None, "no change has no glyph");
}

#[test]
fn a_change_the_waste_grew_after_says_up_in_the_warning_direction() {
    let mut adjustments = mock::adjustments(&[]);
    adjustments.changes[1].since_share = Some(0.143);
    let mut changing = Changing::default();
    changing.arrived(Span::Week, Span::Week, Ok(adjustments));
    match changing.changes_in(Some(&week()), &east(0)).remove(1).after {
        After::Since { moved, chip, .. } => assert_eq!((moved, chip.as_str()), (Moved::Up, "up 2.0 points")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_change_the_waste_did_not_move_after_says_no_change() {
    let mut adjustments = mock::adjustments(&[]);
    adjustments.changes[1].since_share = Some(0.123);
    let mut changing = Changing::default();
    changing.arrived(Span::Week, Span::Week, Ok(adjustments));
    match changing.changes_in(Some(&week()), &east(0)).remove(1).after {
        After::Since { moved, chip, .. } => assert_eq!((moved, chip.as_str()), (Moved::Same, "no change")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_change_a_day_has_not_passed_on_says_when_it_will_be_ready_a_day_after_it_in_the_macs_zone() {
    let row = changes(&east(0)).remove(0);
    assert_eq!((row.change.as_str(), row.date.as_str(), row.before.as_str(), row.kind.as_str()), ("Compact a Job at: 400k → 250k", "18 Nov", "7.5% of your burn", "Main thread ran past 300k context"));
    assert_eq!(row.after, After::Ready { said: "Ready 19 Nov".into(), tip: "After a day, this compares the waste's share since the change with the week before".into() });
    let west = changes(&east(-9)).remove(0);
    assert_eq!((west.date.as_str(), west.after), ("17 Nov", After::Ready { said: "Ready 18 Nov".into(), tip: "After a day, this compares the waste's share since the change with the week before".into() }), "08:00 in UTC is the evening before at UTC-9");
}

#[test]
fn with_no_change_made_there_are_no_rows_to_draw() {
    let mut adjustments = mock::adjustments(&[]);
    adjustments.changes.clear();
    let mut changing = Changing::default();
    changing.arrived(Span::Week, Span::Week, Ok(adjustments));
    assert!(changing.changes(Some(&week())).is_empty());
    assert_eq!(changing.rows(None).len(), 3, "the settings are still there");
}

#[test]
fn the_scenarios_that_open_on_the_page_are_the_five() {
    assert!(["usage-week", "usage-loading", "usage-empty", "usage-failed", "usage-apply-failed"].iter().all(|name| super::is_scenario(name)));
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
        assert!(drawn(cx, Role::Heading, "32% of this week's burn was waste you can fix"));
        assert!(drawn(cx, Role::Row, "Subagents ran past 300k context"));
        assert!(drawn(cx, Role::Row, "Fresh starts and compactions"));
        for heading in ["Settings that change it", "After a change", "Waste by kind"] {
            assert!(drawn(cx, Role::Heading, heading), "{heading}");
        }
        assert!(drawn(cx, Role::Row, "Subagent cache length"));
        assert!(drawn(cx, Role::ComboBox, "Change Subagent cache length to"));
        assert!(drawn(cx, Role::Button, "Apply Compact a Job at"));
        for column in ["Change", "Before", "After"] {
            assert!(drawn(cx, Role::ColumnHeader, column), "{column}");
        }
        assert!(drawn(cx, Role::Row, &format!("Compact a Job at: 400k → 250k, {}", crate::usage_page::model::date_said("2026-11-18T08:00:00Z", 0))));
        assert!(drawn(cx, Role::Row, &format!("Subagent cache length: 5 min → 1 hour, {}", crate::usage_page::model::date_said("2026-11-17T14:20:00Z", 0))));
        assert!(drawn(cx, Role::Status, "down 9.2 points"), "the chip of the change that has a share since");

        assert_eq!(opened(&screen, cx), None, "nothing is open until a row is");
    }

    #[gpui::test]
    fn enter_opens_a_kinds_jobs_and_escape_shuts_them(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-week");
        redraw(cx);
        cx.simulate_keystrokes("j j j enter");
        redraw(cx);
        assert!(drawn(cx, Role::Table, "Jobs hit by Subagents ran past 300k context"), "the keys start on the first setting, and three rows on is the first kind");
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
        cx.simulate_keystrokes("down down down down enter d");
        redraw(cx);
        assert!(drawn(cx, Role::Heading, "32% of today's burn was waste you can fix"), "the day's figures");
        assert!(drawn(cx, Role::Table, "Jobs hit by Main thread ran past 300k context"), "the panel stays on its kind");
        cx.simulate_keystrokes("f");
        redraw(cx);
        assert!(drawn(cx, Role::Heading, "32% of this fortnight's burn was waste you can fix"), "the fortnight's figures");
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
    fn a_refused_read_draws_fleets_reason_and_keeps_the_switch(cx: &mut TestAppContext) {
        let (_, cx) = page(cx, "usage-failed");
        redraw(cx);
        assert!(drawn(cx, Role::Alert, "Not read"));
        assert!(drawn(cx, Role::RadioGroup, "Window"), "another window can be asked");
        assert!(!drawn(cx, Role::Row, "Subagents ran past 300k context"));
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

    /// The rows the page draws for the settings, as the model reads them.
    fn rows(screen: &Entity<OperatorScreen>, cx: &mut VisualTestContext) -> Vec<crate::usage_page::model::SettingRow> {
        let page: Entity<UsageScreen> = screen.read_with(cx, |screen, _| screen.usage_for_test().expect("the screen holds the page"));
        page.read_with(cx, |page, _| page.changing.rows(page.board()))
    }

    #[gpui::test]
    fn a_applies_the_row_the_keys_are_on_and_the_page_reads_it_back(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-week");
        redraw(cx);
        assert_eq!(rows(&screen, cx)[0].now, "5 min");
        cx.simulate_keystrokes("a");
        redraw(cx);
        let row = rows(&screen, cx).remove(0);
        assert_eq!((row.now.as_str(), row.pending, row.saves.as_str()), ("1 hour", false, "-"), "taken, read again, and nothing left to save");
        assert!(drawn(cx, Role::Label, "1 hour now"));
    }

    #[gpui::test]
    fn enter_on_a_setting_opens_its_values_and_a_pick_moves_the_dropdown_without_sending(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-week");
        redraw(cx);
        cx.simulate_keystrokes("enter");
        redraw(cx);
        assert!(drawn(cx, Role::Menu, "Change Subagent cache length to"), "the list of values stands under the dropdown");
        cx.simulate_keystrokes("enter");
        redraw(cx);
        let row = rows(&screen, cx).remove(0);
        assert_eq!((row.now.as_str(), row.choices[row.chosen].as_str(), row.can_apply), ("5 min", "5 min", false), "the first value picked: the dropdown is on it and nothing was sent");
        assert_eq!(opened(&screen, cx), None, "no kind's panel opened");
        cx.simulate_keystrokes("a");
        redraw(cx);
        assert_eq!(rows(&screen, cx)[0].now, "5 min", "Apply on the value in force sends nothing");
    }

    #[gpui::test]
    fn a_change_fleet_refuses_leaves_the_old_value_and_says_why(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-apply-failed");
        redraw(cx);
        cx.simulate_keystrokes("a");
        redraw(cx);
        let row = rows(&screen, cx).remove(0);
        assert_eq!((row.now.as_str(), row.pending, row.can_apply), ("5 min", false, true));
        assert!(drawn(cx, Role::Alert, "settings.json was not written: it is read only"));
    }

    #[gpui::test]
    fn a_setting_that_applies_at_a_restart_says_so_in_its_row(cx: &mut TestAppContext) {
        let (screen, cx) = page(cx, "usage-week");
        redraw(cx);
        cx.simulate_keystrokes("j j enter down enter a");
        redraw(cx);
        let row = rows(&screen, cx).remove(2);
        assert_eq!((row.now.as_str(), row.note), ("30 min", Some(crate::usage_page::model::Note::Restart)));
        assert!(drawn(cx, Role::Status, "Applies when Fleet restarts"));
    }
}
