//! His other Macs on the same network, over Bonjour: each Gateway advertises `_armada._tcp` with its id,
//! name, colour and build at its Wi-Fi door's port, and browses for the others. **What Bonjour says is
//! unchecked**: it only says where to knock. Pairing checks who answers with the four digits, and a
//! paired Mac's door with its pin. `docs/concepts/pocket.md`, *Macs nearby*.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use ipc::MacColour;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::task::JoinHandle;

use crate::tunnel::tunnel_name;
use crate::Gateway;

/// The Bonjour service type every Armada Gateway advertises.
pub(crate) const SERVICE: &str = "_armada._tcp.local.";

/// Another Mac as Bonjour last said it, with where its door answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NearbyMac {
    pub id: String,
    pub name: String,
    pub colour: Option<MacColour>,
    pub build: Option<String>,
    /// `https://<ip>:<port>`.
    pub address: String,
}

#[derive(Default)]
struct Kept {
    own: Option<(u16, String)>,
    /// By the Bonjour name it was resolved under, which is what its removal names.
    seen: HashMap<String, NearbyMac>,
}

/// This Mac's door and the Macs seen nearby, shared by every clone of the Gateway.
#[derive(Clone, Default)]
pub struct Nearby {
    kept: Arc<Mutex<Kept>>,
}

impl Nearby {
    pub(crate) fn set_own(&self, port: u16, pin: String) {
        self.kept.lock().unwrap().own = Some((port, pin));
    }

    /// The pin of this Mac's own door, where it is open.
    pub(crate) fn own_pin(&self) -> Option<String> {
        self.kept.lock().unwrap().own.as_ref().map(|(_, pin)| pin.clone())
    }

    pub(crate) fn saw(&self, key: String, mac: NearbyMac) {
        self.kept.lock().unwrap().seen.insert(key, mac);
    }

    pub(crate) fn lost(&self, key: &str) {
        self.kept.lock().unwrap().seen.remove(key);
    }

    /// Each Mac seen, once, in id order.
    pub(crate) fn seen(&self) -> Vec<NearbyMac> {
        let kept = self.kept.lock().unwrap();
        let mut all: Vec<NearbyMac> = kept.seen.values().cloned().collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        all.dedup_by(|a, b| a.id == b.id);
        all
    }

    /// Where Mac `id`'s door answers, while it is seen.
    pub(crate) fn door_of(&self, id: &str) -> Option<String> {
        self.kept.lock().unwrap().seen.values().find(|mac| mac.id == id).map(|mac| mac.address.clone())
    }

    /// The Mac seen nearby whose tunnel name is `tunnel`.
    pub(crate) fn by_tunnel(&self, tunnel: &str) -> Option<NearbyMac> {
        self.seen().into_iter().find(|mac| tunnel_name(&mac.id) == tunnel)
    }
}

/// Bonjour, advertising and browsing until [`Bonjour::stop`].
pub(crate) struct Bonjour {
    daemon: ServiceDaemon,
    browsing: JoinHandle<()>,
}

impl Bonjour {
    pub(crate) fn stop(self) {
        self.browsing.abort();
        let _ = self.daemon.shutdown();
    }
}

