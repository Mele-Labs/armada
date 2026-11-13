//! The Phone Gateway: what the owner's phone talks to, in front of Fleet.
//!
//! Fleet listens on loopback, takes no auth and refuses any request that
//! carries `Origin`, and that stays as it is. The Gateway sits between, inside
//! Fleet's own process but on a listener of its own: `127.0.0.1` only, for
//! the owner's tunnel to reach (`tailscale serve` unless `pocket-address` names
//! another), calling Fleet over HTTP with no `Origin`.
//!
//! **Every route is on a list.** [`router`] names each one, and anything else
//! is 404. The routes whose issue has not landed answer 501.

mod admin;
mod cloudflared;
mod cors;
mod fleet_client;
mod fleet_events;
mod forward;
mod live;
mod mac_ask_routes;
mod mac_client;
mod mac_keys;
mod mac_link;
mod mac_pages;
mod mac_pair_routes;
mod mac_routes;
mod mac_setup;
mod machine;
mod macs;
mod pair_routes;
mod pairing;
mod phone_sessions;
mod push;
mod reads;
mod routes;
mod session_routes;
mod setup;
mod shown_address;
mod signing;
mod stat;
mod thread_window;
mod tunnel;
mod vapid;
mod watch;
mod webpush;

use std::io;
use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::TcpListener;

pub use fleet_client::{Answer, Unreachable};
pub use mac_keys::{MacKeys, MemoryMacKeys};
pub use mac_link::{link, read_other_mac, send_other_mac};
pub use mac_pages::{open_other_mac_page, PageOpen};
pub use mac_setup::{mac_setup, set_up_mac};
pub use macs::{MacHello, Macs, Timing};
pub use pairing::{
    address_from_file, address_from_status, machine_address, tailscale_address, Address, Clock, Pairing, ADDRESS_FILE,
};
pub use push::{Push, PLACEHOLDER_SUBJECT};
pub use watch::follow;
pub use routes::{router, Fleet, Gateway};
pub use setup::{mac_way_in, Funnel, WayIn};
pub use tunnel::Tunnel;

/// Bind the Gateway's listener. **There is no host parameter**: the address is
/// loopback by construction, so no setting can open it to a network.
pub async fn bind(port: u16) -> io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await
}

/// [`bind`] without waiting, for a switch that turns the Gateway on from a
/// plain call. Loopback by construction, the same way. Address reuse is on, as
/// [`bind`]'s is, so the port is taken again at once after the Gateway stops.
/// Called from inside a Tokio runtime.
pub fn bind_now(port: u16) -> io::Result<TcpListener> {
    let socket = tokio::net::TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))?;
    socket.listen(1024)
}

#[cfg(test)]
mod mac_ask_tests;
#[cfg(test)]
mod mac_tests;
#[cfg(test)]
mod pairing_tests;
#[cfg(test)]
mod push_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod shown_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod thread_tests;
#[cfg(test)]
mod tunnel_tests;
