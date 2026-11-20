//! The API Client tab against a stand-in Fleet on a port of its own: what the tab sends when a request is sent, saved or an
//! environment is kept, how it lists a Job's sends, and how it pages the runs. The stand-in writes down every call it gets, so a
//! test reads the body Fleet would have been given.

use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{AppContext as _, Context, Entity, TestAppContext, VisualTestContext, Window};
use ipc::{
    RequestBy, RequestCollection, RequestPlace, RequestRuns, RequestSaved, RequestSent, RequestToSend, Requests, RequestsNotRead, SaveRequestCollection,
    SaveRequestEnvironment, SaveRequestToCollection, SavedRequest, SavedRequestNamed, SendRequest, SessionId, VariableRead, VariableWrite,
};

use super::model::{self, Filter, Line};
use super::runs::RunsState;
use super::test_fixtures::{environment, header, run, standard};
use super::{ApiClient, Load, Open, Reply};
use crate::fleet::FleetConnection;
use crate::job_view::JobView;

const JOB: &str = "job-api";

/// Every call the stand-in got, in order: its method, its path with the query, and its body.
type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

/// A Fleet on a port of its own. `GET /requests` answers `read`; any other call is answered by `extra(method, path, body)`, or
/// is a 404 where that is none.
fn stand_in_fleet(read: Requests, extra: impl Fn(&str, &str, &str) -> Option<String> + Send + 'static) -> (u16, Seen) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("its address").port();
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let served = ipc::encode(&read).expect("the read encodes");
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
            let mut first = String::new();
            if reader.read_line(&mut first).is_err() {
                continue;
            }
            let mut length = 0usize;
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap_or(0) <= 2 {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                }
            }
            let mut body = vec![0u8; length];
            if reader.read_exact(&mut body).is_err() {
                continue;
            }
            let body = String::from_utf8_lossy(&body).to_string();
            let mut words = first.split_whitespace();
            let (method, path) = (words.next().unwrap_or_default().to_string(), words.next().unwrap_or_default().to_string());
            let answer = if method == "GET" && path.starts_with("/requests?") { Some(served.clone()) } else { extra(&method, &path, &body) };
            log.lock().expect("the log").push((method, path, body));
            let (status, text) = answer.map_or(("404 Not Found", "{}".to_string()), |text| ("200 OK", text));
            let mut stream = stream;
            let _ = write!(stream, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}", text.len());
        }
    });
    (port, seen)
}

/// The bodies posted to `path`, in order.
fn posted(seen: &Seen, path: &str) -> Vec<String> {
    seen.lock().expect("the log").iter().filter(|(method, at, _)| method == "POST" && at == path).map(|(_, _, body)| body.clone()).collect()
}

/// The paths asked with a GET that start with `prefix`, in order.
fn asked(seen: &Seen, prefix: &str) -> Vec<String> {
    seen.lock().expect("the log").iter().filter(|(method, at, _)| method == "GET" && at.starts_with(prefix)).map(|(_, at, _)| at.clone()).collect()
}

fn decoded<T: serde::de::DeserializeOwned>(what: &'static str, body: &str) -> T {
    ipc::decode(what, body.as_bytes()).unwrap_or_else(|why| panic!("{what} decodes: {why:?}"))
}

fn encoded<T: serde::Serialize>(value: &T) -> String {
    ipc::encode(value).expect("it encodes")
}

fn answered() -> String {
    encoded(&RequestSent { url: "http://localhost:4300/health".into(), status: Some(200), ms: 5, headers: Vec::new(), body: String::new(), truncated: false, error: None, note: None, saved: None })
}

