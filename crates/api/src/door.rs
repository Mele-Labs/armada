//! The agent's door: a second MCP path on the listener already bound.
//!
//! **It calls the HTTP surface rather than the daemon.** Every tool is one row
//! of `crates/ipc/operations/` reached at the route [`SERVED`] already
//! names, so there is no second implementation to drift and a route that moves
//! moves for both callers at once.
//!
//! It answers 405 to `GET` and `DELETE`: no server-initiated stream, no session to
//! end, and so nothing added to the unbounded-sink risk on the event socket.
//!
//! Who may open this door is `#698`. What it is scoped to is here: the
//! Manifest its relay names as `?manifest_id=`, which selects and grants
//! nothing. So is the one caller it narrows: a Helm session the daemon places
//! by its connection (`#941`, [`crate::Admitting`]).

use axum::body::Bytes;
use axum::extract::Query;
use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use ipc::door::{self, Answer, Answered, Asked, Reachable, Shape};
use ipc::ManifestId;
use tower::ServiceExt;

use crate::daemon::{offerable, Admitting, Queries};
use crate::placed::Placed;
use crate::routes::SERVED;
use crate::scoped::InManifest;
use crate::served::Served;

/// Where an agent reaches Fleet.
///
/// The HTTP surface, as MCP tools: every row the inventory marks reachable.
pub const DOOR_PATH: &str = "/agent/mcp";

/// The most a tool call's own message may weigh: a few fields, never a stream.
/// Stated rather than left to the extractor's default.
const MOST_A_CALL_MAY_BE: usize = 1024 * 1024;

/// The query parameters a route reads, where it reads any.
///
/// **The route table's own fact.** A path segment is
/// visible in the path and a query is not, so this is the one thing about a
/// route the door cannot read off [`SERVED`].
const QUERIES: &[(&str, &[&str])] = &[
    ("search_files", &["q"]),
    ("get_events_since", &["since"]),
    ("list_lessons", &["lands_in", "state", "most"]),
    ("get_logbook", &["repository", "section", "state"]),
    ("list_sessions", &["q", "state"]),
    ("who_owns", &["kind", "target"]),
    ("validate_mod", &["name"]),
    ("get_requests", &["repository", "session_id"]),
    ("list_watches", &["session_id"]),
];

/// The query parameters `operation`'s route reads, none where it reads none.
fn queried(operation: &str) -> &'static [&'static str] {
    QUERIES
        .iter()
        .find(|(one, _)| *one == operation)
        .map(|(_, names)| *names)
        .unwrap_or(&[])
}

/// Every tool this door offers: the inventory's `agent_access` column, joined
/// to the route table.
///
/// **The set is the file's and the route is this crate's.** An operation the
/// inventory marks reachable and nothing serves gets an empty path and is
/// listed as not served — which the gate refuses, and which is answered rather
/// than silently dropped if it ever happens.
pub fn offered() -> Vec<Shape> {
    shaped(door::REACHABLE.iter())
}

/// Set on a call the door makes for a Helm session it placed, and by nothing
/// else. **An extension, never a header**: bytes on the wire cannot carry one.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HelmCalled;

/// Set on every call the door makes, Helm's or not, so the move it leads to
/// says it came through the door. [`HelmCalled`]'s reason for an extension.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DoorCalled;

fn shaped<'a>(rows: impl Iterator<Item = &'a Reachable>) -> Vec<Shape> {
    rows.map(|row| {
        let route = SERVED.iter().find(|route| route.operation == row.operation);
        Shape {
            operation: row.operation,
            method: route.map(|route| route.method).unwrap_or("GET"),
            path: route.map(|route| route.path).unwrap_or(""),
            query: queried(row.operation),
        }
    })
    .collect()
}

/// The Manifest a session is answered inside.
///
/// **Named by the relay, checked by the daemon.** `armada mcp` names the
/// Manifest it walked to; a caller naming none is answered inside the first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope(String);

impl Scope {
    pub fn of(manifest_id: impl Into<String>) -> Scope {
        Scope(manifest_id.into())
    }

