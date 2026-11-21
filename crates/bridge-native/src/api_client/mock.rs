//! What a walk's and a mock Job's API Client reads and sends, as Fleet would answer: no Fleet is asked.

use ipc::{
    Instant, Placed, RequestBy, RequestCollection, RequestEnvironmentRead, RequestEnvironmentWrite, RequestHeader, RequestPlace, RequestRun, RequestRunAnswer, RequestRunJob, RequestRunSent,
    RequestRunSummary, RequestRuns, RequestSaved, RequestSent, RequestToSave, Requests, RequestsNotRead, RunPlace, SavedRequest, SessionId, VariableRead, VariableWrite,
};

use super::wire::{Answered, ToSend, RUNS_PAGE};

fn header(name: &str, value: &str) -> RequestHeader {
    RequestHeader { name: name.into(), value: value.into() }
}

fn saved(id: &str, name: &str, method: &str, url: &str, body: Option<&str>) -> SavedRequest {
    SavedRequest { id: id.into(), name: name.into(), method: method.into(), url: url.into(), headers: Vec::new(), body: body.map(str::to_string) }
}

fn requests() -> Requests {
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

// ---- what saving, deleting and listing runs answer ----------------------------------------------

/// What a mock Job's API Client keeps between calls, as Fleet would: the two files' collections and environments, and each
/// secret as set or not, never its value.
pub struct Store {
    read: Requests,
    runs: Vec<RequestRun>,
    job: String,
    kept: u32,
}

/// Set to `yours` or `repository` to start the mock with that file unable to read, so the caution line and every Save that goes
/// off with it can be seen without Fleet.
pub const UNREAD_SWITCH: &str = "ARMADA_MOCK_REQUESTS_UNREAD";

/// Fleet's refusal of a save over the person's own file while it will not read.
fn unread_refusal(why: &str) -> String {
    format!("Your requests file would not read: {why}")
}

fn at(minute: u32) -> Instant {
    Instant::carried(format!("2026-11-20T09:{minute:02}:00Z"))
}

/// The runs of the Health request: newest first once listed, more than a page of them.
fn seeded_runs(job: &str) -> Vec<RequestRun> {
    (0..27u32)
        .map(|n| {
            let agent = n % 3 == 1;
            let other = n % 9 == 4;
            let failed = n % 7 == 5;
            let (status, error) = if failed { (None, Some("connection refused".to_string())) } else if n % 5 == 3 { (Some(503), None) } else { (Some(200), None) };
            RequestRun {
                id: format!("run-{n:02}"),
                at: at(n + 1),
                by: if agent { RequestBy::Agent } else { RequestBy::Owner },
                job: if other {
                    Some(RequestRunJob { id: SessionId::carried("other-job"), title: Some("Add rate limits".into()) })
                } else {
                    Some(RequestRunJob { id: SessionId::carried(job), title: Some("This Job".into()) })
                },
                collection: "service".into(),
                request: "health".into(),
                environment: Some("server:api".into()),
                sent: RequestRunSent { method: "GET".into(), url: "http://localhost:4300/health".into(), headers: vec![header("Accept", "application/json")], body: None },
                answer: RequestRunAnswer {
                    status,
                    ms: 20 + u64::from(n) * 7,
                    headers: if failed { Vec::new() } else { vec![header("content-type", "application/json"), header("cache-control", "no-store")] },
                    body: if failed { String::new() } else { "{\"ok\":true,\"uptime\":1204}".into() },
                    truncated: false,
                    bytes: 24,
                    content_type: Some("application/json".into()),
                    error,
                },
            }
        })
        .collect()
}

fn blank(name: &str, what: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        Err(format!("{what} is blank."))
    } else {
        Ok(())
    }
}

impl Store {
    pub fn new(job: &str) -> Store {
        let mut read = requests();
        match std::env::var(UNREAD_SWITCH).as_deref() {
            Ok("yours") => read.not_read.push(RequestsNotRead { place: RequestPlace::Yours, why: "expected `=` after a key at line 3 column 9".into() }),
            Ok("repository") => read.not_read.push(RequestsNotRead { place: RequestPlace::Repository, why: "a table has no name at line 12".into() }),
            _ => {}
        }
        Store { read, runs: seeded_runs(job), job: job.to_string(), kept: 0 }
    }

    /// The why of the person's own file not reading, where it does not.
    fn yours_unread(&self) -> Option<String> {
        self.read.not_read.iter().find(|one| one.place == RequestPlace::Yours).map(|one| unread_refusal(&one.why))
    }