/// The thread of a Job whose requests were sent by `sends`, with a message between them that is not a request.
fn thread_of(sends: &[(&str, RequestBy)]) -> ipc::JobThread {
    let mut items: Vec<String> = sends
        .iter()
        .map(|(id, by)| {
            let by = if *by == RequestBy::Agent { "agent" } else { "owner" };
            format!(r#"{{"kind":"row","row":{{"kind":"requested","id":"{id}","at":"2026-11-20T09:00:00Z","by":"{by}","method":"GET","url":"http://localhost/{id}","ms":10,"response":"","status":200}}}}"#)
        })
        .collect();
    items.insert(1.min(items.len()), r#"{"kind":"row","row":{"kind":"message","id":"m1","at":"2026-11-20T09:00:00Z","from":{"kind":"you"},"text":"hello"}}"#.to_string());
    let wire = format!(r#"{{"session_id":"{JOB}","head":{{"trail":{{"started":"2026-11-20T08:00:00Z"}}}},"items":[{}]}}"#, items.join(","));
    ipc::decode::<ipc::JobThread>("thread", wire.as_bytes()).expect("a thread")
}

/// An API Client on a stand-in Fleet that has read `read`, its Job's thread holding `sends`.
fn client<'a>(
    cx: &'a mut TestAppContext,
    read: Requests,
    sends: &[(&str, RequestBy)],
    extra: impl Fn(&str, &str, &str) -> Option<String> + Send + 'static,
) -> (Entity<ApiClient>, Seen, &'a mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
    });
    let (port, seen) = stand_in_fleet(read, extra);
    let fleet = gpui::AppContext::new(cx, |cx| FleetConnection::idle(cx));
    fleet.update(cx, |fleet, cx| fleet.connect_at(port, cx));
    let list = cx.new(|cx| crate::jobs::JobsList::mock(&[], cx));
    let threads = std::collections::HashMap::from([(JOB.to_string(), thread_of(sends))]);
    let (view, cx) = cx.add_window_view(move |window, cx| JobView::new(list, Some(threads), window, cx));
    let sent = view.read_with(cx, |view, cx| (view.requested_version(JOB, cx), view.requested(JOB, cx)));
    let tab = view.update_in(cx, |_, window, cx| {
        let view = cx.entity();
        cx.new(|cx| ApiClient::new(JOB.into(), view, Some(fleet), sent, window, cx))
    });
    assert!(settle(&tab, cx, |tab| matches!(tab.read, Load::Read(_))), "the stand-in's read lands");
    (tab, seen, cx)
}

