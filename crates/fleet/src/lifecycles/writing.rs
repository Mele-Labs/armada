//! The writes a saved rule makes to a tracker and a forge: whether this Job's repository can make
//! each, and making it.
//!
//! **The one door.** A rule the owner saved counts as his say, so its planned act is the only thing
//! that may write to his tracker or forge on his behalf, and it does so at once under Acts and on his
//! Do it under Asks. Nothing else in Fleet reaches `Trackers::writer` or `request_reviewers`, which
//! `only_a_rules_act_reaches_the_trackers_writes` holds by name. Doing it is [`super::acting`]'s;
//! asking is `session_host::rule_asks`'s.
//!
//! **Asked before it is planned.** [`Writes::carries`] is pure, so an act the repository's tracker or
//! forge cannot make is kept as not carried, with the reason in a sentence, and raises nothing: the
//! owner is never asked to press what cannot be done.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Forge, IssueWrite, Merged, ReadOnly, TrackerWrites, Vcs, WorkProduct};
use core_model::rule::{Act, ForgeWrite, TrackerWrite};
use core_model::{Lifecycle, LifecycleState, StateRead};
use store::{KeptRuleAct, Store};

use super::keeping::{PULL_REQUEST, TICKET};
use super::saying::thing;
use crate::daemon::Fleet;
use crate::tracking::choice_of;

/// What one repository's tracker and forge can write, for a rule's acts to be planned against.
pub struct Writes {
    here: Result<(), String>,
    tracker: Arc<dyn TrackerWrites + Send + Sync>,
    reviewers: Result<(), String>,
}

impl Writes {
    pub fn new(tracker: Arc<dyn TrackerWrites + Send + Sync>, reviewers: Result<(), String>) -> Writes {
        Writes { here: Ok(()), tracker, reviewers }
    }

    /// Nothing to write to, for a Job in no repository this Fleet serves.
    pub fn nowhere(why: &str) -> Writes {
        Writes { here: Err(why.to_string()), tracker: Arc::new(ReadOnly), reviewers: Err(why.to_string()) }
    }

    /// Whether this Fleet does `act`, done to `on` (a lifecycle and a key), and in a sentence why
    /// not where it does not. `lifecycles` are the Job's own, for the tracker's word for a state.
    ///
    /// **A kind of act Fleet has no door for is `Err` too**, so the reason is kept with it.
    pub fn carries(&self, act: &Act, on: Option<(&str, &str)>, lifecycles: &[Lifecycle]) -> Result<(), String> {
        match act {
            Act::StartWatch { .. } | Act::Tell { .. } | Act::Wake | Act::StartJob { .. } | Act::Draft { .. } => Ok(()),
            Act::Tracker { write, .. } => {
                self.here.clone()?;
                self.tracker.can(&tracker_write(write, on, lifecycles)?.as_write())
            }
            Act::Forge { write, .. } => {
                self.here.clone()?;
                pull_number(on)?;
                match write {
                    ForgeWrite::RequestReviewers { .. } => self.reviewers.clone(),
                    ForgeWrite::MarkReady | ForgeWrite::Merge | ForgeWrite::AutoMerge => Ok(()),
                }
            }
            Act::RunCommand { .. } | Act::RunSkill { .. } | Act::RunDrone { .. } => {
                Err(String::from("Fleet does not do this kind of act"))
            }
        }
    }
}

/// A tracker write with the tracker's own word put in, owned so it can go to a blocking thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Resolved {
    Move(String),
    Assign(String),
    Close(Option<String>),
}

impl Resolved {
    pub(crate) fn as_write(&self) -> IssueWrite<'_> {
        match self {
            Resolved::Move(status) => IssueWrite::Move { status },
            Resolved::Assign(to) => IssueWrite::Assign { to },
            Resolved::Close(status) => IssueWrite::Close { status: status.as_deref() },
        }
    }
}

