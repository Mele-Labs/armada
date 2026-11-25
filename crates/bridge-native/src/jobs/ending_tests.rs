//! Close in a Job's row menu: the row reads ended at once and Fleet ends the Job in the background; a refusal puts it back.

use super::*;
use gpui::px;

/// A stand-in Fleet that answers every POST with `status` and `body`, and sends each request's path down `seen`.
fn stand_in_fleet(status: &'static str, body: &'static str, seen: std::sync::mpsc::Sender<String>) -> u16 {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("its address").port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
            let mut first = String::new();
            if reader.read_line(&mut first).is_err() {
                continue;
            }
            let mut length = 0usize;
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap_or(0);
                }
                line.clear();
            }
            let mut sent = vec![0u8; length];
            let _ = reader.read_exact(&mut sent);
            let _ = seen.send(first.split_whitespace().nth(1).unwrap_or_default().to_string());
            let mut stream = stream;
            let _ = write!(stream, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
        }
    });
    port
}

/// The Job's row in `scope`, if that scope lists it.
fn row_in(list: &Entity<JobsList>, scope: Scope, id: &str, cx: &mut gpui::VisualTestContext) -> Option<Row> {
    list.update(cx, |list, cx| {
        let was = list.scope;
        list.scope = scope;
        list.relist(cx);
        let row = list.rows().find(|row| row.id.as_ref() == id).cloned();
        list.scope = was;
        list.relist(cx);
        row
    })
}

/// The list over `all` with a Fleet that answers as told, a live Job of it, and the paths Fleet was sent.
fn over<'a>(status: &'static str, body: &'static str, cx: &'a mut gpui::TestAppContext) -> (Entity<JobsList>, String, Vec<SessionRecord>, std::sync::mpsc::Receiver<String>, &'a mut gpui::VisualTestContext) {
    cx.update(crate::init_app);
    let (seen, sent) = std::sync::mpsc::channel();
    let port = stand_in_fleet(status, body, seen);
    let fleet = cx.new(crate::fleet::FleetConnection::idle);
    fleet.update(cx, |fleet, cx| fleet.connect_at(port, cx));
    let all = crate::mock::jobs("jobs-many").expect("the fixture decodes");
    let (list, cx) = cx.add_window_view(|_, cx| JobsList::mock_over(&all, fleet, cx));
    let live = list.read_with(cx, |list, _| list.rows().find(|row| !row.ended && row.queued.is_none() && row.mac.is_none()).map(|row| row.id.to_string())).expect("a live row");
    (list, live, all, sent, cx)
}

fn close(list: &Entity<JobsList>, id: &str, cx: &mut gpui::VisualTestContext) {
    let at: SharedString = id.to_string().into();
    list.update_in(cx, |list, window, cx| list.open_menu(at, point(px(10.), px(10.)), true, window, cx));
    let menu = list.read_with(cx, |list, _| list.menu.as_ref().expect("the menu is open").menu.clone());
    menu.update(cx, |_, cx| cx.emit(Picked::Entry { id: "close", keep: false }));
}

fn reached(sent: &std::sync::mpsc::Receiver<String>, cx: &mut gpui::VisualTestContext) -> Option<String> {
    for _ in 0..50 {
        cx.run_until_parked();
        if let Ok(path) = sent.recv_timeout(std::time::Duration::from_millis(20)) {
            return Some(path);
        }
    }
    None
}

#[gpui::test]
fn close_ends_the_row_at_once_and_holds_it_until_fleet_serves_it_ended(cx: &mut gpui::TestAppContext) {
    let (list, live, all, sent, cx) = over("200 OK", "{}", cx);
    assert!(row_in(&list, Scope::Active, &live, cx).is_some());

    close(&list, &live, cx);
    // Before Fleet has answered anything, the Job has left the live Jobs and stands ended, marked until Fleet agrees.
    assert!(row_in(&list, Scope::Active, &live, cx).is_none(), "the row leaves the live Jobs on the press");
    let pressed = row_in(&list, Scope::Ended, &live, cx).expect("it stands among the ended Jobs");
    assert!(pressed.ended);
    assert_eq!(pressed.ending.as_deref(), Some("Ending"));
    assert_eq!(reached(&sent, cx).as_deref(), Some("/sessions/close"), "Fleet is asked to end it");

    // Fleet still serves it live while the Job winds up: the hold stands over that record.
    let mut served = all.clone();
    list.update(cx, |list, _| list.hold_ending(&mut served));
    assert_eq!(served.iter().find(|job| job.id.as_str() == live).map(|job| job.state), Some(SessionState::Ended));
    assert!(list.read_with(cx, |list, _| list.ending.contains_key(&live)));

    // Fleet serves it ended: the hold lets go and the mark goes with it.
    let mut ended: Vec<SessionRecord> = all
        .iter()
        .cloned()
        .map(|mut job| {
            if job.id.as_str() == live {
                job.state = SessionState::Ended;
            }
            job
        })
        .collect();
    list.update(cx, |list, cx| {
        list.hold_ending(&mut ended);
        list.replace_records(ended, cx);
    });
    assert!(list.read_with(cx, |list, _| list.ending.is_empty()));
    let settled = row_in(&list, Scope::Ended, &live, cx).expect("it stays ended");
    assert!(settled.ended && settled.ending.is_none(), "the mark goes once Fleet serves it ended");
}

#[gpui::test]
fn a_refused_close_puts_the_row_back_as_fleet_holds_it(cx: &mut gpui::TestAppContext) {
    let (list, live, _, sent, cx) = over("409 Conflict", r#"{"code":"fleet.terminal_session_unreachable","message":"end it there","run_id":"r"}"#, cx);
    close(&list, &live, cx);
    assert!(row_in(&list, Scope::Ended, &live, cx).is_some(), "the row ends on the press");
    assert_eq!(reached(&sent, cx).as_deref(), Some("/sessions/close"));
    for _ in 0..50 {
        cx.run_until_parked();
        if list.read_with(cx, |list, _| list.ending.is_empty()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let back = row_in(&list, Scope::Active, &live, cx).expect("Fleet refused, so the Job stands live again");
    assert!(!back.ended && back.ending.is_none());
}

#[test]
fn a_job_on_another_mac_says_where_it_ends() {
    let at = ipc::Instant::carried("2026-11-24T10:00:00Z".to_string());
    assert_eq!(Ending { was: (SessionState::Live, None, None), at: at.clone(), on: Some("Air".into()) }.said().as_ref(), "Ending on Air");
    assert_eq!(Ending { was: (SessionState::Live, None, None), at, on: None }.said().as_ref(), "Ending");
}