    /// A send of a saved request is one of its runs, newest first.
    pub fn kept_run(&mut self, to_send: &ToSend, answered: &Answered) {
        let Some((collection, request)) = to_send.saved.clone() else { return };
        self.kept += 1;
        let sent = &answered.sent;
        self.runs.push(RequestRun {
            id: format!("run-kept-{}", self.kept),
            at: Instant::carried(format!("2026-11-20T10:{:02}:00Z", self.kept.min(59))),
            by: RequestBy::Owner,
            job: Some(RequestRunJob { id: SessionId::carried(self.job.clone()), title: Some("This Job".into()) }),
            collection,
            request,
            environment: to_send.environment.clone(),
            sent: RequestRunSent { method: to_send.method.clone(), url: sent.url.clone(), headers: to_send.headers.clone(), body: to_send.body.clone() },
            answer: RequestRunAnswer {
                status: sent.status,
                ms: sent.ms,
                headers: sent.headers.clone(),
                body: sent.body.clone(),
                truncated: sent.truncated,
                bytes: sent.body.len() as u64,
                content_type: None,
                error: sent.error.clone(),
            },
        });
    }

    pub fn read(&self) -> Requests {
        self.read.clone()
    }

    pub fn save_collection(&mut self, collection: RequestCollection) -> Result<Requests, String> {
        if let Some(refused) = self.yours_unread() {
            return Err(refused);
        }
        blank(&collection.id, "collection.id")?;
        blank(&collection.name, "collection.name")?;
        let mut seen = Vec::new();
        for (at, request) in collection.requests.iter().enumerate() {
            blank(&request.id, &format!("collection.requests[{at}].id"))?;
            if seen.contains(&request.id) {
                return Err(format!("collection.requests[{at}].id is used twice."));
            }
            seen.push(request.id.clone());
            if !super::model::METHODS.contains(&request.method.to_ascii_uppercase().as_str()) {
                return Err(format!("collection.requests[{at}].method is not a method Fleet sends."));
            }
        }
        if let Some(at) = collection.headers.iter().position(|one| one.name.trim().is_empty() || one.name.chars().any(char::is_whitespace)) {
            return Err(format!("collection.headers[{at}].name is not a header name."));
        }
        let kept = Placed { item: collection, place: RequestPlace::Yours, hides: false };
        match self.read.collections.iter().position(|one| one.item.id == kept.item.id) {
            Some(at) => {
                let hides = self.read.collections[at].place == RequestPlace::Repository || self.read.collections[at].hides;
                self.read.collections[at] = Placed { hides, ..kept };
            }
            None => self.read.collections.push(kept),
        }
        Ok(self.read())
    }

    pub fn save_request(&mut self, collection: &str, request: RequestToSave) -> Result<RequestSaved, String> {
        if let Some(refused) = self.yours_unread() {
            return Err(refused);
        }
        blank(&request.name, "request.name")?;
        blank(collection, "collection")?;
        if !super::model::METHODS.contains(&request.method.to_ascii_uppercase().as_str()) {
            return Err("request.method is not a method Fleet sends.".to_string());
        }
        let at = match self.read.collections.iter().position(|one| one.item.id == collection || one.item.name == collection) {
            Some(at) => at,
            None => {
                self.read.collections.push(Placed {
                    item: RequestCollection { id: super::model::id_of(collection), name: collection.to_string(), headers: Vec::new(), requests: Vec::new() },
                    place: RequestPlace::Yours,
                    hides: false,
                });
                self.read.collections.len() - 1
            }
        };
        let placed = &mut self.read.collections[at];
        let kept = SavedRequest { id: String::new(), name: request.name.trim().to_string(), method: request.method.to_ascii_uppercase(), url: request.url, headers: request.headers, body: request.body };
        let id = match placed.item.requests.iter().position(|one| one.name == kept.name) {
            Some(at) => {
                let id = placed.item.requests[at].id.clone();
                placed.item.requests[at] = SavedRequest { id: id.clone(), ..kept };
                id
            }
            None => {
                let base = super::model::id_of(&kept.name);
                let mut id = base.clone();
                let mut n = 2;
                while placed.item.requests.iter().any(|one| one.id == id) {
                    id = format!("{base}-{n}");
                    n += 1;
                }
                placed.item.requests.push(SavedRequest { id: id.clone(), ..kept });
                id
            }
        };
        placed.place = RequestPlace::Yours;
        Ok(RequestSaved { collection_id: placed.item.id.clone(), request_id: id })
    }

