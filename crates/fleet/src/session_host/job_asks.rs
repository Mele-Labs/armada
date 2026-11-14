//! `raise_conflict` and `ask_held`: the two things a Job may send the Operator. Fleet checks every part, answers
//! what it already keeps, and hands the rest on. **While the Operator is on it is handed the ask**; while it is off
//! the ask stands on the sending Job's Needs you, as `job_conflict` or `job_held`, and his press on it tells the
//! Jobs it names. Derived on every read from the kept ask, so a restart keeps it. Nothing is written to a forge.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, PullRequestStanding as Forge, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    AskHeld, ConflictRaised, ForgeChecks, HeldAsked, HeldChecks, Instant, ManifestId, Notice, RaiseConflict, WaitingAct,
    WaitingActKind, WaitingItem, WaitingOption, WaitingSource,
};
use store::{job_ask_kind as kind, JobAskPut, KeptJobAsk};

use super::address_of;
use super::message_reach::Unreachable;
use super::waiting::SESSION_WAITING_EMPTY;
use crate::daemon::Fleet;

const CONFLICT_REFUSED: &str = "fleet.conflict_refused";
const HELD_REFUSED: &str = "fleet.held_refused";

/// What a conflict's item id begins with, then the two addresses sorted.
pub(crate) const CONFLICT: &str = "conflict:";
/// What a landing question's item id begins with, then the repository and the pull request or branch.
pub(crate) const HELD: &str = "held:";

/// The choices of a `job_conflict` item, in the order they are drawn.
pub(crate) const TELL_SENDER: u32 = 0;
pub(crate) const TELL_OTHER: u32 = 1;
pub(crate) const HOLD_BOTH: u32 = 2;
/// The choices of a `job_held` item.
pub(crate) const LAND: u32 = 0;
pub(crate) const HOLD: u32 = 1;

/// What is told the Jobs of an ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Said {
    /// The Job that raised the conflict changes its side.
    ChangeSender,
    /// The other Job changes its side.
    ChangeOther,
    /// Both hold landing.
    HoldBoth,
    Land,
    Hold,
}

impl Said {
    fn word(self) -> &'static str {
        match self {
            Said::ChangeSender => "change_sender",
            Said::ChangeOther => "change_other",
            Said::HoldBoth => "hold_both",
            Said::Land => "land",
            Said::Hold => "hold",
        }
    }
}

/// Whose word it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Voice {
    /// His press.
    Person,
    /// The Operator's ruling, and why it ruled so.
    Operator(String),
}

fn title_or_address(title: &str, session: &str) -> String {
    match title.trim() {
        "" => address_of(session),
        title => title.to_string(),
    }
}

fn option(label: String) -> WaitingOption {
    WaitingOption { label, description: None, picture: None }
}

/// The items for the asks standing on a Job: the conflicts it raised and the landings it asked about.
pub(crate) fn job_ask_items(asks: &[KeptJobAsk]) -> Vec<WaitingItem> {
    asks.iter().filter_map(job_ask_item).collect()
}

fn job_ask_item(ask: &KeptJobAsk) -> Option<WaitingItem> {
    let (source, text, options) = match ask.kind.as_str() {
        kind::CONFLICT => {
            let other = ask.other_session_id.as_deref()?;
            let (mine, theirs) = (title_or_address(&ask.job_title, &ask.session_id), title_or_address(ask.other_title.as_deref().unwrap_or_default(), other));
            (WaitingSource::JobConflict, ask.breaks.clone(), vec![option(format!("Tell {mine} to change")), option(format!("Tell {theirs}")), option(String::from("Hold both"))])
        }
        kind::HELD => {
            let what = match (ask.pull, &ask.branch) {
                (Some(pull), _) => format!("#{pull}"),
                (None, Some(branch)) => branch.clone(),
                (None, None) => return None,
            };
            (WaitingSource::JobHeld, format!("Should {what} land?"), vec![option(String::from("Land it")), option(String::from("Hold it"))])
        }
        _ => return None,
    };
    Some(WaitingItem {
        approves: false,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: ask.id.clone() }),
        id: ask.id.clone(),
        text,
        since: Instant::carried(&ask.at),
        source,
        options,
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
    })
}

/// A Job on a card of Your turn: its title and its address.
fn card_job(session: &str, title: &str) -> ipc::OperatorAskJob {
    ipc::OperatorAskJob { session_id: ipc::SessionId::carried(session), address: address_of(session), title: title.trim().to_string() }
}

