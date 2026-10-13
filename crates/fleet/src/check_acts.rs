//! A person's hand on a gate that is running Checks: skip one, skip what is
//! left, run one again.
//!
//! **The gate decides nothing differently.** Each act is a request left on the
//! run's [`Control`]; `crate::checking::ran` reads it between awaits and does
//! what it would do for any Check — a skip ends the command and records
//! `Observed::Waived`, which advances the step and is not a pass, and a retry
//! runs that one command again while the ruling waits for it.
//!
//! **Open until the loop has nothing running or queued**, and shut by the one
//! call that checks no request is left unread. An act after that is refused
//! rather than accepted and dropped: the ruling is being written down.

use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId};
use tokio::sync::watch;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Why an act on a Check is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Unactable {
    /// No gate of this Job is running Checks.
    NoGate,
    /// The gate has no Check of that name.
    NoSuchCheck(String),
    /// That Check has already finished.
    Finished(String),
    /// Nothing left to skip: every Check has finished.
    NothingLeft,
    /// That Check is not a command, so there is nothing to run again.
    NotACommand(String),
}

impl Unactable {
    pub(crate) fn said(&self) -> String {
        match self {
            Unactable::NoGate => String::from("no gate is running Checks for this Job"),
            Unactable::NoSuchCheck(name) => {
                format!("the gate has no Check named `{name}`")
            }
            Unactable::Finished(name) => format!("`{name}` has already finished"),
            Unactable::NothingLeft => String::from("every Check has already finished"),
            Unactable::NotACommand(name) => {
                format!("`{name}` runs no command, so there is nothing to run again")
            }
        }
    }
}

/// What was asked of a run since the loop last looked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interrupt {
    Skip,
    Retry,
}

struct Slot {
    name: String,
    /// Whether it is a command the loop can run again.
    retryable: bool,
    skip: bool,
    retry: bool,
}

struct State {
    open: bool,
    slots: Vec<Slot>,
}

/// One gate's run, as a person's requests reach it. By position in the step's
/// declaration, as `crate::underway`'s rows are.
pub(crate) struct Control {
    state: Mutex<State>,
    wake: watch::Sender<u64>,
}

impl Control {
    /// `(name, retryable)` for each declared Check.
    pub(crate) fn new(declared: impl Iterator<Item = (String, bool)>) -> Control {
        Control {
            state: Mutex::new(State {
                open: true,
                slots: declared
                    .map(|(name, retryable)| Slot {
                        name,
                        retryable,
                        skip: false,
                        retry: false,
                    })
                    .collect(),
            }),
            wake: watch::channel(0).0,
        }
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.wake.subscribe()
    }

    fn acting<T>(
        &self,
        change: impl FnOnce(&mut Vec<Slot>) -> Result<T, Unactable>,
    ) -> Result<T, Unactable> {
        let done = {
            let mut state = self.state.lock().map_err(|_| Unactable::NoGate)?;
            if !state.open {
                return Err(Unactable::NoGate);
            }
            change(&mut state.slots)?
        };
        self.wake.send_modify(|n| *n += 1);
        Ok(done)
    }

    /// `finished` is by position: whether that row already has a result.
    pub(crate) fn skip(&self, finished: &[bool], name: &str) -> Result<(), Unactable> {
        self.acting(|slots| {
            let at = slots
                .iter()
                .position(|slot| slot.name == name)
                .ok_or_else(|| Unactable::NoSuchCheck(name.to_string()))?;
            if finished.get(at).copied().unwrap_or(false) {
                return Err(Unactable::Finished(name.to_string()));
            }
            slots[at].skip = true;
            slots[at].retry = false;
            Ok(())
        })
    }

    pub(crate) fn skip_all(&self, finished: &[bool]) -> Result<(), Unactable> {
        self.acting(|slots| {
            let mut any = false;
            for (at, slot) in slots.iter_mut().enumerate() {
                if !finished.get(at).copied().unwrap_or(false) {
                    slot.skip = true;
                    slot.retry = false;
                    any = true;
                }
            }
            any.then_some(()).ok_or(Unactable::NothingLeft)
        })
    }