/// Park until `done` holds of the tab, with the stand-in answering on its own thread.
fn settle(tab: &Entity<ApiClient>, cx: &mut VisualTestContext, done: impl Fn(&ApiClient) -> bool) -> bool {
    for _ in 0..150 {
        cx.run_until_parked();
        if tab.read_with(cx, |tab, _| done(tab)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn type_address(tab: &mut ApiClient, text: &str, window: &mut Window, cx: &mut Context<ApiClient>) {
    let address = tab.address.clone();
    address.update(cx, |input, cx| input.set_value(text.to_string(), window, cx));
    tab.address_typed(&address, window, cx);
}

fn lines(tab: &Entity<ApiClient>, cx: &mut VisualTestContext) -> Vec<Line> {
    tab.read_with(cx, |tab, _| tab.lines.clone())
}

// ---- the request line ---------------------------------------------------------------------------

#[gpui::test]
fn a_name_typed_in_the_address_finds_saved_requests_and_a_braced_name_becomes_one_token(cx: &mut TestAppContext) {
    let (tab, _, cx) = client(cx, standard(), &[], |_, _, _| None);
    tab.update_in(cx, |tab, window, cx| type_address(tab, "user", window, cx));
    assert_eq!(tab.read_with(cx, |tab, _| tab.found.clone()), vec![(0, 1), (0, 2)], "List users and Create user hold the typed name");
    tab.update_in(cx, |tab, window, cx| type_address(tab, "{{base}}/health", window, cx));
    assert!(tab.read_with(cx, |tab, _| tab.found.is_empty()), "an address finds no request by name");
    let tokens = tab.read_with(cx, |tab, cx| tab.address.read(cx).tokens().iter().map(|span| span.range()).collect::<Vec<_>>());
    assert_eq!(tokens, vec![0..8], "the {{{{base}}}} typed is one token of the address");
}

// ---- the list's chips ---------------------------------------------------------------------------

#[gpui::test]
fn a_jobs_sends_are_read_from_its_thread_by_who_sent_them_and_each_chip_lists_its_share(cx: &mut TestAppContext) {
    let sends = [("s1", RequestBy::Agent), ("s2", RequestBy::Owner), ("s3", RequestBy::Agent)];
    let (tab, _, cx) = client(cx, standard(), &sends, |_, _, _| None);
    let who = tab.read_with(cx, |tab, _| tab.sent.iter().map(|line| (line.id.clone(), line.who())).collect::<Vec<_>>());
    assert_eq!(who, [("s1".to_string(), "Agent"), ("s2".to_string(), "You"), ("s3".to_string(), "Agent")], "the message between them is no send");
    let sent_of = |lines: &[Line]| lines.iter().filter_map(|line| if let Line::Sent(at) = line { Some(*at) } else { None }).collect::<Vec<_>>();
    let saved_in = |lines: &[Line]| lines.iter().filter(|line| matches!(line, Line::Saved(..))).count();
    for (filter, shown_sends, shown_saved) in [(Filter::ThisJob, vec![2, 1, 0], 3), (Filter::Saved, vec![], 3), (Filter::Agent, vec![2, 0], 0), (Filter::Mine, vec![1], 0), (Filter::ThisJob, vec![2, 1, 0], 3)] {
        tab.update(cx, |tab, cx| tab.set_filter(filter, cx));
        let shown = lines(&tab, cx);
        assert_eq!((sent_of(&shown), saved_in(&shown)), (shown_sends, shown_saved), "{} holds", filter.word());
    }
}

// ---- the environment ----------------------------------------------------------------------------

#[gpui::test]
fn the_environment_key_cycles_none_then_each_environment_in_fleets_order_then_none_and_each_one_fills_the_address(cx: &mut TestAppContext) {
    let (tab, _, cx) = client(cx, standard(), &[], |_, _, _| None);
    tab.update_in(cx, |tab, window, cx| tab.open_saved(0, 0, window, cx));
    let step = |cx: &mut VisualTestContext| {
        tab.update(cx, |tab, cx| tab.cycle_environment(cx));
        tab.read_with(cx, |tab, cx| (tab.environment.clone(), model::filled(&tab.address_text(cx), tab.environment_read())))
    };
    assert_eq!(tab.read_with(cx, |tab, _| tab.environment.clone()), None, "a tab starts with no environment");
    let (one, filled) = step(cx);
    assert_eq!((one.as_deref(), filled.text.as_str()), (Some("server:api"), "http://localhost:4300/health"), "the Job's server fills {{{{base}}}}");
    let (two, filled) = step(cx);
    assert_eq!((two.as_deref(), filled.text.as_str()), (Some("staging"), "https://staging.example.test/health"));
    let (none, filled) = step(cx);
    assert_eq!((none, filled.text.as_str(), filled.missing), (None, "{{base}}/health", vec!["base".to_string()]), "past the last, none again");
    let (again, _) = step(cx);
    assert_eq!(again.as_deref(), Some("server:api"), "and round");
}

// ---- save ---------------------------------------------------------------------------------------

/// What the stand-in Fleet answers a save of a collection with: the read, the collection replaced.
fn read_with_collection(kept: RequestCollection) -> Requests {
    let mut read = standard();
    read.collections[0].item = kept;
    read
}

#[gpui::test]
fn an_open_saved_request_saves_its_collection_whole_with_that_request_replaced(cx: &mut TestAppContext) {
    let (tab, seen, cx) = client(cx, standard(), &[], |method, path, body| {
        (method == "POST" && path == "/requests/collections").then(|| encoded(&read_with_collection(decoded::<SaveRequestCollection>("the save", body).collection)))
    });
    tab.update_in(cx, |tab, window, cx| {
        tab.open_saved(0, 1, window, cx);
        tab.method = "PUT".into();
        type_address(tab, "{{base}}/users?limit=50", window, cx);
        tab.body.update(cx, |input, cx| input.set_value("{\"x\":1}".to_string(), window, cx));
        let (name, value) = (tab.headers[0].name.clone(), tab.headers[0].value.clone());
        name.update(cx, |input, cx| input.set_value("X-Trace".to_string(), window, cx));
        value.update(cx, |input, cx| input.set_value("abc".to_string(), window, cx));
        tab.save_pressed(window, cx);
    });
    assert!(settle(&tab, cx, |tab| tab.save_task.is_none() && tab.saved().is_some_and(|saved| saved.method == "PUT")), "the save is answered and the read it gave is drawn");
    let bodies = posted(&seen, "/requests/collections");
    assert_eq!(bodies.len(), 1, "one save, to the collection route");
    let save = decoded::<SaveRequestCollection>("the save", &bodies[0]);
    assert_eq!((save.repository, save.session_id), (None, Some(SessionId::carried(JOB))), "the Job's own, the repository left to Fleet");
    let original = standard().collections[0].item.clone();
    assert_eq!((save.collection.id.as_str(), save.collection.name.as_str(), &save.collection.headers), ("service", "Service", &original.headers), "the collection's own id, name and shared headers go back");
    assert_eq!(save.collection.requests.len(), 3);
    assert_eq!(save.collection.requests[0], original.requests[0], "a request not open is as it was");
    assert_eq!(save.collection.requests[2], original.requests[2]);
    assert_eq!(
        save.collection.requests[1],
        SavedRequest {
            id: "list-users".into(),
            name: "List users".into(),
            method: "PUT".into(),
            url: "{{base}}/users?limit=50".into(),
            headers: vec![header("X-Trace", "abc")],
            body: Some("{\"x\":1}".into()),
        },
        "the open one is what the editor holds, under its own id and name"
    );
}

#[gpui::test]
fn a_new_request_saves_under_the_name_and_the_collection_asked_for(cx: &mut TestAppContext) {
    let (tab, seen, cx) = client(cx, standard(), &[], |method, path, _| {
        (method == "POST" && path == "/requests/collections/request").then(|| encoded(&RequestSaved { collection_id: "service".into(), request_id: "create-order".into() }))
    });
    tab.update_in(cx, |tab, window, cx| {
        tab.method = "POST".into();
        type_address(tab, "{{base}}/orders", window, cx);
        tab.body.update(cx, |input, cx| input.set_value("{\"item\":1}".to_string(), window, cx));
        tab.save_pressed(window, cx);
    });
    assert!(tab.read_with(cx, |tab, _| tab.saving.is_some()), "a request that is not saved asks for a name and a collection first");
    assert_eq!(tab.read_with(cx, |tab, _| tab.saving.as_ref().and_then(|row| row.collection.clone())), Some("service".into()), "the person's own collection is the one it offers");
    tab.update_in(cx, |tab, window, cx| tab.save_pressed(window, cx));
    cx.run_until_parked();
    assert!(posted(&seen, "/requests/collections/request").is_empty(), "no name yet, so nothing is saved");
    tab.update_in(cx, |tab, window, cx| {
        let name = tab.saving.as_ref().expect("the row").name.clone();
        name.update(cx, |input, cx| input.set_value("Create order".to_string(), window, cx));
        tab.save_pressed(window, cx);
    });
    assert!(settle(&tab, cx, |tab| tab.open == Open::Saved("service".into(), "create-order".into())), "the saved request is the one open once Fleet has answered");
    let bodies = posted(&seen, "/requests/collections/request");
    assert_eq!(bodies.len(), 1);
    let save = decoded::<SaveRequestToCollection>("the save", &bodies[0]);
    assert_eq!(save.session_id, Some(SessionId::carried(JOB)));
    assert_eq!(save.collection, "service");
    assert_eq!(save.request.name, "Create order");
    assert_eq!((save.request.method.as_str(), save.request.url.as_str()), ("POST", "{{base}}/orders"));
    assert_eq!(save.request.body.as_deref(), Some("{\"item\":1}"));
    assert!(save.request.headers.is_empty());
}

fn unread(place: RequestPlace) -> Requests {
    let mut read = standard();
    read.not_read.push(RequestsNotRead { place, why: "expected `=` after a key at line 3".into() });
    read
}

#[gpui::test]
fn no_save_is_offered_while_the_persons_own_file_would_not_read(cx: &mut TestAppContext) {
    let (tab, seen, cx) = client(cx, unread(RequestPlace::Yours), &[], |_, _, _| None);
    tab.update_in(cx, |tab, window, cx| {
        tab.open_saved(0, 0, window, cx);
        type_address(tab, "{{base}}/health?x=1", window, cx);
        tab.save_pressed(window, cx);
    });
    assert!(tab.read_with(cx, |tab, cx| tab.save_off(cx)).is_some_and(|why| why.contains("would not read")), "the control says why");
    assert!(tab.read_with(cx, |tab, _| tab.save_task.is_none()), "an open saved request is not saved");
    tab.update_in(cx, |tab, window, cx| {
        tab.open = Open::Blank;
        tab.save_pressed(window, cx);
    });
    assert!(tab.read_with(cx, |tab, _| tab.saving.is_none() && tab.save_task.is_none()), "a new request is not even asked for a name");
    assert!(posted(&seen, "/requests/collections").is_empty() && posted(&seen, "/requests/collections/request").is_empty());
}

#[gpui::test]
fn a_repository_file_that_would_not_read_leaves_saving_on(cx: &mut TestAppContext) {
    let (tab, seen, cx) = client(cx, unread(RequestPlace::Repository), &[], |method, path, body| {
        (method == "POST" && path == "/requests/collections").then(|| encoded(&read_with_collection(decoded::<SaveRequestCollection>("the save", body).collection)))
    });
    tab.update_in(cx, |tab, window, cx| {
        tab.open_saved(0, 0, window, cx);
        type_address(tab, "{{base}}/health?x=1", window, cx);
        tab.save_pressed(window, cx);
    });
    assert!(settle(&tab, cx, |_| !posted(&seen, "/requests/collections").is_empty()), "Fleet writes only the person's own file, so the save goes");
}

// ---- send as a run ------------------------------------------------------------------------------

/// What the tab sends after `edit` is made to a request opened from the saved ones (the first, Health), or to a blank one where
/// `saved` is false.
fn sent_after(cx: &mut TestAppContext, saved: bool, edit: impl FnOnce(&mut ApiClient, &mut Window, &mut Context<ApiClient>)) -> SendRequest {
    let (tab, seen, cx) = client(cx, standard(), &[], |method, path, _| (method == "POST" && path == "/requests/send").then(answered));
    tab.update_in(cx, |tab, window, cx| {
        if saved {
            tab.open_saved(0, 0, window, cx);
        } else {
            type_address(tab, "{{base}}/health", window, cx);
        }
        edit(tab, window, cx);
        tab.send(window, cx);
    });
    assert!(settle(&tab, cx, |tab| matches!(tab.reply, Some(Reply::Answered(_)))), "Fleet's answer to the send lands");
    let bodies = posted(&seen, "/requests/send");
    assert_eq!(bodies.len(), 1, "one send");
    decoded::<SendRequest>("the send", &bodies[0])
}

#[gpui::test]
fn a_saved_request_sent_with_its_method_and_address_unchanged_is_sent_as_that_saved_request(cx: &mut TestAppContext) {
    let sent = sent_after(cx, true, |_, _, _| {});
    assert_eq!(sent.request, RequestToSend::Saved(SavedRequestNamed { collection: "service".into(), request: "health".into() }));
    assert_eq!(sent.session_id, Some(SessionId::carried(JOB)));
}

#[gpui::test]
fn what_was_edited_in_the_headers_and_body_still_goes_with_a_saved_send(cx: &mut TestAppContext) {
    let sent = sent_after(cx, true, |tab, window, cx| {
        tab.body.update(cx, |input, cx| input.set_value("{\"a\":1}".to_string(), window, cx));
        let (name, value) = (tab.headers[0].name.clone(), tab.headers[0].value.clone());
        name.update(cx, |input, cx| input.set_value("X-Trace".to_string(), window, cx));
        value.update(cx, |input, cx| input.set_value("on".to_string(), window, cx));
    });
    assert!(matches!(sent.request, RequestToSend::Saved(_)), "still its run");
    assert_eq!((sent.body.as_deref(), sent.headers), (Some("{\"a\":1}"), vec![header("X-Trace", "on")]));
}

#[gpui::test]
fn a_changed_method_sends_the_request_as_its_own_with_the_collection_it_came_from(cx: &mut TestAppContext) {
    let sent = sent_after(cx, true, |tab, _, _| tab.method = "POST".into());
    assert_eq!(sent.request, RequestToSend::Own { method: "POST".into(), url: "{{base}}/health".into(), collection: Some("service".into()) });
}

#[gpui::test]
fn a_changed_address_sends_the_request_as_its_own(cx: &mut TestAppContext) {
    let sent = sent_after(cx, true, |tab, window, cx| type_address(tab, "{{base}}/health?deep=1", window, cx));
    assert_eq!(sent.request, RequestToSend::Own { method: "GET".into(), url: "{{base}}/health?deep=1".into(), collection: Some("service".into()) });
}

#[gpui::test]
fn a_request_that_is_not_saved_is_sent_as_its_own_with_no_collection(cx: &mut TestAppContext) {
    let sent = sent_after(cx, false, |_, _, _| {});
    assert_eq!(sent.request, RequestToSend::Own { method: "GET".into(), url: "{{base}}/health".into(), collection: None });
}

// ---- secrets ------------------------------------------------------------------------------------

const TYPED: &str = "s3cr3t-value-7f2a";

/// What Fleet would answer a save of an environment with: the read, the environment in it, a secret that was sent or already set
/// read as set and no value anywhere.
fn read_with_environment(kept: ipc::RequestEnvironmentWrite) -> Requests {
    let mut read = standard();
    let before: Vec<VariableRead> = read.environments.iter().find(|one| one.item.name == kept.name).map(|one| one.item.variables.clone()).unwrap_or_default();
    let variables = kept
        .variables
        .into_iter()
        .map(|variable| match variable {
            VariableWrite::Plain { name, value } => VariableRead::Plain { name, value },
            VariableWrite::Secret { name, value } => {
                let was = before.iter().any(|one| matches!(one, VariableRead::Secret { name: other, set: true } if *other == name));
                VariableRead::Secret { name, set: value.is_some() || was }
            }
        })
        .collect();
    let placed = environment(&kept.name, variables, RequestPlace::Yours);
    match read.environments.iter().position(|one| one.item.name == kept.name) {
        Some(at) => read.environments[at] = placed,
        None => read.environments.push(placed),
    }
    read
}

fn environment_stand_in() -> impl Fn(&str, &str, &str) -> Option<String> + Send + 'static {
    |method, path, body| (method == "POST" && path == "/requests/environments").then(|| encoded(&read_with_environment(decoded::<SaveRequestEnvironment>("the save", body).environment)))
}

/// Whether `needle` is anywhere a person or a log could find it in the tab: every field of text, every form, what Fleet last
/// read and the words of a refusal.
fn holds(tab: &ApiClient, needle: &str, cx: &gpui::App) -> bool {
    let mut texts = vec![tab.address_text(cx), tab.body_text(cx), tab.save_said.clone().unwrap_or_default(), format!("{:?}", tab.requests())];
    for row in &tab.headers {
        texts.push(row.name.read(cx).value().to_string());
        texts.push(row.value.read(cx).value().to_string());
    }
    for form in tab.forms.iter().chain(tab.adding.as_ref()) {
        texts.push(form.name.read(cx).value().to_string());
        texts.push(form.refused.clone().unwrap_or_default());
        for row in &form.rows {
            texts.push(row.name.read(cx).value().to_string());
            texts.push(row.value.read(cx).value().to_string());
        }
    }
    texts.iter().any(|text| text.contains(needle))
}

#[gpui::test]
fn a_typed_secret_goes_in_the_save_once_and_is_nowhere_in_the_tab_once_the_body_is_built(cx: &mut TestAppContext) {
    let (tab, seen, cx) = client(cx, standard(), &[], environment_stand_in());
    tab.update_in(cx, |tab, window, cx| tab.add_environment(window, cx));
    let held_before_and_after = tab.update_in(cx, |tab, window, cx| {
        let form = tab.adding.as_mut().expect("the environment being added");
        let (id, name, variable, value) = (form.id, form.name.clone(), form.rows[0].name.clone(), form.rows[0].value.clone());
        form.rows[0].secret = true;
        name.update(cx, |input, cx| input.set_value("prod".to_string(), window, cx));
        variable.update(cx, |input, cx| input.set_value("token".to_string(), window, cx));
        value.update(cx, |input, cx| input.set_value(TYPED.to_string(), window, cx));
        let before = holds(tab, TYPED, cx);
        tab.save_environment(id, window, cx);
        (before, holds(tab, TYPED, cx))
    });
    assert_eq!(held_before_and_after, (true, false), "typed in a field before the save, in no field of the tab the moment the call is made");
    assert!(settle(&tab, cx, |tab| tab.adding.is_none() && !posted(&seen, "/requests/environments").is_empty()), "Fleet answers the save");
    let bodies = posted(&seen, "/requests/environments");
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0].matches(TYPED).count(), 1, "the value is in the one body to Fleet, once");
    let save = decoded::<SaveRequestEnvironment>("the save", &bodies[0]);
    assert_eq!(save.environment.name, "prod");
    assert_eq!(save.environment.variables, vec![VariableWrite::Secret { name: "token".into(), value: Some(TYPED.into()) }], "a secret typed goes with its value; the blank row goes nowhere");
    assert!(!tab.read_with(cx, |tab, cx| holds(tab, TYPED, cx)), "and the answer that landed holds no value either");
    let (read, row) = tab.read_with(cx, |tab, cx| {
        let read = tab.environments().iter().find(|one| one.item.name == "prod").map(|one| one.item.variables.clone());
        let form = tab.forms.iter().find(|form| form.read.as_ref().is_some_and(|read| read.item.name == "prod"));
        (read, form.and_then(|form| form.rows.first()).map(|row| (row.secret, row.set, row.value.read(cx).value().to_string())))
    });
    assert_eq!(read, Some(vec![VariableRead::Secret { name: "token".into(), set: true }]), "Fleet reads it back as set, with no value");
    assert_eq!(row, Some((true, true, String::new())), "its row is a secret that is set and shows no value");
}

