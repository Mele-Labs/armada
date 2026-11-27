//! A feature's board and the clashes among its Jobs: two Jobs that write the same files, and the order they settled, read off
//! the Jobs' own records (`clashes`). Only a clash nobody has settled is a row, under What holds what's rows: the one thing
//! on the board that needs him. A settled one is a line on the Job that waits, with its why in that line's tooltip.

use super::*;
use ipc::{ClashStanding, JobClash};

/// A clash as the board reads it.
pub(super) struct Clash {
    pub files: SharedString,
    pub why: SharedString,
    pub standing: ClashStanding,
    /// The Job that waits and the title of the one it waits for, where the clash was settled with one going first.
    pub waits: Option<Wait>,
}

pub(super) struct Wait {
    pub who: String,
    pub on: String,
    pub ruled: bool,
}

impl Clash {
    /// Nobody has settled it: a row of its own.
    pub(super) fn open(&self) -> bool {
        matches!(self.standing, ClashStanding::Waiting)
    }
}

/// The clashes the Jobs' records carry, once each though both Jobs carry it, in the order first met.
pub(super) fn clashes_of(jobs: &[&SessionRecord]) -> Vec<Clash> {
    let mut seen: Vec<&str> = Vec::new();
    let mut rows = Vec::new();
    for clash in jobs.iter().flat_map(|job| &job.clashes) {
        if !seen.contains(&clash.id.as_str()) {
            seen.push(&clash.id);
            rows.push(Clash { files: format!("Two Jobs write {}", files_of(&clash.files)).into(), why: clash.why.clone().into(), standing: clash.standing, waits: wait_of(clash) });
        }
    }
    rows
}

fn files_of(files: &[String]) -> String {
    const NAMED: usize = 3;
    let named = files.iter().take(NAMED).cloned().collect::<Vec<_>>().join(", ");
    match files.len().saturating_sub(NAMED) {
        0 => named,
        rest => format!("{named} and {rest} more"),
    }
}

fn wait_of(clash: &JobClash) -> Option<Wait> {
    let ruled = match clash.standing {
        ClashStanding::Waiting => return None,
        ClashStanding::Agreed => false,
        ClashStanding::Ruled => true,
    };
    let first = clash.first.as_ref()?;
    let name = |job: &ipc::OperatorAskJob| if job.title.is_empty() { job.address.clone() } else { job.title.clone() };
    let on = clash.jobs.iter().find(|job| &job.session_id == first).map(name).unwrap_or_default();
    let who = clash.jobs.iter().find(|job| &job.session_id != first)?;
    Some(Wait { who: who.session_id.as_str().to_string(), on, ruled })
}

/// What a settled clash adds to the second line of the Job `id`, and its why for that line's tooltip: the Jobs it waits for,
/// named once each. The Job that goes first, and a clash where both go on, add nothing.
pub(super) fn folded(clashes: &[Clash], id: &str) -> Option<(String, String)> {
    let mine: Vec<(&Wait, &Clash)> = clashes.iter().filter_map(|clash| clash.waits.as_ref().filter(|wait| wait.who == id).map(|wait| (wait, clash))).collect();
    if mine.is_empty() {
        return None;
    }
    let mut names: Vec<String> = Vec::new();
    let mut whys: Vec<&str> = Vec::new();
    for (wait, clash) in &mine {
        let quoted = format!("\"{}\"", wait.on);
        if !names.contains(&quoted) {
            names.push(quoted);
        }
        if !clash.why.is_empty() && !whys.contains(&clash.why.as_ref()) {
            whys.push(&clash.why);
        }
    }
    let named = match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.join(""),
    };
    let ruled = mine.iter().filter(|(wait, _)| wait.ruled).count();
    let by = if ruled == 0 {
        "as they agreed"
    } else if ruled == mine.len() {
        "as you chose"
    } else {
        "as settled"
    };
    Some((format!("Waits for {named} to land, {by}"), whys.join("\n")))
}

/// What an unsettled clash's row says under the files.
const SAID: &str = "Waiting on you";

