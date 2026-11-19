//! A Refresh of both Macs, claimed so each Mac shows it running, not only the one restarting.
//!
//! **Held in memory only.** The pressing Mac's own claim stands while its wrapper's status says `running`,
//! so it follows the stage and ends with the build without a record of its own. A claim another Mac told this
//! one of is kept under that Mac and lapses after the same twenty quiet minutes a `running` status does; a
//! Fleet that restarts holds none, which is how the Mac restarting onto the build clears it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{BuildSource, RefreshClaim, RefreshClaimSent, WireError};

use crate::building::{build_word, status_in, Building, Status, RESTARTING, RUNNING_FOR_AT_MOST};
use crate::daemon::Fleet;
use crate::phone::PhoneSwitch;

/// Where a Mac is told of this one's Refresh.
const CLAIM_PATH: &str = "/other_mac/refresh_claim";
/// How often the wrapper's stage is read, to tell the other Mac when it moves.
const FOLLOW_EVERY: Duration = Duration::from_secs(1);
/// A send that has not answered by now is given up on: the claim it carried is not shown there.
const SEND_WITHIN: Duration = Duration::from_secs(10);

/// What a Fleet holds of Refreshes: its own, and each another Mac told it of.
#[derive(Clone, Default)]
pub(crate) struct Claims(Arc<Mutex<Held>>);

#[derive(Default)]
struct Held {
    own: Option<Own>,
    others: BTreeMap<String, Heard>,
}

struct Own {
    mac: String,
    name: String,
    at: SystemTime,
}

struct Heard {
    claim: RefreshClaim,
    heard: SystemTime,
    /// The build its Mac said it ran when it told the claim, where it had said.
    build: Option<String>,
}

fn instant(at: SystemTime) -> ipc::Instant {
    let millis = at.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    ipc::Instant::carried(crate::clock::rfc3339_utc(i64::try_from(millis).unwrap_or(0)))
}

fn standing(heard: &Heard, now: SystemTime) -> bool {
    now.duration_since(heard.heard).unwrap_or_default() <= RUNNING_FOR_AT_MOST
}