    pub fn named(&self) -> &str {
        &self.0
    }
}

/// The door path a relay standing in `manifest_id` posts to.
pub fn door_within(manifest_id: &str) -> String {
    format!("{DOOR_PATH}?manifest_id={}", door::encoded(manifest_id))
}

/// The door path a relay standing in no Manifest posts to: a hosted session's,
/// in a root Setup has not written `armada.yml` in yet.
///
/// **Said rather than left absent**, because an absent `?manifest_id=` is
/// answered inside the first Manifest, which is another repository's.
pub fn door_without_a_manifest() -> String {
    format!("{DOOR_PATH}?{WITHOUT_A_MANIFEST}=true")
}

const WITHOUT_A_MANIFEST: &str = "without_manifest";

/// What a hosted session standing in no Manifest is offered: the permission
/// tool its `--permission-prompt-tool` names, and its own acts, each placed by
/// its connection and none read inside a Manifest.
const WITHOUT_A_MANIFEST_OFFERS: &[&str] = &[
    door::ASKS_A_PERSON,
    "waiting_for",
    "all_done",
    "show_window",
    "withdraw_window",
    "show_sketch",
    "show_file",
    "record_plan",
    "ask_in_window",
    "save_kit_manifest",
    // A request is sent from the Job, not from a repository.
    "send_request",
    "get_requests",
    "save_request_to_collection",
];

/// The routes that read `?manifest_id=` and, absent it, answer about every
/// repository or the first. The door always names its scope on these.
const NAMES_ITS_SCOPE: &[&str] = &[
    "get_events_since",
    "list_worktrees",
    "list_sessions",
    "who_owns",
    "change_slot_pool",
    "rescue_slot",
    "list_servers",
    "get_manifest_reading",
    "get_manifest_drift",
    "start_server",
    "list_studios",
    // The main checkout's runs, which a Helm session starts and reads (#1288).
    // Absent a name they answer about the first repository served, so a
    // session standing in another would start and read runs in that one.
    "get_checkout_run_sheet",
    "list_checkout_runs",
    "get_checkout_run_output",
    "start_checkout_run",
    "stop_checkout_run",
    "undo_checkout_run",
    "start_checkout_verify",
    "list_lessons",
    // The thread that keeps where the words went is the session's own repository's.
    "tell_session",
];

/// Set on every call the door makes, so a `:job_id` resolves inside the
/// session's Manifest. **An extension, never a header**, for [`HelmCalled`]'s
/// reason: Bridge's own `/jobs/:job_id` requests cannot carry one.
#[derive(Clone, Debug)]
pub(crate) struct Scoped(ManifestId);

impl Scoped {
    pub(crate) fn manifest_id(&self) -> ManifestId {
        self.0.clone()
    }
}

/// What a door handler holds: the daemon, for the scope, and the surface every
/// tool call is made against.
///
/// **The surface is a second `Router` over the same state**, not the one this
/// is merged into — a router holding itself is not a value that exists. It is
/// the same table built twice, which costs a clone of a route list at start
/// and nothing afterwards.
pub struct Doorway<D> {
    served: Served<D>,
    surface: Router,
}

/// Cloned per request like every other state on this listener. **Written out
/// rather than derived**, because `#[derive(Clone)]` would demand `D: Clone`
/// and the daemon is held by `Arc` precisely so it is not.
impl<D> Clone for Doorway<D> {
    fn clone(&self) -> Doorway<D> {
        Doorway {
            served: self.served.clone(),
            surface: self.surface.clone(),
        }
    }
}

pub fn mounted<D: Queries + Admitting>(served: Served<D>, surface: Router) -> Router {
    Router::new()
        .route(
            DOOR_PATH,
            post(called::<D>).get(no_stream).delete(no_stream),
        )
        .layer(axum::extract::DefaultBodyLimit::max(MOST_A_CALL_MAY_BE))
        .with_state(Doorway { served, surface })
}