/// The first word the ticket lifecycle lists, in the tracker's own, for `state`.
fn first_word(state: &LifecycleState) -> Option<String> {
    match state.read() {
        StateRead::Tracker(words) => words.as_slice().first().cloned(),
        _ => None,
    }
}

/// The key of the ticket `on` names, or why it names none.
fn ticket<'a>(on: Option<(&'a str, &'a str)>) -> Result<&'a str, String> {
    match on {
        Some((TICKET, key)) => Ok(key),
        Some(_) => Err(String::from("a tracker write is done to a ticket, and this is not one")),
        None => Err(String::from("this tracker write names no ticket")),
    }
}

/// `write` as the tracker is asked to make it. **A move writes the first of the target state's
/// words**, which the Job's own ticket lifecycle gives, so the Workspace says what its tracker calls it.
pub(crate) fn tracker_write(write: &TrackerWrite, on: Option<(&str, &str)>, lifecycles: &[Lifecycle]) -> Result<Resolved, String> {
    ticket(on)?;
    let tickets = lifecycles.iter().find(|one| one.id().as_str() == TICKET);
    match write {
        TrackerWrite::Assign { to } => Ok(Resolved::Assign(to.as_str().to_string())),
        TrackerWrite::Move { to } => {
            let lifecycle = tickets.ok_or("the Job has no ticket lifecycle")?;
            let state = lifecycle.state(to).ok_or_else(|| format!("the ticket lifecycle has no state {}", to.as_str()))?;
            first_word(state)
                .map(Resolved::Move)
                .ok_or_else(|| format!("the ticket lifecycle lists no tracker word for {}", state.label().as_str()))
        }
        // The word for closed, where the lifecycle gives one: the first of an end state that lists any.
        TrackerWrite::Close => Ok(Resolved::Close(tickets.and_then(|lifecycle| {
            lifecycle.ends().iter().filter_map(|end| lifecycle.state(end)).find_map(first_word)
        }))),
    }
}

/// The number of the pull request `on` names. **Only a plain number**: a key that names another
/// repository's pull request would write to this one's.
fn pull_number<'a>(on: Option<(&'a str, &'a str)>) -> Result<&'a str, String> {
    match on {
        Some((PULL_REQUEST, key)) => {
            let number = key.trim_start_matches('#');
            match !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()) {
                true => Ok(number),
                false => Err(format!("`{key}` is not a pull request of this repository")),
            }
        }
        Some(_) => Err(String::from("a forge write is done to a pull request, and this is not one")),
        None => Err(String::from("this forge write names no pull request")),
    }
}

/// Make `write` to the ticket `on` names, through `tracker`, and say what was done. **Blocking**:
/// the tracker runs a program or makes a request.
pub fn tracker_written(
    tracker: &dyn TrackerWrites,
    in_repo: &str,
    write: &TrackerWrite,
    on: Option<(&str, &str)>,
    lifecycles: &[Lifecycle],
) -> Result<String, String> {
    let resolved = tracker_write(write, on, lifecycles)?;
    let reference = ticket(on)?;
    tracker.write(in_repo, reference, &resolved.as_write()).map_err(|not| not.said)?;
    Ok(match resolved {
        Resolved::Move(status) => format!("Moved {reference} to {status}"),
        Resolved::Assign(to) => format!("Assigned {reference} to {to}"),
        Resolved::Close(_) => format!("Closed {reference}"),
    })
}

