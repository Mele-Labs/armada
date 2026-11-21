//! What the API Client's tests read from, built by hand so no test depends on the walk's mock data.

use ipc::{
    Instant, Placed, RequestBy, RequestCollection, RequestEnvironmentRead, RequestHeader, RequestPlace, RequestRunJob, RequestRunSummary, Requests, RunPlace, SavedRequest, SessionId, VariableRead,
};

pub fn header(name: &str, value: &str) -> RequestHeader {
    RequestHeader { name: name.into(), value: value.into() }
}

pub fn saved(id: &str, name: &str, method: &str, url: &str) -> SavedRequest {
    SavedRequest { id: id.into(), name: name.into(), method: method.into(), url: url.into(), headers: Vec::new(), body: None }
}

pub fn collection(id: &str, name: &str, headers: Vec<RequestHeader>, requests: Vec<SavedRequest>, place: RequestPlace) -> Placed<RequestCollection> {
    Placed { item: RequestCollection { id: id.into(), name: name.into(), headers, requests }, place, hides: false }
}

pub fn plain(name: &str, value: &str) -> VariableRead {
    VariableRead::Plain { name: name.into(), value: value.into() }
}

pub fn secret(name: &str, set: bool) -> VariableRead {
    VariableRead::Secret { name: name.into(), set }
}

pub fn environment(name: &str, variables: Vec<VariableRead>, place: RequestPlace) -> Placed<RequestEnvironmentRead> {
    Placed { item: RequestEnvironmentRead { name: name.into(), variables }, place, hides: false }
}

pub fn requests(collections: Vec<Placed<RequestCollection>>, environments: Vec<Placed<RequestEnvironmentRead>>) -> Requests {
    Requests { repository: "/repo".into(), place: RunPlace::ThisMac, collections, environments, not_read: Vec::new() }
}

/// One of the person's own collections with three requests, a server's environment and the person's `staging`, whose `token` is
/// a secret the Keychain holds.
pub fn standard() -> Requests {
    requests(
        vec![collection(
            "service",
            "Service",
            vec![header("Accept", "application/json")],
            vec![
                saved("health", "Health", "GET", "{{base}}/health"),
                saved("list-users", "List users", "GET", "{{base}}/users?limit=20"),
                SavedRequest { body: Some("{\"name\": \"Ada\"}".into()), ..saved("make-user", "Create user", "POST", "{{base}}/users") },
            ],
            RequestPlace::Yours,
        )],
        vec![
            environment("server:api", vec![plain("base", "http://localhost:4300")], RequestPlace::Server),
            environment("staging", vec![plain("base", "https://staging.example.test"), secret("token", true)], RequestPlace::Yours),
        ],
    )
}

/// A run of a request as Fleet lists it: by whom, in which Job (with that Job's title where it has one), and how it answered.
pub fn run(id: &str, by: RequestBy, job: Option<(&str, Option<&str>)>, status: Option<u16>) -> RequestRunSummary {
    RequestRunSummary {
        id: id.into(),
        at: Instant::carried("2026-11-20T09:00:00Z".to_string()),
        by,
        job: job.map(|(job, title)| RequestRunJob { id: SessionId::carried(job), title: title.map(str::to_string) }),
        method: "GET".into(),
        url: "http://localhost:4300/health".into(),
        status,
        ms: 20,
        error: status.is_none().then(|| "connection refused".to_string()),
        summary: String::new(),
    }
}