/// One JSON-RPC message in, at most one out.
///
/// A notification is acknowledged with 202 and no body, for
/// JSON-RPC forbids answering one.
async fn called<D: Queries + Admitting>(
    State(doorway): State<Doorway<D>>,
    request: axum::extract::Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let named = Query::<InManifest>::try_from_uri(&parts.uri)
        .ok()
        .and_then(|Query(scope)| scope.manifest());
    let caller = crate::mcp::who_called(&parts);
    let placed = Placed::of(doorway.served.daemon(), caller);
    let body = match axum::body::to_bytes(body, MOST_A_CALL_MAY_BE).await {
        Ok(body) => body,
        // Answered as unreadable bytes.
        Err(_) => Bytes::new(),
    };
    if Query::<Relayed>::try_from_uri(&parts.uri)
        .is_ok_and(|Query(relayed)| relayed.without_manifest)
    {
        return match without_a_manifest(&doorway, &body, &placed, caller).await {
            Some(answered) => answering(answered),
            None => StatusCode::ACCEPTED.into_response(),
        };
    }
    // **A Helm session reads against everything it could be
    // offered**, so a call outside its reach is refused by name below rather
    // than answered as a tool that does not exist.
    let shapes = match placed.is_narrowed() {
        false => offered(),
        true => shaped(offerable()),
    };
    let answered = match door::read(&body, &shapes) {
        Asked::Nothing => return StatusCode::ACCEPTED.into_response(),
        Asked::Ping { id } => Answered::Ping { id },
        Asked::Tools { id } => Answered::Tools {
            id,
            shapes: within(shapes, &placed),
        },
        Asked::NoSuchMethod { id, named } => Answered::NoSuchMethod { id, named },
        Asked::Unreadable { why } => Answered::Unreadable { why },
        // Refused as a tool error and never a status code, for the reason the
        // Drone's endpoint refuses that way: an agent reads the one and can
        // only retry the other.
        Asked::NotACall { id, why } => Answered::Refused { id, why },
        Asked::Handshake { id, revision } => match doorway.scope(named).await {
            Ok(scope) => Answered::Handshake {
                id,
                revision,
                scope: scope.named().to_string(),
            },
            Err(why) => Answered::Refused { id, why },
        },
        Asked::Call { id, call } => match doorway.scope(named).await {
            Err(why) => Answered::Refused { id, why },
            Ok(_) if !placed.may(call.operation) => Answered::Refused {
                id,
                why: placed.refusing(call.operation),
            },
            Ok(scope) => match within_scope(&call, &scope) {
                Ok(call) => Answered::Served {
                    id,
                    answer: doorway.through(&call, &placed, Some(&scope), caller).await,
                },
                Err(why) => Answered::Refused { id, why },
            },
        },
    };
    answering(answered)
}

/// `?without_manifest=` on the door, which [`door_without_a_manifest`] sets.
#[derive(serde::Deserialize)]
struct Relayed {
    #[serde(default)]
    without_manifest: bool,
}

/// One message from a relay standing in no Manifest.
///
/// **Only a hosted session is answered**, because what is offered is that
/// session's own acts and nothing else here has any. Anyone else is told so at
/// the handshake and listed nothing.
async fn without_a_manifest<D: Queries>(
    doorway: &Doorway<D>,
    body: &[u8],
    placed: &Placed,
    caller: crate::mcp::Caller,
) -> Option<Answered> {
    let session = placed.is_a_hosted_session();
    let shapes = match session {
        true => shaped(
            crate::daemon::offerable()
                .filter(|row| WITHOUT_A_MANIFEST_OFFERS.contains(&row.operation)),
        ),
        false => Vec::new(),
    };
    Some(match door::read(body, &shapes) {
        Asked::Nothing => return None,
        Asked::Ping { id } => Answered::Ping { id },
        Asked::Tools { id } => Answered::Tools { id, shapes },
        Asked::NoSuchMethod { id, named } => Answered::NoSuchMethod { id, named },
        Asked::Unreadable { why } => Answered::Unreadable { why },
        Asked::Handshake { id, revision } if session => {
            Answered::WithoutAManifest { id, revision }
        }
        Asked::Handshake { id, revision } => Answered::Unreachable {
            id,
            revision,
            why: String::from(NO_SESSION_HERE),
        },
        Asked::NotACall { id, why } if session => Answered::Refused {
            id,
            why: format!(
                "{why}. This repository has no armada.yml yet, so only this job's own acts \
                 are served here"
            ),
        },
        Asked::NotACall { id, .. } => Answered::Refused {
            id,
            why: String::from(NO_SESSION_HERE),
        },
        Asked::Call { id, call } => Answered::Served {
            id,
            answer: doorway.through(&call, placed, None, caller).await,
        },
    })
}