#[gpui::test]
fn a_secret_already_kept_reads_as_set_with_no_value_and_a_save_without_typing_keeps_it(cx: &mut TestAppContext) {
    let (tab, seen, cx) = client(cx, standard(), &[], environment_stand_in());
    let row = tab.read_with(cx, |tab, cx| {
        let form = tab.forms.iter().find(|form| form.read.as_ref().is_some_and(|read| read.item.name == "staging")).expect("the person's own staging has a form");
        form.rows.iter().find(|row| row.secret).map(|row| (row.set, row.replacing, row.value.read(cx).value().to_string()))
    });
    assert_eq!(row, Some((true, false, String::new())), "Set, and no value to show");
    tab.update_in(cx, |tab, window, cx| {
        let id = tab.forms.iter().find(|form| form.read.as_ref().is_some_and(|read| read.item.name == "staging")).map(|form| form.id).expect("the form");
        tab.save_environment(id, window, cx);
    });
    assert!(settle(&tab, cx, |_| !posted(&seen, "/requests/environments").is_empty()), "the save is made");
    let save = decoded::<SaveRequestEnvironment>("the save", &posted(&seen, "/requests/environments")[0]);
    assert_eq!(
        save.environment.variables,
        vec![VariableWrite::Plain { name: "base".into(), value: "https://staging.example.test".into() }, VariableWrite::Secret { name: "token".into(), value: None }],
        "no value for a secret nobody replaced, so Fleet keeps what it has"
    );
}