/// Advertise this Mac's door at `port`, and keep each other Mac Bonjour finds in `gateway.macs.nearby`.
/// **This is what macOS asks Local Network access for**, the first time the Gateway starts.
pub(crate) fn advertise_and_browse(gateway: &Gateway, port: u16) -> Result<Bonjour, String> {
    let me = crate::mac_client::hello(gateway)?;
    let daemon = ServiceDaemon::new().map_err(|why| why.to_string())?;
    let instance = tunnel_name(&me.id);
    let mut said = HashMap::from([
        ("id".to_string(), me.id.clone()),
        ("name".to_string(), me.name.clone()),
        ("colour".to_string(), me.colour.name().to_string()),
    ]);
    if let Some(build) = me.build.clone() {
        said.insert("build".to_string(), build);
    }
    let info = ServiceInfo::new(SERVICE, &instance, &format!("{instance}.local."), "", port, said)
        .map_err(|why| why.to_string())?
        .enable_addr_auto();
    daemon.register(info).map_err(|why| why.to_string())?;
    let events = daemon.browse(SERVICE).map_err(|why| why.to_string())?;
    let nearby = gateway.macs.nearby.clone();
    let browsing = tokio::spawn(async move {
        while let Ok(event) = events.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(found) => {
                    let addresses = found.addresses.iter().map(|ip| ip.to_ip_addr()).collect();
                    let read = |key: &str| found.txt_properties.get_property_val_str(key).map(str::to_string);
                    match read_mac(read, &addresses, found.port) {
                        Some(mac) if mac.id != me.id => nearby.saw(found.fullname.clone(), mac),
                        _ => {}
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => nearby.lost(&fullname),
                _ => {}
            }
        }
    });
    Ok(Bonjour { daemon, browsing })
}

/// A Mac out of what Bonjour resolved: its id and name as a pairing Mac must give them, and an IPv4
/// address that is not link-local where there is one, else any.
pub(crate) fn read_mac(read: impl Fn(&str) -> Option<String>, addresses: &HashSet<IpAddr>, port: u16) -> Option<NearbyMac> {
    let id = read("id")?;
    let name = read("name")?;
    let plain = !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit());
    if !plain || name.trim().is_empty() || name.len() > 80 || port == 0 {
        return None;
    }
    let mut ips: Vec<&IpAddr> = addresses.iter().collect();
    ips.sort_by_key(|ip| match ip {
        IpAddr::V4(v4) if !v4.is_link_local() && !v4.is_loopback() => (0, ip.to_string()),
        IpAddr::V4(_) => (1, ip.to_string()),
        IpAddr::V6(_) => (2, ip.to_string()),
    });
    let address = match ips.first()? {
        IpAddr::V4(v4) => format!("https://{v4}:{port}"),
        // A scoped link-local address cannot be written in a URL that reqwest takes.
        IpAddr::V6(v6) if v6.segments()[0] & 0xffc0 == 0xfe80 => return None,
        IpAddr::V6(v6) => format!("https://[{v6}]:{port}"),
    };
    Some(NearbyMac {
        id,
        name,
        colour: read("colour").as_deref().and_then(MacColour::named),
        build: read("build").filter(|build| !build.is_empty()),
        address,
    })
}

/// Where to call paired Mac `mac`, and with what, in the order to try: its door while Bonjour sees it and
/// its pin is kept, then `address`, its tunnel, with the plain client. An error where neither is there.
pub(crate) fn reach(gateway: &Gateway, mac: &str, address: &str) -> Result<Vec<(String, reqwest::Client)>, String> {
    let pin = gateway.pairing.store.lock().unwrap().paired_mac(mac).ok().flatten().and_then(|row| row.pin);
    let mut ways = Vec::new();
    if let (Some(door), Some(pin)) = (gateway.macs.nearby.door_of(mac), pin) {
        ways.push((door, gateway.macs.pinned(&pin)?));
    }
    if !address.is_empty() {
        ways.push((address.to_string(), gateway.macs.http.clone()));
    }
    if ways.is_empty() {
        return Err("It is not on this network, and it has no address to reach it elsewhere.".to_string());
    }
    Ok(ways)
}

/// Where to reach a Mac this one is pairing with, before its row holds the pin: its door with `pin`
/// while Bonjour sees it, else its tunnel `address`.
pub(crate) fn reach_pairing(
    gateway: &Gateway,
    mac: &str,
    address: Option<&str>,
    pin: Option<&str>,
) -> Option<(String, reqwest::Client)> {
    if let (Some(door), Some(pin)) = (gateway.macs.nearby.door_of(mac), pin) {
        if let Ok(client) = gateway.macs.pinned(pin) {
            return Some((door, client));
        }
    }
    address.filter(|address| crate::macs::reachable(address)).map(|address| (address.to_string(), gateway.macs.http.clone()))
}