impl Claims {
    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Building {
    /// This Mac pressed a Refresh.
    pub(crate) fn refresh_pressed(&self, mac: &str, name: &str, now: SystemTime) {
        self.refresh.held().own = Some(Own { mac: mac.to_string(), name: name.to_string(), at: now });
    }

    /// Mac `from` told this Mac of its Refresh. **Kept under `from`**, the id the link's signature named, and
    /// not under whatever the claim says.
    pub(crate) fn refresh_heard(&self, from: &str, mut claim: RefreshClaim, build: Option<String>, now: SystemTime) {
        claim.mac = from.to_string();
        self.refresh.held().others.insert(from.to_string(), Heard { claim, heard: now, build });
    }

    /// Mac `from` said its Refresh ended.
    pub(crate) fn refresh_dropped(&self, from: &str) {
        self.refresh.held().others.remove(from);
    }

    /// Forget the claim of each Mac that is no longer paired, or whose build is no longer the one it had when it
    /// told its claim: it restarted, and what it restarted onto is the build.
    pub(crate) fn refresh_forget_rebuilt(&self, builds: &[(String, Option<String>)]) {
        self.refresh.held().others.retain(|mac, heard| match builds.iter().find(|(id, _)| id == mac) {
            None => false,
            Some((_, now)) => heard.build.is_none() || now.is_none() || *now == heard.build,
        });
    }

    /// This Mac's own Refresh, where its wrapper still works: **the status file's stage, read now**. A Refresh
    /// whose wrapper stopped, failed or went quiet is over, and is forgotten.
    pub(crate) fn refresh_own(&self, support: &Path, now: SystemTime) -> Option<RefreshClaim> {
        let mut held = self.refresh.held();
        let own = held.own.as_ref()?;
        let Some(Status::Running(_, stage)) = status_in(support, now) else {
            held.own = None;
            return None;
        };
        Some(RefreshClaim { mac: own.mac.clone(), name: own.name.clone(), stage, at: instant(own.at) })
    }

    /// The Refresh to show: this Mac's own, else the latest of those another Mac told it of that has not lapsed.
    pub(crate) fn refresh_shown(&self, support: &Path, now: SystemTime) -> Option<RefreshClaim> {
        if let Some(own) = self.refresh_own(support, now) {
            return Some(own);
        }
        let mut held = self.refresh.held();
        held.others.retain(|_, heard| standing(heard, now));
        held.others.values().max_by_key(|heard| heard.heard).map(|heard| heard.claim.clone())
    }

    /// Whether a Refresh another Mac told this one of stands.
    pub(crate) fn refresh_held(&self, now: SystemTime) -> bool {
        self.refresh.held().others.values().any(|heard| standing(heard, now))
    }
}

/// Tell each of `others` of `sent`, one after the other so a stage and its end arrive in order. **What cannot be
/// sent is not an error**: a Mac asleep or on another build shows no claim, and the Refresh goes on.
async fn told(switch: &Arc<dyn PhoneSwitch>, others: &[String], sent: &RefreshClaimSent) {
    let Ok(body) = ipc::encode(sent) else {
        return;
    };
    for mac in others {
        let sending = switch.send_other_mac(mac, CLAIM_PATH, body.clone().into_bytes());
        let _ = tokio::time::timeout(SEND_WITHIN, sending).await;
    }
}

/// Tell `others` of this Mac's Refresh now, and again each time its stage moves, and that it ended.
fn follow(switch: Arc<dyn PhoneSwitch>, building: Building, support: PathBuf, others: Vec<String>) {
    drop(tokio::spawn(async move {
        let mut last: Option<RefreshClaim> = None;
        loop {
            let own = building.refresh_own(&support, SystemTime::now());
            if own != last {
                told(&switch, &others, &RefreshClaimSent { claim: own.clone() }).await;
                last = own.clone();
            }
            if own.is_none() {
                return;
            }
            tokio::time::sleep(FOLLOW_EVERY).await;
        }
    }));
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
    /// A Refresh was accepted here: claim it, and tell `tell`, else every Mac linked to this one. **Nothing is
    /// claimed where no other Mac is linked**, since there is no one to show it to.
    pub(crate) async fn refresh_pressed(&self, tell: Option<&str>) {
        let (me, others) = {
            let store = self.store().lock().await;
            let Some(me) = store.this_mac().ok().flatten() else {
                return;
            };
            let others: Vec<String> = match tell {
                Some(mac) => vec![mac.to_string()],
                None => store
                    .paired_macs()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|mac| mac.state == store::MacState::Linked)
                    .map(|mac| mac.id)
                    .collect(),
            };
            (me, others)
        };
        if others.is_empty() {
            return;
        }
        self.building().refresh_pressed(&me.id, &me.name, SystemTime::now());
        if let Ok(switch) = self.phone_switch() {
            let support = crate::runtime::support_dir(&self.host().home);
            follow(Arc::clone(switch), self.building().clone(), support, others);
        }
    }

    /// `hold_refresh_claim`: Mac `mac` told this Mac of its Refresh, or that it ended.
    pub(crate) async fn refresh_claim_held(&self, mac: String, sent: RefreshClaimSent) -> Result<(), Refusal> {
        let mac = self.mac_named(&mac)?;
        match sent.claim {
            Some(claim) => {
                let build = self.store().lock().await.paired_mac(&mac).ok().flatten().and_then(|row| row.build);
                self.building().refresh_heard(&mac, claim, build, SystemTime::now());
            }
            None => self.building().refresh_dropped(&mac),
        }
        Ok(())
    }

    /// Refused while another Mac's Refresh stands, as a restart under way is: a second press cannot start another.
    /// **A call another Mac makes is let through**: the build a claim announces reaches this Mac as one.
    pub(crate) fn refresh_unclaimed(&self) -> Result<(), Refusal> {
        if api::via().is_some_and(|via| via.as_wire() == api::MAC) || !self.building().refresh_held(SystemTime::now()) {
            return Ok(());
        }
        let said = format!("restarting onto {}", build_word(BuildSource::Preview));
        Err(Refusal::IllegalMove(WireError::raised(RESTARTING, said, self.run_id())))
    }

    /// Forget the claims of Macs that have since come back on another build, as the store last heard.
    pub(crate) async fn refresh_forget_rebuilt(&self) {
        let paired = self.store().lock().await.paired_macs().unwrap_or_default();
        let builds: Vec<(String, Option<String>)> = paired.into_iter().map(|mac| (mac.id, mac.build)).collect();
        self.building().refresh_forget_rebuilt(&builds);
    }
}
