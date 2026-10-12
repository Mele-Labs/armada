//! The four limits a person changes while Fleet runs — Drones at once, the
//! memory share, the disk floor, Checks at once — and how a saved one reaches
//! admission and the gate.
//!
//! **Shipped, overlaid by saved.** The composition root hands in the shipped
//! numbers as [`Fittings`](crate::daemon::Fittings)' `concurrency`, `headroom`
//! and `checks_at_once`; a value settings.json holds replaces its shipped one
//! at assembly and at every save or hand edit. `config::settings` has already
//! refused a file holding one out of range, so nothing here checks a range.
//!
//! **A change reaches the next admission and the next Check, and nothing
//! else.** The roster's bound and the headroom are replaced under the roster
//! lock, so no admission sees one limit changed and the other not; a Drone
//! already working keeps working, a gate already running keeps the headroom it
//! began with, and a saved Checks at once counts from the next place given out.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Resolved};
use ipc::{FleetLimits, LimitValues, SaveLimits, SaveSettings, SettingValue};

use crate::daemon::Fleet;
use crate::headroom::{Bytes, Headroom, Spare};
use crate::places::ChecksAtOnce;
use crate::slots::Concurrency;

/// The limits admission holds a new Drone to, and a gate holds its Checks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub concurrency: Concurrency,
    pub headroom: Headroom,
    pub checks_at_once: ChecksAtOnce,
}

impl Limits {
    /// These limits with every value settings.json chose put over them.
    pub fn overlaid_by(self, settings: &Resolved) -> Limits {
        let whole = |key| settings.chosen(key).and_then(|n: i64| u32::try_from(n).ok());
        let concurrency = whole(keys::DRONES_AT_ONCE)
            .map(|jobs| Concurrency::of(jobs as usize))
            .unwrap_or(self.concurrency);
        let spare = whole(keys::MEMORY_SPARE_PERCENT)
            .map(Spare::percent)
            .unwrap_or(self.headroom.memory_spare());
        let disk = whole(keys::DISK_FLOOR_GIB)
            .map(|floor| Bytes::gibibytes(u64::from(floor)))
            .unwrap_or(self.headroom.disk_floor());
        let checks_at_once = whole(keys::CHECKS_AT_ONCE)
            .map(|checks| ChecksAtOnce::of(checks as usize))
            .unwrap_or(self.checks_at_once);
        Limits {
            concurrency,
            headroom: Headroom::of(spare, disk),
            checks_at_once,
        }
    }

    /// As the wire spells them. Saturating rather than `as`, for
    /// `ipc::FleetCapacity::of`'s reason.
    pub fn values(&self) -> LimitValues {
        LimitValues {
            concurrency: u32::try_from(self.concurrency.jobs()).unwrap_or(u32::MAX),
            memory_spare_percent: self.headroom.memory_spare().percentage(),
            disk_floor_gib: u32::try_from(self.headroom.disk_floor().whole_gibibytes())
                .unwrap_or(u32::MAX),
            checks_at_once: u32::try_from(self.checks_at_once.get()).unwrap_or(u32::MAX),
        }
    }
}

/// A save of the older shape as the keys it names. **An omitted field is not
/// a key**, so it keeps whatever the file holds — including nothing.
fn as_changes(save: &SaveLimits) -> SaveSettings {
    let mut changes = BTreeMap::new();
    let mut named = |key: keys::Integer, value: Option<u32>| {
        if let Some(value) = value {
            changes.insert(key_name(key), Some(SettingValue::Integer(i64::from(value))));
        }
    };
    named(keys::DRONES_AT_ONCE, save.concurrency.map(|v| v.get()));
    named(keys::MEMORY_SPARE_PERCENT, save.memory_spare_percent.map(|v| v.get()));
    named(keys::DISK_FLOOR_GIB, save.disk_floor_gib.map(|v| v.get()));
    named(keys::CHECKS_AT_ONCE, save.checks_at_once.map(|v| v.get()));
    SaveSettings { changes }
}

fn key_name(key: impl config::settings::Key) -> String {
    key.name().to_string()
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
    /// The limits in force and what shipped, read under the roster so the bound
    /// and the headroom are one instant.
    pub(crate) async fn limits_in_force(&self) -> FleetLimits {
        let slots = self.slots().lock().await;
        FleetLimits {
            values: Limits {
                concurrency: Concurrency::of(slots.cap()),
                headroom: self.headroom(),
                checks_at_once: self.checks_at_once(),
            }
            .values(),
            shipped: self.shipped().values(),
        }
    }

    /// Save into settings.json, then answer with what is in force.
    pub(crate) async fn save_limits_now(&self, save: SaveLimits) -> Result<FleetLimits, Refusal> {
        self.save_settings_now(as_changes(&save)).await?;
        Ok(self.limits_in_force().await)
    }

    /// Put the limits settings.json holds in force.
    ///
    /// **Roster first**, the order `crate::slots` states, and the file is
    /// already written by the time this runs, so nothing in memory moves on a
    /// save that did not land.
    pub(crate) async fn relimited(&self, settings: &Resolved) {
        let mut slots = self.slots().lock().await;
        let limits = self.shipped().overlaid_by(settings);
        slots.rebound(limits.concurrency);
        self.rehoused(limits.headroom);
        self.rechecked(limits.checks_at_once);
        // **Beside `rebound`, because it is derived from it.** A person who
        // raised the Jobs bound narrowed every Check on the machine by the same
        // act, and a width still divided by the old bound would hand out more
        // of the machine than the new one allows. #1444.
        self.rewidened(limits.concurrency);
    }
}
