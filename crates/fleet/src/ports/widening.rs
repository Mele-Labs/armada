//! A checkout's span read against a Manifest that is not the one Fleet booted with, and widened
//! where that Manifest names more ports than the span has.
//!
//! **Why this exists.** A run saved while Fleet is up declares ports Fleet did not hold when it
//! claimed the checkout's span, and a run starts at once, with no restart. `docs/concepts/fleet.md`,
//! *Servers*: the span is held for as long as Fleet runs, so it grows rather than being re-claimed
//! under a server that is up.

use std::collections::{BTreeMap, BTreeSet};

use config::Manifest;
use store::{PortClaim, PortClaimant};

use super::{env_names, fits, pick_span, rounded_width, BindConnectProbe, PortProbe, PortsRefused};
use crate::daemon::Fleet;
use crate::servers::Holder;
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

/// Each port of `fresh` on the span from `base`, as `(name, port)`; **a port may fall past the
/// span's end, and then the span is what grows.**
///
/// A name Fleet booted with keeps the offset the booted Manifest gives it, since the proof run after
/// a merge reads its ports from that Manifest and must not meet a server on one. A name the boot
/// did not know goes after them, and a port a live server is on stays that server's whatever the
/// order says, so a start never hands out a port that something answers on.
pub(crate) fn spread(
    held: &[String],
    fresh: &[String],
    live: &BTreeMap<String, u16>,
    base: u16,
) -> BTreeMap<String, u16> {
    let mut used: BTreeSet<u16> = live.values().copied().collect();
    let mut placed: BTreeMap<String, u16> = BTreeMap::new();
    for name in fresh {
        if let Some(port) = live.get(name) {
            placed.insert(name.clone(), *port);
        }
    }
    for (offset, name) in held.iter().enumerate() {
        let port = base + offset as u16;
        if fresh.contains(name) && !placed.contains_key(name) && used.insert(port) {
            placed.insert(name.clone(), port);
        }
    }
    let mut next = base + held.len() as u16;
    let rest: Vec<&String> = fresh.iter().filter(|name| !placed.contains_key(*name)).collect();
    for name in rest {
        while used.contains(&next) {
            next += 1;
        }
        used.insert(next);
        placed.insert(name.clone(), next);
    }
    placed
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The ports `fresh` declares, on the span the checkout at `path` holds: claimed on first need,
    /// and **widened where `fresh` names more than it has room for.** `held` is the Manifest Fleet
    /// booted with, whose offsets stay where they were. A refusal is said, and nothing is started.
    pub(crate) async fn checkout_ports_declared(
        &self,
        path: &str,
        held: &Manifest,
        fresh: &Manifest,
    ) -> Result<BTreeMap<String, u16>, PortsRefused> {
        let wanted = fresh.port_names();
        if wanted.is_empty() {
            return Ok(BTreeMap::new());
        }
        env_names(fresh)?;
        let holder = Holder::Checkout(path.to_string());
        let live: BTreeMap<String, u16> = self
            .servers()
            .ports_live(&holder)
            .into_iter()
            .map(|one| (one.name, one.port))
            .collect();
        // A pinned server is on a number outside the span, so it takes no part in laying the span out.
        let leased: BTreeMap<String, u16> = live
            .iter()
            .filter(|(name, _)| fresh.port(name).is_none_or(|port| port.fixed().is_none()))
            .map(|(name, port)| (name.clone(), *port))
            .collect();
        let claim = match self.checkout_claim(path).await {
            Some(claim) => claim,
            None => {
                self.try_claim(PortClaimant::Checkout(path.to_string()), fresh).await?;
                match self.checkout_claim(path).await {
                    Some(claim) => claim,
                    None => return Err(PortsRefused::RangeExhausted { width: 0 }),
                }
            }
        };
        let placed = spread(&held.port_names(), &wanted, &leased, claim.base);
        let top = placed.values().copied().max().unwrap_or(claim.base);
        let needed = u32::from(top - claim.base) + 1;
        if needed <= u32::from(claim.width) {
            return self.pins_applied(fresh, placed, &live);
        }
        let widened = self.span_widened(claim.clone(), needed, self.servers().any_live(&holder)).await?;
        // A span that moved is one nothing was up on, so its ports are laid out again from its base.
        let placed = match widened.base == claim.base {
            true => placed,
            false => spread(&held.port_names(), &wanted, &leased, widened.base),
        };
        self.pins_applied(fresh, placed, &live)
    }

    /// `placed` with every port a person pinned put at its number. **A pinned port keeps its place
    /// in the span and is never moved**: where something else holds the number the start is
    /// refused, naming what, since another number would be the one the program was pinned to avoid.
    fn pins_applied(
        &self,
        fresh: &Manifest,
        mut placed: BTreeMap<String, u16>,
        live: &BTreeMap<String, u16>,
    ) -> Result<BTreeMap<String, u16>, PortsRefused> {
        for name in fresh.port_names() {
            let Some(fixed) = fresh.port(&name).and_then(|port| port.fixed()) else {
                continue;
            };
            // This server's own instance is the one already on it.
            if live.get(&name) == Some(&fixed) {
                placed.insert(name, fixed);
                continue;
            }
            // **Fleet's own servers first**: one that has started has not always bound yet, so
            // the probe alone would call its number free.
            let holder = self
                .servers()
                .every()
                .into_iter()
                .find(|one| one.phase != ipc::ServerPhase::Exited && one.ports.iter().any(|port| port.port == fixed))
                .map(|one| format!("`{}` on {}", one.name, one.checkout.path));
            if holder.is_some() || !BindConnectProbe.free(fixed) {
                return Err(PortsRefused::PinnedTaken { name, port: fixed, holder });
            }
            placed.insert(name, fixed);
        }
        Ok(placed)
    }

    /// `claim` made wide enough for `needed` ports. **In place where the ports after it are free**,
    /// so a server that is up keeps its own; else a new span, which only a checkout with nothing up
    /// can take, since a server on the old span would be left on ports nobody holds.
    async fn span_widened(&self, claim: PortClaim, needed: u32, any_live: bool) -> Result<PortClaim, PortsRefused> {
        let width = rounded_width(needed.min(u32::from(u16::MAX)) as usize, self.port_range().granule());
        let store = self.store();
        let mut store = store.lock().await;
        let others: Vec<(u16, u16)> = store
            .every_port_claim()
            .map_err(PortsRefused::Database)?
            .iter()
            .filter(|other| other.claimant != claim.claimant)
            .map(|other| (other.base, other.width))
            .collect();
        let tail = claim.base.saturating_add(claim.width)..=claim.base.saturating_add(width - 1);
        let base = if fits(self.port_range(), claim.base, width, &others) && tail.into_iter().all(|port| BindConnectProbe.free(port)) {
            claim.base
        } else if any_live {
            return Err(PortsRefused::RangeExhausted { width });
        } else {
            pick_span(self.port_range(), width, &others, &BindConnectProbe)
                .ok_or(PortsRefused::RangeExhausted { width })?
        };
        let widened = PortClaim { base, width, claimed_at: self.now(), ..claim };
        store.release_port_span(&widened.claimant).map_err(PortsRefused::Write)?;
        store.claim_port_span(&widened).map_err(PortsRefused::Write)?;
        Ok(widened)
    }
}
