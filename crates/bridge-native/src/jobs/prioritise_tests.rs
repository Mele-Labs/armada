//! Prioritise in a Job's row menu: its words follow the Job's mark, and a press sends the act to Fleet.

use super::*;
use gpui::px;

/// A stand-in Fleet that answers every POST with the Job as it was and sends each request's path and body down `seen`.
fn stand_in_fleet(seen: std::sync::mpsc::Sender<(String, String)>) -> u16 {
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
            let mut body = vec![0u8; length];
            let _ = reader.read_exact(&mut body);
            let path = first.split_whitespace().nth(1).unwrap_or_default().to_string();
            let _ = seen.send((path, String::from_utf8_lossy(&body).to_string()));
            let mut stream = stream;
            let _ = write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}");
        }
    });
    port
}

/// What the open menu says for Prioritise, if it offers it.
fn said(list: &Entity<JobsList>, cx: &mut gpui::VisualTestContext) -> Option<String> {
    list.read_with(cx, |list, cx| {
        let menu = list.menu.as_ref()?.menu.read(cx);
        menu.entries.iter().find_map(|entry| match entry {
            Entry::Act { id: "prioritise", label, .. } | Entry::Item { id: "prioritise", label, .. } => Some(label.to_string()),
            _ => None,
        })
    })
}

fn open(list: &Entity<JobsList>, id: &str, cx: &mut gpui::VisualTestContext) {
    let id: SharedString = id.to_string().into();
    list.update_in(cx, |list, window, cx| list.open_menu(id, point(px(10.), px(10.)), true, window, cx));
}

#[gpui::test]
fn the_row_menu_reads_prioritise_then_stop_prioritising_and_a_press_sends_the_act(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init_app);
    let (seen, sent) = std::sync::mpsc::channel();
    let port = stand_in_fleet(seen);
    let fleet = cx.new(crate::fleet::FleetConnection::idle);
    fleet.update(cx, |fleet, cx| fleet.connect_at(port, cx));
    let all = crate::mock::jobs("jobs-many").expect("the fixture decodes");
    let live = all.iter().find(|job| job.state == ipc::SessionState::Live).expect("a live Job").id.as_str().to_string();
    let (list, cx) = cx.add_window_view(|_, cx| JobsList::mock_over(&all, fleet, cx));

    open(&list, &live, cx);
    assert_eq!(said(&list, cx).as_deref(), Some("Prioritise"), "a Job without the mark offers it");
    assert!(list.read_with(cx, |list, _| list.rows().all(|row| !row.prioritised)), "no row wears the mark yet");

    // A press sends the act, by the Job's id, to the route Fleet answers.
    let menu = list.read_with(cx, |list, _| list.menu.as_ref().expect("the menu is open").menu.clone());
    menu.update(cx, |_, cx| cx.emit(Picked::Entry { id: "prioritise", keep: false }));
    let mut request = None;
    for _ in 0..50 {
        cx.run_until_parked();
        if let Ok(got) = sent.recv_timeout(std::time::Duration::from_millis(20)) {
            request = Some(got);
            break;
        }
    }
    let (path, body) = request.expect("the press reached Fleet");
    assert_eq!(path, "/sessions/prioritise");
    assert!(body.contains(&live), "the body names the Job: {body}");

    // Fleet's answer marks the Job: the row wears it and the menu offers to stop.
    let marked: Vec<SessionRecord> = all
        .iter()
        .cloned()
        .map(|mut job| {
            job.prioritised = job.id.as_str() == live;
            job
        })
        .collect();
    list.update(cx, |list, cx| list.replace_records(marked, cx));
    assert_eq!(list.read_with(cx, |list, _| list.rows().filter(|row| row.prioritised).map(|row| row.id.to_string()).collect::<Vec<_>>()), [live.clone()]);
    open(&list, &live, cx);
    assert_eq!(said(&list, cx).as_deref(), Some("Stop prioritising"));
}

#[gpui::test]
fn an_ended_job_has_no_row_menu_so_it_is_never_offered_prioritise(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init_app);
    let all = crate::mock::jobs("jobs-many").expect("the fixture decodes");
    let ended = all.iter().find(|job| job.state == ipc::SessionState::Ended).expect("an ended Job").id.as_str().to_string();
    let (list, cx) = cx.add_window_view(|_, cx| JobsList::mock(&all, cx));
    open(&list, &ended, cx);
    assert_eq!(said(&list, cx), None);
}
