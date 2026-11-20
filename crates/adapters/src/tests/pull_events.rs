//! Pull request events from recorded streams and refusals, and against a socket on this machine.
//! Nothing here reaches the forge, which answers 403 to the owner's account anyway.

use std::time::Duration;

use adapter_traits::{PullEvents, PullEventsFailure, PullStreamAction, PullStreamEnd, PullStreamSignal};

use super::github_api::{api, serve, Reply};
use super::repo::TempRepo;
use crate::github_api::{ApiFailure, RateLimit};
use crate::pull_events::frames::{Frame, Frames, Outcome};
use crate::pull_events::{failure_of, left, ApiPullEvents};

/// A stream as the forge sends it: a heartbeat, two events, a control frame, the end.
const RECORDED: &str = ": heartbeat\n\
id: 1\n\
event: com.github.pull_request.head.changed.v1\n\
data: {\"type\":\"com.github.pull_request.head.changed.v1\",\"data\":{\"api_url\":\"https://api.github.com/repos/o/r/pulls/7\"}}\n\
\n\
: heartbeat\n\
\n\
id: 2\n\
event: com.github.pull_request.check_suite.status.changed.v1\n\
data: {\"data\":{\"api_url\":\"https://api.github.com/repos/o/r/pulls/7\",\"conclusion\":\"success\"}}\n\
\n\
id: 3\n\
event: github.stream.start\n\
data: {}\n\
\n\
id: 4\n\
event: github.stream.end\n\
data: {\"data\":{\"outcome\":\"subscription_ended\"}}\n\
\n";

fn head_changed() -> Frame {
    Frame::Event {
        kind: "com.github.pull_request.head.changed.v1".to_string(),
        api_url: Some("https://api.github.com/repos/o/r/pulls/7".to_string()),
    }
}

fn frames_of(stream: &str, piece: usize) -> Vec<Frame> {
    let mut frames = Frames::default();
    stream.as_bytes().chunks(piece).flat_map(|bytes| frames.feed(bytes)).collect()
}

#[test]
fn a_recorded_stream_is_its_events_and_its_end() {
    let frames = frames_of(RECORDED, RECORDED.len());
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0], head_changed());
    assert!(matches!(&frames[1], Frame::Event { kind, .. } if kind.ends_with("check_suite.status.changed.v1")));
    assert_eq!(frames[2], Frame::End(Outcome::SubscriptionEnded));
}

#[test]
fn a_stream_reads_the_same_in_any_pieces() {
    let whole = frames_of(RECORDED, RECORDED.len());
    for piece in [1, 2, 7, 64] {
        assert_eq!(frames_of(RECORDED, piece), whole, "{piece} bytes at a time");
    }
}

#[test]
fn a_heartbeat_says_nothing() {
    assert!(frames_of(": heartbeat\n\n: heartbeat\n\n", 4096).is_empty());
}

#[test]
fn lines_ended_with_carriage_returns_read_the_same() {
    let crlf = RECORDED.replace('\n', "\r\n");
    assert_eq!(frames_of(&crlf, 4096), frames_of(RECORDED, 4096));
}

#[test]
fn an_event_whose_data_will_not_read_is_still_a_change() {
    let frames = frames_of("event: com.github.pull_request.state.changed.v1\ndata: not json\n\n", 4096);
    assert_eq!(
        frames,
        vec![Frame::Event { kind: "com.github.pull_request.state.changed.v1".to_string(), api_url: None }]
    );
}

#[test]
fn the_end_says_whether_to_reconnect() {
    let said = |outcome: &str| frames_of(&format!("event: github.stream.end\ndata: {{\"data\":{{\"outcome\":\"{outcome}\"}}}}\n\n"), 4096);
    assert_eq!(said("reconnect"), vec![Frame::End(Outcome::Reconnect)]);
    assert_eq!(said("subscription_ended"), vec![Frame::End(Outcome::SubscriptionEnded)]);
    assert_eq!(said("later"), vec![Frame::End(Outcome::Unknown("later".to_string()))]);
}

fn refused(status: u16, said: &str, limit: Option<RateLimit>) -> ApiFailure {
    ApiFailure::Refused { status, said: said.to_string(), limit }
}

#[test]
fn a_403_that_says_events_are_off_is_told_apart_from_any_other() {
    let off = refused(403, "Event subscriptions are not enabled for this account", None);
    assert_eq!(failure_of(&off), PullEventsFailure::NotEnabled);
    assert_eq!(failure_of(&refused(403, "Resource not accessible by integration", None)), PullEventsFailure::Forbidden);
}