/// The conflict as Your turn's card draws it. Absent for any other ask.
pub(crate) fn card_conflict(ask: &KeptJobAsk) -> Option<ipc::OperatorConflict> {
    let other = ask.other_session_id.as_deref().filter(|_| ask.kind == kind::CONFLICT)?;
    Some(ipc::OperatorConflict {
        raised_by: card_job(&ask.session_id, &ask.job_title),
        with: card_job(other, ask.other_title.as_deref().unwrap_or_default()),
        files: ask.files.clone(),
        wire: ask.wire.clone(),
        mine: ask.mine.clone(),
        theirs: ask.theirs.clone(),
        breaks: ask.breaks.clone(),
    })
}

/// The landing question as Your turn's card draws it. Absent for any other ask.
pub(crate) fn card_held(ask: &KeptJobAsk) -> Option<ipc::OperatorHeld> {
    (ask.kind == kind::HELD).then(|| ipc::OperatorHeld {
        pull: ask.pull,
        branch: ask.branch.clone(),
        title: ask.title.clone(),
        checks: ask.checks.as_deref().and_then(|word| match word {
            "passed" => Some(HeldChecks::Passed),
            "pending" => Some(HeldChecks::Pending),
            "failed" => Some(HeldChecks::Failed),
            _ => None,
        }),
        asked_by: card_job(&ask.session_id, &ask.job_title),
    })
}

/// The lines a call sent, each trimmed, with the blank ones left out.
fn lines(sent: Vec<String>) -> Vec<String> {
    sent.into_iter().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect()
}

/// A forge's checks as the card reads them: only a required check that failed is a failure.
fn checks_of(checks: &ForgeChecks) -> HeldChecks {
    match checks {
        ForgeChecks::Passed => HeldChecks::Passed,
        ForgeChecks::Pending => HeldChecks::Pending,
        ForgeChecks::Failed { weight: ipc::FailureWeight::Required, .. } => HeldChecks::Failed,
        ForgeChecks::Failed { weight: ipc::FailureWeight::WhileRunning, .. } => HeldChecks::Pending,
        ForgeChecks::Failed { weight: ipc::FailureWeight::AfterRequired, .. } => HeldChecks::Passed,
    }
}

fn checks_word(checks: HeldChecks) -> &'static str {
    match checks {
        HeldChecks::Passed => "passed",
        HeldChecks::Pending => "pending",
        HeldChecks::Failed => "failed",
    }
}

/// What a conflict's two Jobs are told, from the point of view of the one told.
fn conflict_told(ask: &KeptJobAsk, to_sender: bool, said: Said, voice: &Voice) -> String {
    let (peer, peer_session) = match to_sender {
        true => (ask.other_title.as_deref().unwrap_or_default(), ask.other_session_id.as_deref().unwrap_or_default()),
        false => (ask.job_title.as_str(), ask.session_id.as_str()),
    };
    let (mine, theirs) = if to_sender { (&ask.mine, &ask.theirs) } else { (&ask.theirs, &ask.mine) };
    let he = match voice {
        Voice::Person => "He",
        Voice::Operator(_) => "The Operator",
    };
    let ruling = match said {
        Said::HoldBoth => format!("hold landing until {} rules", if *voice == Voice::Person { "he" } else { "the person" }),
        _ => String::from("your side changes"),
    };
    let mut text = format!(
        "Conflict with {} ({}): {}\nYours: {mine}\nTheirs: {theirs}\n{he} says {ruling}.",
        title_or_address(peer, peer_session),
        address_of(peer_session),
        ask.breaks
    );
    if let Voice::Operator(because) = voice {
        text.push(' ');
        text.push_str(because.trim());
    }
    text
}