/// Make `write` to the pull request `on` names, through `forge`, and say what was done. **Blocking**.
///
/// **A merge is `Forge::merge` and nothing else**: where the base has a merge queue the forge
/// enqueues it, and nothing here asks to skip the queue, squash, or push the base.
pub fn forge_written<F: Forge + ?Sized>(forge: &F, in_repo: &str, write: &ForgeWrite, on: Option<(&str, &str)>) -> Result<String, String> {
    let number = pull_number(on)?;
    let pull = thing(PULL_REQUEST, number);
    match write {
        ForgeWrite::MarkReady => forge.mark_ready(in_repo, number).map(|()| format!("Marked {pull} ready")),
        ForgeWrite::AutoMerge => forge.enable_auto_merge(in_repo, number).map(|()| format!("Turned on auto-merge for {pull}")),
        ForgeWrite::Merge => forge.merge(in_repo, number).map_err(|not| not.said()).map(|merged| match merged {
            Merged::Taken => format!("The forge took the merge of {pull}"),
            Merged::AlreadyMerged => format!("{pull} was already merged"),
        }),
        ForgeWrite::RequestReviewers { from } => {
            let names: Vec<&str> = from.iter().map(|one| one.as_str()).collect();
            forge.request_reviewers(in_repo, number, &names).map(|()| format!("Asked {} to review {pull}", names.join(", ")))
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
    /// What the repository `manifest` names can write: its tracker's and the forge's. Pure, and the
    /// same for every Job in it.
    pub(crate) fn writes_for(&self, manifest: Option<&str>) -> Writes {
        let Some(served) = manifest.and_then(|id| self.repositories().serving(id)) else {
            return Writes::nowhere("the Job is in no repository this Fleet serves");
        };
        Writes::new(self.trackers().writer(&choice_of(served.manifest())), self.vcs().can_request_reviewers())
    }

    /// Whether the repository the Job of `row` is in still carries `act`, and why not. Asked before
    /// each try, so an act is not tried against a tracker that can no longer make it.
    pub(super) fn rule_act_carried(&self, store: &Store, row: &KeptRuleAct, act: &Act, lifecycles: &[Lifecycle]) -> Result<(), String> {
        let manifest = store.session(&row.session_id).ok().flatten().and_then(|one| one.manifest_id);
        let on = row.on.as_ref().map(|(lifecycle, key)| (lifecycle.as_str(), key.as_str()));
        self.writes_for(manifest.as_deref()).carries(act, on, lifecycles)
    }

    /// The repository the Job `row` belongs to.
    async fn rule_repository(&self, row: &KeptRuleAct) -> Result<crate::repositories::Served, String> {
        let manifest = {
            let store = self.store().lock().await;
            store.session(&row.session_id).map_err(|why| why.to_string())?.and_then(|one| one.manifest_id)
        };
        manifest
            .as_deref()
            .and_then(|id| self.repositories().serving(id))
            .ok_or_else(|| String::from("the Job is in no repository this Fleet serves"))
    }

    /// Make the write to the ticket the act is done to, in the Job's repository's tracker.
    pub(super) async fn rule_tracker_written(
        self: &Arc<Self>,
        row: &KeptRuleAct,
        write: &TrackerWrite,
        lifecycles: &[Lifecycle],
    ) -> Result<String, String> {
        let served = self.rule_repository(row).await?;
        let (root, tracker) = (served.root().to_string(), self.trackers().writer(&choice_of(served.manifest())));
        let (write, lifecycles, on) = (write.clone(), lifecycles.to_vec(), row.on.clone());
        tokio::task::spawn_blocking(move || {
            let on = on.as_ref().map(|(lifecycle, key)| (lifecycle.as_str(), key.as_str()));
            tracker_written(&*tracker, &root, &write, on, &lifecycles)
        })
        .await
        .map_err(|why| why.to_string())?
    }

    /// Make the write to the pull request the act is done to, through the forge Fleet holds.
    pub(super) async fn rule_forge_written(self: &Arc<Self>, row: &KeptRuleAct, write: &ForgeWrite) -> Result<String, String> {
        let root = self.rule_repository(row).await?.root().to_string();
        let (write, on) = (write.clone(), row.on.clone());
        self.forge_asked(&root, move |vcs: &V, root: &str| {
            let on = on.as_ref().map(|(lifecycle, key)| (lifecycle.as_str(), key.as_str()));
            forge_written(vcs, root, &write, on)
        })
        .await
    }
}