#[test]
fn each_refusal_is_the_kind_a_caller_acts_on() {
    let wait = Some(RateLimit { retry_after: Some(42), ..RateLimit::default() });
    let cases = [
        (refused(401, "Bad credentials", None), PullEventsFailure::Unauthorized),
        (refused(404, "Not Found", None), PullEventsFailure::Forbidden),
        (refused(422, "Subscription limit exceeded", None), PullEventsFailure::LimitReached),
        (refused(500, "", None), PullEventsFailure::Transient),
        (refused(504, "", None), PullEventsFailure::Transient),
        (refused(503, "", wait), PullEventsFailure::RetryAfter(Duration::from_secs(42))),
        (refused(429, "", wait), PullEventsFailure::RetryAfter(Duration::from_secs(42))),
        (ApiFailure::Unreachable("timed out".to_string()), PullEventsFailure::Transient),
        (ApiFailure::NoToken("none".to_string()), PullEventsFailure::Unauthorized),
        (ApiFailure::HeldBack { seconds: 9 }, PullEventsFailure::RetryAfter(Duration::from_secs(9))),
    ];
    for (failure, kind) in cases {
        assert_eq!(failure_of(&failure), kind, "{failure}");
    }
    assert!(matches!(failure_of(&refused(418, "teapot", None)), PullEventsFailure::Other(_)));
}

#[test]
fn what_a_subscription_has_left_is_counted_from_now() {
    let now = 1_700_000_000_000_i64;
    // `now` is 2023-11-14T22:13:20Z.
    assert_eq!(left(Some("2023-11-14T23:13:20Z".to_string()), now), Duration::from_secs(3600));
    assert_eq!(left(Some("2023-11-14T21:13:20Z".to_string()), now), Duration::ZERO);
    assert_eq!(left(Some("soon".to_string()), now), Duration::from_secs(86_400));
    assert_eq!(left(None, now), Duration::from_secs(86_400));
}

fn on_the_forge() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.git(&["remote", "add", "origin", "https://github.com/Mele-Labs/armada.git"]);
    repo
}

fn sse(body: &str) -> Reply {
    Reply { status: 200, headers: vec![("content-type", "text/event-stream".to_string())], body: body.to_string() }
}

