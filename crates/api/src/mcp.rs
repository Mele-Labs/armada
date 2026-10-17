//! Who is on the other end of a connection.
//!
//! The Drone endpoint that lived here is gone with the Drone. [`Caller`] stays:
//! the door places a session by the connection its call arrived on.

use std::net::SocketAddr;

use axum::extract::ConnectInfo;

/// Where a call came from: the address the listener saw on the other
/// end of the connection it arrived on.
///
/// **Minted by the transport and by nothing else.** There is no constructor
/// taking a session id and no field a caller could put one in, so which session
/// a call comes from is a property of the type. What the daemon does with it —
/// matching the port pair against the processes it spawned — is
/// `fleet::peer`'s, and this crate holds none of that.
///
/// `None` for the port is a request that arrived with no connection information
/// on it, which is a test's router rather than a served one. It attributes to
/// nothing, and a call is refused rather than guessed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller(Option<SocketAddr>);

impl Caller {
    /// The peer of an accepted connection.
    pub fn at(peer: SocketAddr) -> Caller {
        Caller(Some(peer))
    }

    /// A call that arrived with no peer address. See the type.
    pub fn unplaceable() -> Caller {
        Caller(None)
    }

    /// The port the peer opened the connection from. **Half of the pair** —
    /// the other half is the port Fleet is listening on, and the daemon knows
    /// that for itself.
    pub fn port(&self) -> Option<u16> {
        self.0.map(|peer| peer.port())
    }
}

/// The peer of the connection this request arrived on.
///
/// **Read out of the extensions rather than extracted**, so a router served
/// without `into_make_service_with_connect_info` answers
/// [`Caller::unplaceable`] instead of rejecting the request. A rejection would
/// be a 500 where the honest answer is that nothing said who called.
pub(crate) fn who_called(parts: &axum::http::request::Parts) -> Caller {
    match parts.extensions.get::<ConnectInfo<SocketAddr>>() {
        Some(ConnectInfo(peer)) => Caller::at(*peer),
        None => Caller::unplaceable(),
    }
}
