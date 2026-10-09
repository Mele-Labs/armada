//! The Phone Gateway: what the owner's phone talks to, in front of Fleet.
//!
//! Fleet listens on loopback, takes no auth and refuses any request that
//! carries `Origin`, and that stays as it is. This process sits between: it
//! listens on `127.0.0.1` only, for `tailscale serve` to reach, and calls Fleet
//! with no `Origin`.
//!
//! **Every route is on a list.** [`router`] names each one, and anything else
//! is 404. The routes whose issue has not landed answer 501.

mod actions;
mod admin;
mod fleet_client;
mod fleet_events;
mod live;
mod pair_routes;
mod pairing;
mod phone;
mod push;
mod reads;
mod routes;
mod signing;
mod stat;
mod vapid;
mod watch;
mod webpush;

use std::io;
use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::TcpListener;

pub use fleet_client::{Answer, Unreachable};
pub use pairing::{address_from_status, tailscale_address, Address, Clock, Pairing};
pub use push::{Push, PLACEHOLDER_SUBJECT};
pub use watch::follow;
pub use routes::{router, Fleet, Gateway};

/// Bind the Gateway's listener. **There is no host parameter**: the address is
/// loopback by construction, so no setting can open it to a network.
pub async fn bind(port: u16) -> io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await
}

#[cfg(test)]
mod actions_tests;
#[cfg(test)]
mod pairing_tests;
#[cfg(test)]
mod push_tests;
#[cfg(test)]
mod reads_tests;
#[cfg(test)]
mod tests;