#[test]
fn subscribing_names_the_pull_request_and_all_five_event_types() {
    let server = serve(|_| Reply {
        status: 201,
        headers: Vec::new(),
        body: r#"{"id":"s1","resource_url":"x","event_types":[],"created_at":"2023-11-14T00:00:00Z","expires_at":"2099-01-01T00:00:00Z","url":"u","events_url":"https://api.github.com/event-subscriptions/s1/events"}"#.to_string(),
    });
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    let repo = on_the_forge();
    let made = events.subscribe(&repo.root_str(), 7).unwrap();
    assert_eq!(made.id, "s1");
    assert_eq!(made.events_url, "https://api.github.com/event-subscriptions/s1/events");
    assert!(made.left > Duration::from_secs(3600));
    let seen = server.seen();
    assert_eq!((seen[0].method.as_str(), seen[0].target.as_str()), ("POST", "/event-subscriptions"));
    assert!(seen[0].body.contains(r#""resource_url":"https://github.com/mele-labs/armada/pull/7""#), "{}", seen[0].body);
    for kind in [
        "review.submitted.v1",
        "state.changed.v1",
        "head.changed.v1",
        "merge_conflict.v1",
        "check_suite.status.changed.v1",
    ] {
        assert!(seen[0].body.contains(&format!("com.github.pull_request.{kind}")), "{kind}");
    }
}

#[test]
fn an_account_with_events_off_is_told_so_by_the_probe_and_by_subscribing() {
    let server = serve(|_| Reply {
        status: 403,
        headers: Vec::new(),
        body: r#"{"message":"Event subscriptions are not enabled for this account"}"#.to_string(),
    });
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    let repo = on_the_forge();
    assert_eq!(events.probe(), Err(PullEventsFailure::NotEnabled));
    assert_eq!(events.subscribe(&repo.root_str(), 7).unwrap_err(), PullEventsFailure::NotEnabled);
}

#[test]
fn a_repository_off_the_forge_is_not_subscribed() {
    let server = serve(|_| panic!("nothing is asked"));
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    let repo = TempRepo::with_a_commit();
    repo.git(&["remote", "add", "origin", "https://example.invalid/o/r.git"]);
    assert_eq!(events.subscribe(&repo.root_str(), 7).unwrap_err(), PullEventsFailure::NotOnThisForge);
}

#[test]
fn unsubscribing_a_subscription_already_gone_is_done() {
    let server = serve(|_| Reply { status: 404, headers: Vec::new(), body: r#"{"message":"Not Found"}"#.to_string() });
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    assert_eq!(events.unsubscribe("s1"), Ok(()));
    assert_eq!(server.seen()[0].target, "/event-subscriptions/s1");
}

/// What the reader of a stream was told, and what the stream came to.
fn listened(events: &ApiPullEvents, url: &str) -> (Vec<PullStreamSignal>, PullStreamEnd) {
    let mut told = Vec::new();
    let end = events.listen(url, &mut |signal| {
        told.push(signal);
        PullStreamAction::Keep
    });
    (told, end)
}

#[test]
fn a_recorded_stream_attaches_changes_and_ends() {
    let server = serve(|_| sse(RECORDED));
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    let (told, end) = listened(&events, &format!("{}/event-subscriptions/s1/events", server.url));
    let changes = told.iter().filter(|signal| **signal == PullStreamSignal::Changed).count();
    assert_eq!(told.first(), Some(&PullStreamSignal::Attached));
    assert_eq!(changes, 2);
    assert_eq!(end, PullStreamEnd::SubscriptionEnded);
    let seen = &server.seen()[0];
    assert_eq!(seen.accept.as_deref(), Some("text/event-stream"));
    assert_eq!(seen.authorization.as_deref(), Some("Bearer t"));
}

#[test]
fn a_stream_the_forge_asks_to_reopen_is_a_reconnect() {
    let body = "event: github.stream.end\ndata: {\"data\":{\"outcome\":\"reconnect\"}}\n\n";
    let server = serve(move |_| sse(body));
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    assert_eq!(listened(&events, &format!("{}/s/events", server.url)).1, PullStreamEnd::Reconnect);
}

#[test]
fn a_stream_that_stops_without_an_end_was_dropped() {
    let server = serve(|_| sse("event: com.github.pull_request.head.changed.v1\ndata: {}\n\n"));
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    assert_eq!(listened(&events, &format!("{}/s/events", server.url)).1, PullStreamEnd::Dropped);
}

#[test]
fn a_reader_that_stops_ends_the_stream() {
    let server = serve(|_| sse(RECORDED));
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    let end = events.listen(&format!("{}/s/events", server.url), &mut |_| PullStreamAction::Stop);
    assert_eq!(end, PullStreamEnd::Stopped);
}

#[test]
fn a_stream_the_forge_refuses_is_a_failure_not_an_end() {
    let server = serve(|_| Reply {
        status: 403,
        headers: Vec::new(),
        body: r#"{"message":"Event subscriptions are not enabled for this account"}"#.to_string(),
    });
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    let (told, end) = listened(&events, &format!("{}/s/events", server.url));
    assert!(told.is_empty());
    assert_eq!(end, PullStreamEnd::Failed(PullEventsFailure::NotEnabled));
}

#[test]
fn a_stream_outside_the_apis_own_address_is_never_opened_with_the_token() {
    let server = serve(|_| panic!("the token must not go anywhere else"));
    let events = ApiPullEvents::through(api(&server.url, &["t"]).0);
    let (_, end) = listened(&events, "https://elsewhere.invalid/s/events");
    assert!(matches!(end, PullStreamEnd::Failed(PullEventsFailure::Other(_))), "{end:?}");
    assert!(server.seen().is_empty());
}

#[test]
fn a_stream_asked_with_a_refused_token_is_asked_again_with_a_fresh_one() {
    let server = serve(|seen| match seen.authorization.as_deref() {
        Some("Bearer old") => Reply { status: 401, headers: Vec::new(), body: r#"{"message":"Bad credentials"}"#.to_string() },
        _ => sse(RECORDED),
    });
    let events = ApiPullEvents::through(api(&server.url, &["old", "new"]).0);
    let (_, end) = listened(&events, &format!("{}/s/events", server.url));
    assert_eq!(end, PullStreamEnd::SubscriptionEnded);
    assert_eq!(server.seen().len(), 2);
}
