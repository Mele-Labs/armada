//! Which door a request came through, named before a handler sees it.
//! `docs/concepts/retro.md`, *Who acted*.
//!
//! **A task-local, so no command's signature changes to carry it.** Fleet signs
//! a person's move in dozens of places, and every one of them is reached from a
//! handler awaiting it inside this scope; [`via`] reads the answer where the
//! move is written. A move a handler spawns onto another task is outside the
//! scope and reads `None`, which records it as Fleet records a move made with
//! no request behind it — unless the spawn goes through [`carrying`], as every
//! command Fleet races against its budget does.
//!
//! **Bridge names itself, and absence is not Bridge.** A header any process can
//! send is attribution and not authentication: what it buys is that an agent's
//! `curl` stops reading as a person's press. The door's two callers are told
//! apart by extensions the door sets, which bytes on the wire cannot carry.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use ipc::Via;

use crate::door::{DoorCalled, HelmCalled};

/// The header Bridge sends on every request, with [`BRIDGE`] as its value.
pub const CALLER_HEADER: &str = "x-armada-caller";

/// What Bridge says in [`CALLER_HEADER`].
pub const BRIDGE: &str = "bridge";

tokio::task_local! {
    static VIA: Via;
}

/// The door the request being answered came through, or `None` where nothing
/// is being answered: a turn, a spawned task, a test calling Fleet directly.
pub fn via() -> Option<Via> {
    VIA.try_with(|via| *via).ok()
}

/// `work`, answered inside the door `via` names: for a handler that hands its
/// work to a task of its own, which a task-local does not follow.
pub async fn carrying<F: std::future::Future>(via: Option<Via>, work: F) -> F::Output {
    match via {
        Some(via) => VIA.scope(via, work).await,
        None => work.await,
    }
}

/// The layer: name the door, then answer inside it.
pub(crate) async fn named_the_door(request: Request, next: Next) -> Response {
    let via = door_of(&request);
    VIA.scope(via, next.run(request)).await
}

fn door_of(request: &Request) -> Via {
    let spelled = if request.extensions().get::<HelmCalled>().is_some() {
        "helm"
    } else if request.extensions().get::<DoorCalled>().is_some() {
        "door"
    } else if request
        .headers()
        .get(CALLER_HEADER)
        .is_some_and(|said| said.as_bytes() == BRIDGE.as_bytes())
    {
        "bridge"
    } else {
        "http"
    };
    // Total: the four spellings are the domain's own.
    Via::from_wire(spelled).unwrap_or_else(|| unreachable!("`{spelled}` is a door"))
}