    pub(crate) fn retry(&self, name: &str) -> Result<(), Unactable> {
        self.acting(|slots| {
            let slot = slots
                .iter_mut()
                .find(|slot| slot.name == name)
                .ok_or_else(|| Unactable::NoSuchCheck(name.to_string()))?;
            if !slot.retryable {
                return Err(Unactable::NotACommand(name.to_string()));
            }
            slot.retry = true;
            slot.skip = false;
            Ok(())
        })
    }

    /// What the Check at `at` has been asked, a skip before a retry.
    pub(crate) fn pending(&self, at: usize) -> Option<Interrupt> {
        let state = self.state.lock().ok()?;
        let slot = state.slots.get(at)?;
        match (slot.skip, slot.retry) {
            (true, _) => Some(Interrupt::Skip),
            (false, true) => Some(Interrupt::Retry),
            (false, false) => None,
        }
    }

    /// Every Check with a request on it, by position.
    pub(crate) fn requested(&self) -> Vec<(usize, Interrupt)> {
        let Ok(state) = self.state.lock() else {
            return Vec::new();
        };
        state
            .slots
            .iter()
            .enumerate()
            .filter_map(|(at, slot)| match (slot.skip, slot.retry) {
                (true, _) => Some((at, Interrupt::Skip)),
                (false, true) => Some((at, Interrupt::Retry)),
                (false, false) => None,
            })
            .collect()
    }

    /// The request on `at` has been read.
    pub(crate) fn settled(&self, at: usize) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(slot) = state.slots.get_mut(at) {
                slot.skip = false;
                slot.retry = false;
            }
        }
    }

    /// Shut where no request is waiting to be read. `false` leaves it open.
    pub(crate) fn close_if_idle(&self) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return true;
        };
        if state.slots.iter().any(|slot| slot.skip || slot.retry) {
            return false;
        }
        state.open = false;
        true
    }

    /// Shut whatever is waiting: the run is over.
    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.open = false;
        }
    }
}

/// Completes once the Check at `at` has been asked to skip or run again, and
/// never for a run nobody can ask anything of.
pub(crate) async fn interrupted(control: Option<Arc<Control>>, at: usize) {
    let Some(control) = control else {
        return std::future::pending().await;
    };
    let mut seen = control.subscribe();
    loop {
        if control.pending(at).is_some() {
            return;
        }
        if seen.changed().await.is_err() {
            return std::future::pending().await;
        }
    }
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
    /// End one Check of the running gate, recorded as skipped.
    pub async fn skip_check(&self, job_id: &JobId, check: &str) -> Result<Job, Adrift> {
        self.act_on_checks(job_id, |control, finished| control.skip(finished, check))
            .await
    }

    /// End every Check of the running gate that has not finished.
    pub async fn skip_checks(&self, job_id: &JobId) -> Result<Job, Adrift> {
        self.act_on_checks(job_id, |control, finished| control.skip_all(finished))
            .await
    }

    /// Run one Check of the running gate again, and only that one.
    pub async fn retry_check(&self, job_id: &JobId, check: &str) -> Result<Job, Adrift> {
        self.act_on_checks(job_id, |control, _| control.retry(check))
            .await
    }

    async fn act_on_checks(
        &self,
        job_id: &JobId,
        act: impl FnOnce(&Control, &[bool]) -> Result<(), Unactable>,
    ) -> Result<Job, Adrift> {
        self.load(job_id).await?;
        let refused = |why: Unactable| Adrift::NoCheckToAct {
            job: job_id.clone(),
            because: why.said(),
        };
        let (control, finished) = self
            .underway()
            .gate_of(&ipc::JobId::from(job_id))
            .ok_or_else(|| refused(Unactable::NoGate))?;
        act(&control, &finished).map_err(refused)?;
        self.load(job_id).await
    }
}