fn held_told(ask: &KeptJobAsk, said: Said, voice: &Voice) -> String {
    let what = match (ask.pull, &ask.branch) {
        (Some(pull), _) => format!("#{pull}"),
        (None, branch) => branch.clone().unwrap_or_default(),
    };
    let he = if *voice == Voice::Person { "He" } else { "The Operator" };
    let verdict = if said == Said::Land { "land it" } else { "hold it" };
    let mut text = format!("held? {what}: {he} says {verdict}.");
    if let Voice::Operator(because) = voice {
        text.push(' ');
        text.push_str(because.trim());
    }
    text
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
    /// The live Job a call is made from, and the repository it is in: refused where it has ended or has none.
    async fn asking_job(&self, caller: Option<api::Caller>, named: Option<&ipc::SessionId>, code: &'static str) -> Result<(String, String, String), Refusal> {
        let session = self.waiting_session(caller, named)?;
        let store = self.store().lock().await;
        let kept = store.session(&session).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.hosted_refusal(code, "no Job Fleet keeps is making this call"))?;
        if kept.state == store::SessionState::Ended {
            return Err(self.closed(&session));
        }
        let manifest = store.hosting(&session).map_err(|why| self.ledger_fault(why))?.map(|hosting| hosting.manifest_id).filter(|id| !id.is_empty());
        let manifest = manifest.ok_or_else(|| self.hosted_refusal(code, "only a Job Fleet hosts in a repository may call this"))?;
        Ok((session, kept.title.unwrap_or_default(), manifest))
    }

    /// `raise_conflict`. **Every part is checked and the first that fails is named.** An open conflict between the
    /// same two Jobs takes this one as "also raised by", and nothing new is shown.
    pub(crate) async fn conflict_raised(self: &Arc<Self>, caller: Option<api::Caller>, conflict: RaiseConflict) -> Result<ConflictRaised, Refusal> {
        let (session, title, manifest) = self.asking_job(caller, conflict.session_id.as_ref(), CONFLICT_REFUSED).await?;
        let refuse = |why: &str| self.hosted_refusal(CONFLICT_REFUSED, why);
        let with = conflict.with.trim().to_string();
        let sender = address_of(&session);
        if with == sender {
            return Err(refuse("`with` is this Job's own address. Name the other Job"));
        }
        if !with.starts_with("s-") {
            return Err(refuse(&format!("`with` is {with}, which is no Job's address. Give the other Job's address, `s-` and eight characters")));
        }
        match self.unreachable(&with).await? {
            Some(Unreachable::Ended) => return Err(refuse(&format!("{with} has ended, so its work cannot clash with yours"))),
            Some(Unreachable::Unknown) => return Err(refuse(&format!("no Job has the address {with}. ListAgents names the live ones"))),
            None => {}
        }
        let (files, wire) = (lines(conflict.files), lines(conflict.wire));
        if files.is_empty() && wire.is_empty() {
            return Err(refuse("`files` and `wire` are both empty. Name the paths or the wire types the clash is in"));
        }
        let (mine, theirs, breaks) = (conflict.mine.trim(), conflict.theirs.trim(), conflict.breaks.trim());
        for (name, said, what) in [("mine", mine, "what your work does"), ("theirs", theirs, "what the other Job's work does"), ("breaks", breaks, "what fails if both land")] {
            if said.is_empty() {
                return Err(refuse(&format!("`{name}` is empty. Say in one sentence {what}")));
            }
        }
        let (other, other_title) = {
            let store = self.store().lock().await;
            let named = store.sessions_beginning(&with[2..]).map_err(|why| self.ledger_fault(why))?;
            let live = named.into_iter().find(|one| address_of(&one.id) == with && one.state != store::SessionState::Ended);
            let live = live.ok_or_else(|| refuse(&format!("no live Job has the address {with}")))?;
            (live.id, live.title.unwrap_or_default())
        };
        let mut pair = [sender.clone(), with.clone()];
        pair.sort();
        let id = format!("{CONFLICT}{}:{}", pair[0], pair[1]);
        let ask = KeptJobAsk {
            id: id.clone(),
            round: 0,
            kind: kind::CONFLICT.to_string(),
            session_id: session.clone(),
            job_title: title.clone(),
            other_session_id: Some(other),
            other_title: Some(other_title),
            manifest_id: manifest,
            files,
            wire,
            mine: mine.to_string(),
            theirs: theirs.to_string(),
            breaks: breaks.to_string(),
            pull: None,
            branch: None,
            title: None,
            checks: None,
            also_raised_by: Vec::new(),
            raised: !self.operator_on(),
            at: self.now().as_str().to_string(),
            state: store::job_ask_standing::OPEN.to_string(),
            answered_at: None,
            answer: None,
        };
        let also = format!("{} ({sender})", title_or_address(&title, &session));
        let put = self.store().lock().await.put_job_ask(&ask, &also).map_err(|why| self.ledger_fault(why))?;
        Ok(ConflictRaised { id: id.clone(), folded: self.job_ask_put(&session, put, ask.raised).await })
    }

    /// What follows a kept ask: an open one that took it changes nothing shown, and a new one is on the sender's
    /// Needs you or handed to the Operator. Whether it was folded.
    async fn job_ask_put(&self, session: &str, put: JobAskPut, raised: bool) -> bool {
        match put {
            JobAskPut::Folded => {
                // Where it stands on Needs you or in Your turn, the page reads "also raised by" again.
                self.events().publish(ipc::Event::OperatorPageChanged(ipc::operator_page::OperatorPageChanged {}));
                true
            }
            JobAskPut::Opened(_) => {
                // Where the Operator is on, the next tick hands it the ask.
                if raised {
                    let _ = self.published_hosted(session).await;
                }
                false
            }
        }
    }

    /// `ask_held`. Answered by Fleet, with no call, where the pull request merged or closed.
    pub(crate) async fn held_asked(self: &Arc<Self>, caller: Option<api::Caller>, asked: AskHeld) -> Result<HeldAsked, Refusal> {
        let (session, title, manifest) = self.asking_job(caller, asked.session_id.as_ref(), HELD_REFUSED).await?;
        let refuse = |why: &str| self.hosted_refusal(HELD_REFUSED, why);
        let branch = asked.branch.as_deref().map(str::trim).filter(|branch| !branch.is_empty());
        let (pull, branch) = match (asked.pull, branch) {
            (Some(pull), None) => (Some(pull), None),
            (None, Some(branch)) => (None, Some(branch.to_string())),
            (None, None) => return Err(refuse("neither `pull` nor `branch` is given. Name the pull request or the branch")),
            (Some(_), Some(_)) => return Err(refuse("both `pull` and `branch` are given. Name one")),
        };
        let manifest_id = ManifestId::carried(manifest.clone());
        let served = self.served_named(Some(&manifest_id)).map_err(|_| refuse(&format!("{manifest} is no repository Fleet serves")))?;
        let root = served.root().to_string();
        let (mut shown, mut checks) = (None, None);
        if let Some(pull) = pull {
            let (vcs, read_root, named) = (Arc::clone(self.vcs()), root.clone(), pull.to_string());
            let facts = tokio::task::spawn_blocking(move || vcs.pull_request_facts(&read_root, &named)).await.ok().flatten();
            let facts = facts.ok_or_else(|| refuse(&format!("the repository has no pull request #{pull} Fleet can read")))?;
            match facts.standing {
                Forge::Merged => {
                    let at = self.store().lock().await.pull_merged_at(&manifest, pull).map_err(|why| self.ledger_fault(why))?;
                    let said = match self.clock_of(at.as_deref()).await {
                        Some(time) => format!("held? #{pull}: merged at {time}."),
                        None => format!("held? #{pull}: merged."),
                    };
                    return self.held_answered(&session, &manifest, pull, said).await;
                }
                Forge::Closed => return self.held_answered(&session, &manifest, pull, format!("held? #{pull}: closed.")).await,
                Forge::Open | Forge::Draft => {
                    shown = Some(facts.title);
                    // Its checks, where the forge says: a card without them is still a card.
                    checks = self.pull_request_read_off(&manifest_id, &root, pull).await.ok().map(|state| checks_of(&state.checks));
                }
            }
        }
        if let Some(branch) = &branch {
            let (vcs, base, root) = (Arc::clone(self.vcs()), served.manifest().base().map(str::to_string), root.clone());
            let held = tokio::task::spawn_blocking(move || vcs.branches(&root, base.as_deref()).ok().map(|listed| listed.into_iter().map(|one| one.name).collect::<Vec<_>>()))
                .await
                .ok()
                .flatten();
            if !held.is_some_and(|names| names.iter().any(|name| name == branch)) {
                return Err(refuse(&format!("the repository has no local branch {branch}")));
            }
        }
        let what = pull.map_or_else(|| branch.clone().unwrap_or_default(), |pull| pull.to_string());
        let id = format!("{HELD}{manifest}:{what}");
        let ask = KeptJobAsk {
            id: id.clone(),
            round: 0,
            kind: kind::HELD.to_string(),
            session_id: session.clone(),
            job_title: title.clone(),
            other_session_id: None,
            other_title: None,
            manifest_id: manifest,
            files: Vec::new(),
            wire: Vec::new(),
            mine: String::new(),
            theirs: String::new(),
            breaks: String::new(),
            pull,
            branch,
            title: shown,
            checks: checks.map(|checks| checks_word(checks).to_string()),
            also_raised_by: Vec::new(),
            raised: !self.operator_on(),
            at: self.now().as_str().to_string(),
            state: store::job_ask_standing::OPEN.to_string(),
            answered_at: None,
            answer: None,
        };
        let also = format!("{} ({})", title_or_address(&title, &session), address_of(&session));
        let put = self.store().lock().await.put_job_ask(&ask, &also).map_err(|why| self.ledger_fault(why))?;
        self.job_ask_put(&session, put, ask.raised).await;
        Ok(HeldAsked { id, answer: None })
    }

    /// Fleet's own answer to a landing question: the Job is sent it, and it is the call's answer too. Nothing is kept.
    async fn held_answered(self: &Arc<Self>, session: &str, manifest: &str, pull: u64, said: String) -> Result<HeldAsked, Refusal> {
        let _ = self.told_by_fleet(session, &said, Notice::Operator).await;
        Ok(HeldAsked { id: format!("{HELD}{manifest}:{pull}"), answer: Some(said) })
    }

    /// `at` as `HH:MM` on this machine's clock.
    async fn clock_of(&self, at: Option<&str>) -> Option<String> {
        let millis = core_model::Timestamp::from_rfc3339(at?).epoch_millis()?;
        let reading = crate::clock::local_reading(millis).await;
        Some(format!("{:02}:{:02}", reading.hour, reading.minute))
    }

    /// Put the open ask `id` on its sender's Needs you, where it was not: the Operator raised it, or it ruled nothing.
    /// Whether it moved.
    pub(crate) async fn job_ask_raised(&self, id: &str) -> bool {
        let moved = {
            let mut store = self.store().lock().await;
            let moved = store.raise_job_ask(id).unwrap_or(false);
            moved.then(|| store.open_job_ask(id).ok().flatten()).flatten()
        };
        let Some(ask) = moved else { return false };
        let _ = self.published_hosted(&ask.session_id).await;
        true
    }

    /// `answer_waiting` on a `job_conflict` or `job_held` item: his press tells the Jobs it names, and the item goes.
    pub(crate) async fn job_ask_chosen(self: &Arc<Self>, session: &str, item: &str, choice: Option<u32>) -> Result<ipc::SessionRecord, Refusal> {
        let ask = self.store().lock().await.open_job_ask(item).map_err(|why| self.ledger_fault(why))?;
        // The item must be this Job's: an id from another Job's list settles nothing here.
        let ask = ask.filter(|ask| ask.session_id == session).ok_or_else(|| self.settled_already())?;
        let said = match (ask.kind.as_str(), choice) {
            (kind::CONFLICT, Some(TELL_SENDER)) => Said::ChangeSender,
            (kind::CONFLICT, Some(TELL_OTHER)) => Said::ChangeOther,
            (kind::CONFLICT, Some(HOLD_BOTH)) => Said::HoldBoth,
            (kind::HELD, Some(LAND)) => Said::Land,
            (kind::HELD, Some(HOLD)) => Said::Hold,
            _ => return Err(self.hosted_refusal(SESSION_WAITING_EMPTY, "that is answered by choosing one of its buttons")),
        };
        self.job_ask_settled(&ask, said, Voice::Person).await.map_err(|_| self.settled_already())?;
        self.published_hosted(session).await
    }

    /// The open ask answered with `said`, and the Jobs it names told. **The first answer wins**: `Err` where it was
    /// answered already, or `said` does not fit what was asked.
    pub(crate) async fn job_ask_settled(self: &Arc<Self>, ask: &KeptJobAsk, said: Said, voice: Voice) -> Result<(), String> {
        let fits = match said {
            Said::Land | Said::Hold => ask.kind == kind::HELD,
            _ => ask.kind == kind::CONFLICT,
        };
        if !fits {
            return Err("that answer is not one of its choices".to_string());
        }
        let at = self.now().as_str().to_string();
        let moved = self.store().lock().await.answer_job_ask(&ask.id, said.word(), &at).map_err(|why| why.to_string())?;
        if !moved {
            return Err("it was answered already".to_string());
        }
        let other = ask.other_session_id.clone().unwrap_or_default();
        let told: Vec<(String, String)> = match said {
            Said::ChangeSender => vec![(ask.session_id.clone(), conflict_told(ask, true, said, &voice))],
            Said::ChangeOther => vec![(other, conflict_told(ask, false, said, &voice))],
            Said::HoldBoth => vec![(ask.session_id.clone(), conflict_told(ask, true, said, &voice)), (other, conflict_told(ask, false, said, &voice))],
            Said::Land | Said::Hold => vec![(ask.session_id.clone(), held_told(ask, said, &voice))],
        };
        for (job, text) in told {
            // A Job that has ended, or that muted the notice, is told nothing: the answer stands.
            let _ = self.told_by_fleet(&job, &text, Notice::Operator).await;
        }
        let _ = self.published_hosted(&ask.session_id).await;
        self.events().publish(ipc::Event::OperatorPageChanged(ipc::operator_page::OperatorPageChanged {}));
        Ok(())
    }
}