    /// An environment saved whole. A secret sent with a value is set and the value goes no further; one sent with none keeps
    /// what it was; a variable left out is forgotten.
    pub fn save_environment(&mut self, environment: RequestEnvironmentWrite) -> Result<Requests, String> {
        if let Some(refused) = self.yours_unread() {
            return Err(refused);
        }
        blank(&environment.name, "environment.name")?;
        if environment.name.starts_with(ipc::SERVER_ENVIRONMENT_PREFIX) {
            return Err(format!("environment.name cannot start with `{}`.", ipc::SERVER_ENVIRONMENT_PREFIX));
        }
        let before: Vec<VariableRead> = self.read.environments.iter().find(|one| one.item.name == environment.name && one.place != RequestPlace::Server).map(|one| one.item.variables.clone()).unwrap_or_default();
        let mut variables = Vec::new();
        for (at, variable) in environment.variables.into_iter().enumerate() {
            let name = match &variable {
                VariableWrite::Plain { name, .. } | VariableWrite::Secret { name, .. } => name.clone(),
            };
            blank(&name, &format!("environment.variables[{at}].name"))?;
            if !name.chars().all(|one| one.is_ascii_alphanumeric() || matches!(one, '_' | '.' | '-')) {
                return Err(format!("environment.variables[{at}].name has a character a name cannot."));
            }
            if variables.iter().any(|one: &VariableRead| match one {
                VariableRead::Plain { name: other, .. } | VariableRead::Secret { name: other, .. } => *other == name,
            }) {
                return Err(format!("environment.variables[{at}].name is used twice."));
            }
            variables.push(match variable {
                VariableWrite::Plain { name, value } => VariableRead::Plain { name, value },
                VariableWrite::Secret { name, value } => {
                    let was_set = before.iter().any(|one| matches!(one, VariableRead::Secret { name: other, set: true } if *other == name));
                    VariableRead::Secret { name, set: value.is_some() || was_set }
                }
            });
        }
        let kept = Placed { item: RequestEnvironmentRead { name: environment.name.clone(), variables }, place: RequestPlace::Yours, hides: false };
        match self.read.environments.iter().position(|one| one.item.name == environment.name && one.place != RequestPlace::Server) {
            Some(at) => {
                let hides = self.read.environments[at].place == RequestPlace::Repository || self.read.environments[at].hides;
                self.read.environments[at] = Placed { hides, ..kept };
            }
            None => {
                let at = self.read.environments.iter().position(|one| one.place == RequestPlace::Server).unwrap_or(self.read.environments.len());
                self.read.environments.insert(at, kept);
            }
        }
        Ok(self.read())
    }

    pub fn delete_environment(&mut self, name: &str) -> Result<Requests, String> {
        if let Some(refused) = self.yours_unread() {
            return Err(refused);
        }
        match self.read.environments.iter().position(|one| one.item.name == name && one.place == RequestPlace::Yours) {
            Some(at) => {
                self.read.environments.remove(at);
                Ok(self.read())
            }
            None => Err(format!("No environment named `{name}` in your own file.")),
        }
    }

    /// A page of a request's runs, newest first, `RUNS_PAGE` at a time.
    pub fn runs(&self, collection: &str, request: &str, before: Option<&str>) -> RequestRuns {
        let mut all: Vec<&RequestRun> = self.runs.iter().filter(|run| run.collection == collection && run.request == request).collect();
        all.sort_by(|a, b| b.at.cmp(&a.at));
        let from = before.and_then(|before| all.iter().position(|run| run.id == before)).map_or(0, |at| at + 1);
        let page: Vec<&RequestRun> = all.iter().skip(from).take(RUNS_PAGE).copied().collect();
        let next = (from + page.len() < all.len()).then(|| page.last().map(|run| run.id.clone())).flatten();
        let runs = page
            .into_iter()
            .map(|run| RequestRunSummary {
                id: run.id.clone(),
                at: run.at.clone(),
                by: run.by,
                job: run.job.clone(),
                method: run.sent.method.clone(),
                url: run.sent.url.clone(),
                status: run.answer.status,
                ms: run.answer.ms,
                error: run.answer.error.clone(),
                summary: run.sent.body.clone().unwrap_or_default(),
            })
            .collect();
        RequestRuns { runs, next }
    }

    pub fn run(&self, id: &str) -> Result<RequestRun, String> {
        self.runs.iter().find(|run| run.id == id).cloned().ok_or_else(|| "No such run is kept.".to_string())
    }
}