const NO_SESSION_HERE: &str = "this directory has no armada.yml, and this connection is not a \
     job Armada is hosting, so nothing here is answered";

fn answering(answered: Answered) -> Response {
    match door::answer(answered) {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        // Unreachable for plain data, and answered rather than panicked: a
        // panic here drops the connection an agent's whole session runs on.
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// The tools a caller is listed: every shape for any agent, and only what the
/// reach decided for a Helm session.
fn within(shapes: Vec<Shape>, placed: &Placed) -> Vec<Shape> {
    shapes
        .into_iter()
        .filter(|shape| placed.may(shape.operation))
        .collect()
}

impl<D: Queries> Doorway<D> {
    /// The Manifest this session is answered inside.
    ///
    /// **Asked per call rather than held.** A session that outlived a
    /// Manifest reload would be answering inside one Fleet no longer serves,
    /// and the read is a field on a struct.
    async fn scope(&self, named: Option<ManifestId>) -> Result<Scope, String> {
        match self.served.daemon().scope(named).await {
            Ok(manifest_id) => Ok(Scope::of(manifest_id.as_str())),
            Err(refusal) => Err(format!(
                "this Fleet could not say which Manifest your job is inside, so nothing \
                 can be answered within one: {}",
                refusal.error().message
            )),
        }
    }

    /// One tool call, made against the surface inside `scope`. `placed`
    /// marks it for the route that records who acted.
    async fn through(
        &self,
        call: &door::Call,
        placed: &Placed,
        scope: Option<&Scope>,
        caller: crate::mcp::Caller,
    ) -> Answer {
        if let Some(scope) = scope {
            if let Some(why) = self.elsewhere(call, scope).await {
                return refused_here(call, &why);
            }
        }
        let path = match scope {
            Some(scope) => as_an_agent_reads(call, scoped_path(call, scope)),
            None => call.path.clone(),
        };
        let request = Request::builder()
            .method(call.method)
            .uri(&path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(
                call.body.clone().unwrap_or_default(),
            ));
        let Ok(mut request) = request else {
            return unanswerable(call, "that call did not make a request");
        };
        placed.mark(&mut request);
        request.extensions_mut().insert(DoorCalled);
        request
            .extensions_mut()
            .insert(crate::acting::DoorCaller(caller));
        if let Some(scope) = scope {
            request
                .extensions_mut()
                .insert(Scoped(ManifestId::carried(scope.named())));
        }
        // Infallible: the surface's error type is `Infallible`, so a failure
        // here is a request that was never made rather than a route that
        // refused — and a refusal comes back as a status like any other.
        let response = match self.surface.clone().oneshot(request).await {
            Ok(response) => response,
            Err(_) => return unanswerable(call, "the surface did not answer"),
        };
        let status = response.status().as_u16();
        let media_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json")
            .to_string();
        let body = axum::body::to_bytes(response.into_body(), MOST_A_RESPONSE_MAY_BE)
            .await
            .map(|bytes| bytes.to_vec())
            .unwrap_or_default();
        Answer {
            status,
            media_type,
            body,
            whole_at: path,
            narrowed_by: queried(call.operation),
        }
    }
}

/// The call as the session's scope allows it: refused where it names another
/// Manifest.
fn within_scope(call: &door::Call, scope: &Scope) -> Result<door::Call, String> {
    if let Some(named) = call
        .manifest_id
        .as_deref()
        .filter(|named| *named != scope.named())
    {
        return Err(format!(
            "this job is answered only about Manifest `{}`, the one it stands in, and \
             `{}` named `{named}`",
            scope.named(),
            call.operation
        ));
    }
    Ok(call.clone())
}

impl<D: Queries> Doorway<D> {
    /// Why a call naming a server by id is not the session's to make:
    /// another repository owns it. `None` lets the route answer, a miss included.
    async fn elsewhere(&self, call: &door::Call, scope: &Scope) -> Option<String> {
        let daemon = self.served.daemon();
        let within = ManifestId::carried(scope.named());
        let (what, id) = match call.operation {
            "stop_server" => {
                let id = door::named_in(call.body.as_deref()?, "id")?;
                let every = daemon.list_servers(None).await.ok()?;
                let mine = daemon.list_servers(Some(within)).await.ok()?;
                let held = |list: &ipc::ServerList| list.servers.iter().any(|one| one.id == id);
                if !held(&every) || held(&mine) {
                    return None;
                }
                ("server", id)
            }
            _ => return None,
        };
        Some(format!(
            "{what} `{id}` belongs to another repository, and this job is answered only \
             about Manifest `{}`, the one it stands in",
            scope.named()
        ))
    }
}

/// A call the door refused before the surface saw it, answered as a tool error.
fn refused_here(call: &door::Call, why: &str) -> Answer {
    Answer {
        status: 403,
        media_type: "text/plain".to_string(),
        body: why.as_bytes().to_vec(),
        whole_at: call.path.clone(),
        narrowed_by: &[],
    }
}

/// The call's path, with the session's Manifest named where the route reads
/// one. **Replaced, never appended beside another.**
fn scoped_path(call: &door::Call, scope: &Scope) -> String {
    if !NAMES_ITS_SCOPE.contains(&call.operation) {
        return call.path.clone();
    }
    let (path, query) = call.path.split_once('?').unwrap_or((&call.path, ""));
    let mut pairs: Vec<String> = query
        .split('&')
        .filter(|pair| !pair.is_empty() && !pair.starts_with("manifest_id="))
        .map(str::to_string)
        .collect();
    pairs.push(format!("manifest_id={}", door::encoded(scope.named())));
    format!("{path}?{}", pairs.join("&"))
}

/// `list_sessions` answers an agent a line a Session and only the live ones: whole, the ledger
/// is 696 KB against this door's 64 KiB. The view is forced and is not among [`QUERIES`], so an
/// agent cannot ask for the wide one; the state is a default it can set to `ended`.
fn as_an_agent_reads(call: &door::Call, path: String) -> String {
    if call.operation != "list_sessions" {
        return path;
    }
    let (route, query) = path.split_once('?').unwrap_or((&path, ""));
    let mut pairs: Vec<&str> = query
        .split('&')
        .filter(|pair| !pair.is_empty() && !pair.starts_with("view="))
        .collect();
    if !pairs.iter().any(|pair| pair.starts_with("state=")) {
        pairs.push("state=live");
    }
    pairs.push("view=summary");
    format!("{route}?{}", pairs.join("&"))
}

/// The most a surface answer may weigh before the door stops reading it.
///
/// **Far above the cap the answer is cut to**, so what an agent is told is
/// *cut at 65536 of 900000* rather than a body that silently ended. A diff
/// larger than this is one no session was going to hold either way.
const MOST_A_RESPONSE_MAY_BE: usize = 16 * 1024 * 1024;

/// A call the transport could not make or could not read back. **A 500 in the
/// answer, never a dropped connection**, so the agent reads a sentence.
fn unanswerable(call: &door::Call, why: &str) -> Answer {
    Answer {
        status: 500,
        media_type: "text/plain".to_string(),
        body: format!("{why}: {} {}", call.method, call.path).into_bytes(),
        whole_at: call.path.clone(),
        narrowed_by: &[],
    }
}

/// There is no server-initiated stream and no session to end.
async fn no_stream() -> Response {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}
