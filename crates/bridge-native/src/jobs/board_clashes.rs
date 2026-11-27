//! A feature's board, a row for each clash among its Jobs: two Jobs that write the same files, and the order they
//! settled, read off the Jobs' own records (`clashes`). Its tooltip is their why. Drawn under What holds what's rows.

use super::*;
use ipc::{ClashStanding, JobClash};

/// A clash as its row draws it.
pub(super) struct Clash {
    pub files: SharedString,
    pub said: SharedString,
    pub why: SharedString,
}

/// The clashes the Jobs' records carry, once each though both Jobs carry it, in the order first met.
pub(super) fn clashes_of(jobs: &[&SessionRecord]) -> Vec<Clash> {
    let mut seen: Vec<&str> = Vec::new();
    let mut rows = Vec::new();
    for clash in jobs.iter().flat_map(|job| &job.clashes) {
        if !seen.contains(&clash.id.as_str()) {
            seen.push(&clash.id);
            rows.push(Clash { files: format!("Two Jobs write {}", files_of(&clash.files)).into(), said: said_of(clash).into(), why: clash.why.clone().into() });
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

fn said_of(clash: &JobClash) -> String {
    let name = |id: &ipc::SessionId| {
        let job = clash.jobs.iter().find(|job| &job.session_id == id);
        job.map(|job| if job.title.is_empty() { job.address.clone() } else { job.title.clone() }).unwrap_or_default()
    };
    let order = |first: &ipc::SessionId| {
        let waits = clash.jobs.iter().find(|job| &job.session_id != first).map(|job| name(&job.session_id)).unwrap_or_default();
        format!("\"{waits}\" waits until \"{}\" lands.", name(first))
    };
    match (clash.standing, clash.first.as_ref()) {
        (ClashStanding::Waiting, _) => "Waiting on you".to_string(),
        (ClashStanding::Agreed, None) => "They agreed: both go on.".to_string(),
        (ClashStanding::Agreed, Some(first)) => format!("They agreed: {}", order(first)),
        (ClashStanding::Ruled, Some(first)) => format!("You chose: {}", order(first)),
        (ClashStanding::Ruled, None) => "You chose: both go on.".to_string(),
    }
}

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
                    .child(div().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(clash.said.clone())),
            )
            .when(!why.is_empty(), |row| row.tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(why.clone()).build(window, cx))),
        Role::Row,
        format!("{}, {}", clash.files, clash.said),
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
    fn a_clash_both_rows_carry_is_one_row_saying_the_order_they_settled() {
        let all = clashing(clash(ClashStanding::Agreed, Some("mock-02")));
        let rows = clashes_of(&all.iter().collect::<Vec<_>>());
        let [row] = rows.as_slice() else { panic!("one row") };
        assert_eq!((row.files.as_ref(), row.said.as_ref(), row.why.as_ref()), ("Two Jobs write src/log.rs", "They agreed: \"Tidy the log\" waits until \"Quiet the log\" lands.", "Its change is the smaller one."));
        assert_eq!(said_of(&clash(ClashStanding::Agreed, None)), "They agreed: both go on.");
        assert_eq!(said_of(&clash(ClashStanding::Waiting, None)), "Waiting on you", "raised and not settled");
    }

    #[gpui::test]
    fn a_features_board_draws_a_row_for_the_clash_among_its_jobs(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init_app);
        let all = clashing(clash(ClashStanding::Agreed, Some("mock-02")));
        let list = cx.new(|cx| JobsList::mock(&all, cx));
        let held = list.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| JobsScreen::new(held, None, window, cx));
        let library = crate::features::mock::library_on(&[("job-list-and-live-status", "mock-01"), ("job-list-and-live-status", "mock-02")]);
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
        assert!(list.read_with(cx, |list, _| list.board().is_some()), "the board stands");
        let named = "Two Jobs write src/log.rs, They agreed: \"Tidy the log\" waits until \"Quiet the log\" lands.";
        let drawn = cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Row, named).is_some()
        });
        assert!(drawn, "the clash is a row on the board");
    }
}
