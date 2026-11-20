//! What a walk's and a mock Job's API Client reads and sends, as Fleet would answer: no Fleet is asked.

use ipc::{Placed, RequestCollection, RequestEnvironmentRead, RequestHeader, RequestPlace, RequestSent, Requests, RunPlace, SavedRequest, VariableRead};

use super::wire::{Answered, ToSend};

fn header(name: &str, value: &str) -> RequestHeader {
    RequestHeader { name: name.into(), value: value.into() }
}

fn saved(id: &str, name: &str, method: &str, url: &str, body: Option<&str>) -> SavedRequest {
    SavedRequest { id: id.into(), name: name.into(), method: method.into(), url: url.into(), headers: Vec::new(), body: body.map(str::to_string) }
}

pub fn requests() -> Requests {
    Requests {
        repository: "/repo".into(),
        place: RunPlace::ThisMac,
        collections: vec![Placed {
            item: RequestCollection {
                id: "service".into(),
                name: "Service".into(),
                headers: vec![header("Accept", "application/json")],
                requests: vec![
                    saved("health", "Health", "GET", "{{base}}/health", None),
                    saved("list-users", "List users", "GET", "{{base}}/users?limit=20", None),
                    saved("make-user", "Create user", "POST", "{{base}}/users", Some("{\"name\": \"Ada\", \"admin\": false}")),
                ],
            },
            place: RequestPlace::Yours,
            hides: false,
        }],
        environments: vec![
            Placed {
                item: RequestEnvironmentRead { name: "server:api".into(), variables: vec![VariableRead::Plain { name: "base".into(), value: "http://localhost:4300".into() }] },
                place: RequestPlace::Server,
                hides: false,
            },
            Placed {
                item: RequestEnvironmentRead {
                    name: "staging".into(),
                    variables: vec![
                        VariableRead::Plain { name: "base".into(), value: "https://staging.example.test".into() },
                        VariableRead::Secret { name: "token".into(), set: true },
                    ],
                },
                place: RequestPlace::Yours,
                hides: false,
            },
        ],
        not_read: Vec::new(),
    }
}

/// What sending answers: a JSON list for a read, a created one for a write, and a refusal to connect for an address no server has.
pub fn send(to_send: &ToSend) -> Answered {
    let url = super::model::filled(&to_send.url, None).text;
    let sent = if to_send.url.contains("{{") {
        RequestSent { url, status: None, ms: 12, headers: Vec::new(), body: String::new(), truncated: false, error: Some("A name with no value was left in the address".into()), note: None, saved: None }
    } else if to_send.method == "POST" {
        RequestSent {
            url,
            status: Some(201),
            ms: 84,
            headers: vec![header("content-type", "application/json"), header("location", "/users/7")],
            body: "{\"id\":7,\"name\":\"Ada\",\"admin\":false}".into(),
            truncated: false,
            error: None,
            note: None,
            saved: None,
        }
    } else {
        RequestSent {
            url,
            status: Some(200),
            ms: 1240,
            headers: vec![header("content-type", "application/json"), header("cache-control", "no-store")],
            body: "{\"users\":[{\"id\":1,\"name\":\"Grace\"},{\"id\":2,\"name\":\"Alan\"}],\"next\":null}".into(),
            truncated: false,
            error: None,
            note: None,
            saved: None,
        }
    };
    Answered::of(sent)
}