// ---- runs ---------------------------------------------------------------------------------------

#[gpui::test]
fn a_saved_requests_runs_are_listed_a_page_at_a_time_by_who_sent_them_and_next_asks_for_the_page_after(cx: &mut TestAppContext) {
    let this_job = Some((JOB, Some("This Job")));
    let first = RequestRuns {
        runs: vec![
            run("r1", RequestBy::Owner, this_job, Some(200)),
            run("r2", RequestBy::Agent, this_job, Some(503)),
            run("r3", RequestBy::Owner, Some(("other-job", Some("Add rate limits"))), None),
        ],
        next: Some("r3".into()),
    };
    let second = RequestRuns { runs: vec![run("r4", RequestBy::Agent, None, Some(200)), run("r5", RequestBy::Owner, None, Some(404))], next: None };
    let (tab, seen, cx) = client(cx, standard(), &[], move |method, path, _| {
        if method != "GET" || !path.starts_with("/requests/runs?") {
            return None;
        }
        Some(encoded(if path.contains("&before=r3") { &second } else { &first }))
    });
    tab.update_in(cx, |tab, window, cx| tab.open_saved(0, 0, window, cx));
    assert!(settle(&tab, cx, |tab| tab.runs.as_ref().is_some_and(|runs| matches!(runs.state, RunsState::Ready))), "the first page lands");
    let rows = |tab: &ApiClient| {
        let runs = tab.runs.as_ref().expect("the runs");
        runs.runs.iter().map(|one| (one.id.clone(), model::run_who(one, JOB), model::outcome_of(one.status))).collect::<Vec<_>>()
    };
    use model::Outcome::{Answered, Failed, None as NoAnswer};
    assert_eq!(
        tab.read_with(cx, |tab, _| rows(tab)),
        [("r1".to_string(), "You".to_string(), Answered), ("r2".into(), "Agent".into(), Failed), ("r3".into(), "Add rate limits".into(), NoAnswer)],
        "status and who: You, the Job's agent, another Job by its title"
    );
    assert_eq!(tab.read_with(cx, |tab, _| tab.runs.as_ref().and_then(|runs| runs.next.clone())), Some("r3".into()), "Fleet says where the next page begins");
    assert_eq!(&lines(&tab, cx)[..5], [Line::Collection(0), Line::Saved(0, 0), Line::Run(0), Line::Run(1), Line::Run(2)], "the runs stand under the request they belong to");
    tab.update_in(cx, |tab, window, cx| tab.want_more_runs(window, cx));
    assert!(settle(&tab, cx, |tab| tab.runs.as_ref().is_some_and(|runs| runs.runs.len() == 5 && matches!(runs.state, RunsState::Ready))), "the second page follows the first");
    assert_eq!(tab.read_with(cx, |tab, _| rows(tab).iter().map(|row| row.0.clone()).collect::<Vec<_>>()), ["r1", "r2", "r3", "r4", "r5"], "newest first, the page after the one drawn");
    assert_eq!(tab.read_with(cx, |tab, _| tab.runs.as_ref().and_then(|runs| runs.next.clone())), None, "the last page has no next");
    let pages = asked(&seen, "/requests/runs?");
    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains("collection=service&request=health&limit=20") && !pages[0].contains("before"), "the first page is asked from the top: {}", pages[0]);
    assert!(pages[1].ends_with("&before=r3"), "the second begins after the last run of the first: {}", pages[1]);
    tab.update_in(cx, |tab, window, cx| tab.want_more_runs(window, cx));
    cx.run_until_parked();
    assert_eq!(asked(&seen, "/requests/runs?").len(), 2, "nothing more is asked once there is no next");
}