/// One clash's row: the warning tone's `circle-pause` on a tint of it.
pub(super) fn clash_row(look: &Look, at: usize, clash: &Clash) -> gpui::AnyElement {
    let ink = look.color(token::NOTICE_CAUTION);
    let why = clash.why.clone();
    labelled(
        div()
            .id(("board-clash", at))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .flex_none()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::NOTICE_CAUTION_BG))
            .child(svg().path(icon_path("circle-pause")).flex_none().size(look.length(token::SPACE_4)).text_color(ink))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(div().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(clash.files.clone()))
                    .child(div().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SAID)),
            )
            .when(!why.is_empty(), |row| row.tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(why.clone()).build(window, cx))),
        Role::Row,
        format!("{}, {SAID}", clash.files),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::OperatorAskJob;

    fn clash(standing: ClashStanding, first: Option<&str>) -> JobClash {
        let job = |id: &str, title: &str| OperatorAskJob { session_id: ipc::SessionId::carried(id), address: format!("s-{id}"), title: title.into() };
        JobClash {
            id: "clash:s-mock-01:s-mock-02".into(),
            jobs: vec![job("mock-01", "Tidy the log"), job("mock-02", "Quiet the log")],
            files: vec!["src/log.rs".into()],
            standing,
            first: first.map(ipc::SessionId::carried),
            why: "Its change is the smaller one.".into(),
            at: "2026-11-27T12:00:00Z".into(),
        }
    }

    /// Two of the three Jobs, both writing to Job list and live status, carrying the one clash on both rows.
    fn clashing(held: JobClash) -> Vec<SessionRecord> {
        let mut all: Vec<SessionRecord> = crate::mock::jobs("jobs-many").expect("the fixture decodes").into_iter().filter(|job| ["mock-01", "mock-02"].contains(&job.id.as_str())).collect();
        for job in &mut all {
            job.attachments.retain(|one| one.kind != "started_by");
            job.snoozed_until = None;
            job.mac = None;
            job.clashes = vec![held.clone()];
        }
        all
    }

    #[test]
    fn a_clash_both_rows_carry_is_read_once_and_a_settled_one_is_a_line_on_the_job_that_waits() {
        let all = clashing(clash(ClashStanding::Agreed, Some("mock-02")));
        let read = clashes_of(&all.iter().collect::<Vec<_>>());
        let [one] = read.as_slice() else { panic!("one clash") };
        assert_eq!((one.files.as_ref(), one.why.as_ref(), one.open()), ("Two Jobs write src/log.rs", "Its change is the smaller one.", false));
        assert_eq!(folded(&read, "mock-01"), Some(("Waits for \"Quiet the log\" to land, as they agreed".to_string(), "Its change is the smaller one.".to_string())), "the Job that waits carries the line and the why");
        assert_eq!(folded(&read, "mock-02"), None, "the Job that goes first gets nothing");
        let ruled = clashes_of(&clashing(clash(ClashStanding::Ruled, Some("mock-01"))).iter().collect::<Vec<_>>());
        assert_eq!(folded(&ruled, "mock-02").map(|line| line.0), Some("Waits for \"Tidy the log\" to land, as you chose".to_string()));
        let both = clashes_of(&clashing(clash(ClashStanding::Agreed, None)).iter().collect::<Vec<_>>());
        assert_eq!((both[0].open(), folded(&both, "mock-01"), folded(&both, "mock-02")), (false, None, None), "both go on adds nothing");
        let waiting = clashes_of(&clashing(clash(ClashStanding::Waiting, None)).iter().collect::<Vec<_>>());
        assert_eq!((waiting[0].open(), folded(&waiting, "mock-01")), (true, None), "an unsettled clash is a row, not a line");
    }

    #[test]
    fn a_job_waiting_on_several_names_each_job_once() {
        let mut other = clash(ClashStanding::Agreed, Some("mock-02"));
        other.id = "clash:s-mock-01:s-mock-02:more".into();
        let mut all = clashing(clash(ClashStanding::Agreed, Some("mock-02")));
        all[0].clashes.push(other);
        let read = clashes_of(&all.iter().collect::<Vec<_>>());
        assert_eq!(read.len(), 2);
        assert_eq!(folded(&read, "mock-01").map(|line| line.0), Some("Waits for \"Quiet the log\" to land, as they agreed".to_string()));
    }

    /// The feature's board over `all`, opened from its heading, with the keys on it.
    fn board_over<'a>(all: &[SessionRecord], ids: &[&str], cx: &'a mut gpui::TestAppContext) -> (Entity<JobsList>, Entity<board::LeadBoard>, &'a mut gpui::VisualTestContext) {
        cx.update(crate::init_app);
        let list = cx.new(|cx| JobsList::mock(all, cx));
        let held = list.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| JobsScreen::new(held, None, window, cx));
        let on: Vec<(&str, &str)> = ids.iter().map(|id| ("job-list-and-live-status", *id)).collect();
        let library = crate::features::mock::library_on(&on);
        list.update(cx, |list, cx| {
            list.pick_view("feature", cx);
            list.hold_libraries(vec![library], cx);
        });
        let act = Act { id: "open_focused", slot: 0, digit: None };
        cx.update(|window, cx| {
            list.update(cx, |list, cx| {
                list.focused = Some("lead:feature:armada/job-list-and-live-status".into());
                list.on_act(&act, window, cx);
            })
        });
        cx.run_until_parked();
        let board = list.read_with(cx, |list, _| list.board()).expect("the board stands");
        (list, board, cx)
    }

    fn drawn(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some()
        })
    }

    #[gpui::test]
    fn a_features_board_folds_an_agreed_clash_into_the_line_of_the_job_that_waits(cx: &mut gpui::TestAppContext) {
        let all = clashing(clash(ClashStanding::Agreed, Some("mock-02")));
        let waits = format!("{} \u{b7} Waits for \"Quiet the log\" to land, as they agreed", leads::holds(&all[0]));
        let (_, _, cx) = board_over(&all, &["mock-01", "mock-02"], cx);
        assert!(drawn(cx, Role::Row, &format!("{}, {waits}", title_of(&all[0]))), "the Job that waits says so on its second line");
        assert!(!drawn(cx, Role::Row, "Two Jobs write src/log.rs, Waiting on you"), "a settled clash is not a row");
        let first = format!("{}, {}", title_of(&all[1]), leads::holds(&all[1]));
        assert!(drawn(cx, Role::Row, &first), "the Job that goes first has nothing added");
    }

    #[gpui::test]
    fn a_features_board_draws_a_row_for_a_clash_nobody_has_settled(cx: &mut gpui::TestAppContext) {
        let all = clashing(clash(ClashStanding::Waiting, None));
        let (_, _, cx) = board_over(&all, &["mock-01", "mock-02"], cx);
        assert!(drawn(cx, Role::Row, "Two Jobs write src/log.rs, Waiting on you"), "the unsettled clash is a row on the board");
    }

    #[gpui::test]
    fn a_features_board_scrolls_to_a_row_past_the_window_and_the_keys_bring_theirs_into_view(cx: &mut gpui::TestAppContext) {
        let all: Vec<SessionRecord> = crate::mock::jobs("jobs-many").expect("the fixture decodes").into_iter().map(|mut job| {
            job.attachments.retain(|one| one.kind != "started_by");
            job.snoozed_until = None;
            job.mac = None;
            job
        }).collect();
        let live: Vec<String> = all.iter().filter(|job| job.state == SessionState::Live).map(|job| job.id.as_str().to_string()).collect();
        let ids: Vec<&str> = live.iter().map(String::as_str).collect();
        let (_, board, cx) = board_over(&all, &ids, cx);
        let scroll = board.read_with(cx, |board, _| board.scroll().clone());
        assert!(drawn(cx, Role::Heading, "Job list and live status"), "the title stands");
        assert!(scroll.max_offset().y > gpui::px(0.), "the rows past the window can be scrolled to: {:?}", scroll.max_offset());
        assert_eq!(scroll.offset().y, gpui::px(0.));
        cx.update(|window, cx| board.update(cx, |board, cx| window.focus(board.focus(), cx)));
        for _ in 0..ids.len() {
            cx.simulate_keystrokes("down");
        }
        cx.run_until_parked();
        assert!(scroll.offset().y < gpui::px(0.), "the keys moved the body to the last row: {:?}", scroll.offset());
        assert!(drawn(cx, Role::Heading, "Job list and live status"), "the title stays put");
    }
}
