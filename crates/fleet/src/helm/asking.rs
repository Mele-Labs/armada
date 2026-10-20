//! Every Helm call held open while a person answers it, and the answer going
//! back inside the call. `#1389`.
//!
//! **Fleet decides nothing here.** The person's own settings already allowed
//! what they allow and refused what they refuse; what reaches this module is
//! the remainder, and the only thing it does with one is carry it to the dock
//! and carry an answer back. There is no policy to read and none to write.
//!
//! **Held on the table and written to no column**, for
//! `crate::permitting::Waiting`'s reason one subject over: what is waiting is
//! one process inside one tool call, and a Fleet that restarts has lost the
//! process whose call it was. A restart leaves nothing to resume.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use adapter_traits::CallDetail;
use core_model::Timestamp;
use ipc::{
    AnswerHelmCall, AskingToRun, HelmCallAnswer, HelmCallInFlight, Instant, ManifestId,
    QuestionAnswer, ReachesTo, RunOrNot,
};
use tokio::sync::oneshot;

/// How long one ask is held open for a person before Fleet answers `deny` for
/// them. The shipped value is the *Helm ask hold* setting, which the
/// composition root resolves and writes out.
///
/// **A fitting rather than a bare constant**, for `PermissionHold`'s reason: a
/// test can plant a bound it can outlive, where five real minutes is not a wait
/// any case can make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelmAskHold(Duration);

impl HelmAskHold {
    pub const fn of(hold: Duration) -> HelmAskHold {
        HelmAskHold(hold)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// What a person said, as it travels from the route to the call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub answer: HelmCallAnswer,
    /// Their words, on a refusal. Trimmed away where they typed nothing.
    pub note: Option<String>,
    /// What was chosen, for a call that asked questions.
    pub answers: Vec<QuestionAnswer>,
    /// The folder an "always" answer is for.
    pub folder: Option<String>,
}

impl Said {
    pub fn of(said: &AnswerHelmCall) -> Said {
        Said {
            answer: said.answer,
            note: said
                .note
                .as_deref()
                .map(str::trim)
                .filter(|note| !note.is_empty())
                .map(str::to_string),
            answers: said.answers.clone(),
            folder: said.folder.clone(),
        }
    }
}

/// Why an answer did not land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnswerable {
    /// No call by that id is waiting. An answer from a window left open across
    /// one, or a second press.
    NothingWaiting,
    /// An answer outside what that call offered.
    NotOffered,
    /// A question with no answer, or an answer to one that was not asked. The
    /// ask stays on the table.
    Incomplete,
    /// An "always" answer naming no folder, or one that does not hold the path asked about. The
    /// ask stays on the table.
    NotAFolderAbove,
}

/// One ask, and the half of the channel its answer goes down.
struct Held {
    in_flight: HelmCallInFlight,
    answer: oneshot::Sender<Said>,
}

/// Every Helm call waiting on a person right now, across every repository.
///
/// **One table for the machine, not one per conversation.** The dock is
/// Fleet-wide and a person answers what is waiting on them wherever it came
/// from — which is how `#935` already gathers a Job's questions.
pub struct Asks {
    waiting: Mutex<BTreeMap<String, Held>>,
    minted: AtomicU64,
    hold: HelmAskHold,
    /// Fleet is stopping: every call held is let go, and none is held again.
    stopping: AtomicBool,
}

impl Asks {
    pub fn holding_for(hold: HelmAskHold) -> Asks {
        Asks {
            waiting: Mutex::new(BTreeMap::new()),
            minted: AtomicU64::new(0),
            hold,
            stopping: AtomicBool::new(false),
        }
    }

    /// Fleet is stopping: every call on the table is let go unanswered, and one minted after
    /// this is let go at once. **Its waiter reads [`Asks::stopping`] and answers [`restarted`]**,
    /// because graceful shutdown waits on a held call and a Fleet killed over one writes nothing.
    pub fn all_withdrawn(&self) {
        if let Ok(mut waiting) = self.waiting.lock() {
            self.stopping.store(true, Ordering::SeqCst);
            waiting.clear();
        }
    }

    /// Whether a call let go unanswered was let go because Fleet is stopping.
    pub fn stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }

    pub fn hold(&self) -> Duration {
        self.hold.duration()
    }

    pub fn held_for(&self) -> HelmAskHold {
        self.hold
    }

    /// Put one call on the table and hand back what a person will be shown and
    /// the channel their answer arrives on.
    ///
    /// **The id is Fleet's, not the harness's.** `tool_use_id` is the session's
    /// own and is absent on some calls; an answer naming an id Fleet minted is
    /// an answer to the ask a person actually read.
    pub fn minted(
        &self,
        manifest_id: &ManifestId,
        asking: &AskingToRun,
        at: &Timestamp,
    ) -> (HelmCallInFlight, oneshot::Receiver<Said>) {
        self.minted_reaching(manifest_id, asking, at, None)
    }

    /// [`Asks::minted`] for a call that reaches past a session's slots (`Some`), and whether it
    /// reads or writes there: its card offers the folder answers where the guard named the path,
    /// for it or a folder above it, and only the two "once" answers where it named none or the root.
    pub fn minted_reaching(
        &self,
        manifest_id: &ManifestId,
        asking: &AskingToRun,
        at: &Timestamp,
        reach: Option<(Option<String>, ReachesTo)>,
    ) -> (HelmCallInFlight, oneshot::Receiver<Said>) {
        let reaches = reach.clone().and_then(|(path, _)| path);
        let reaches_to = reach.as_ref().map(|(_, to)| *to);
        let questions = asking.questions();
        let wording = asking
            .detail()
            .or_else(|| questions.first().map(|first| first.question.as_str()))
            .unwrap_or_default();
        let detail = CallDetail::of(wording.trim());
        let call = format!(
            "helm-{}",
            self.minted.fetch_add(1, Ordering::SeqCst).wrapping_add(1)
        );
        let in_flight = HelmCallInFlight {
            call: call.clone(),
            manifest_id: manifest_id.clone(),
            asked_at: Instant::from(at),
            tool: asking.tool_name.clone(),
            detail: detail.shown().to_string(),
            truncated: detail.truncated(),
            length: Some(detail.length()),
            whole: detail.whole().map(String::from),
            rule: rule_for(&asking.tool_name, detail.whole().unwrap_or(detail.shown())),
            // The root names no folder an "always" could take, so `find /` is asked once only.
            offers: match &reach {
                Some((Some(path), _)) if path.split('/').any(|part| !part.is_empty()) => {
                    REACHING_OFFERS.to_vec()
                }
                Some(_) => REACHING_OFFERS[..2].to_vec(),
                None => offers_for(&asking.tool_name),
            },
            holding_for_seconds: self.hold().as_secs(),
            questions,
            reaches,
            reaches_to,
        };
        let (sender, receiver) = oneshot::channel();
        // Read under the lock `all_withdrawn` takes, so a call minted as Fleet stops is never held.
        let held = self.waiting.lock().ok().filter(|_| !self.stopping());
        if let Some(mut waiting) = held {
            waiting.insert(
                call,
                Held {
                    in_flight: in_flight.clone(),
                    answer: sender,
                },
            );
        }
        (in_flight, receiver)
    }

    /// A person's answer. **Takes the ask off the table** — a second press
    /// finds nothing waiting, which is what makes it a refusal rather than a
    /// second answer to the same call.
    pub fn answer(&self, said: &AnswerHelmCall) -> Result<HelmCallInFlight, NotAnswerable> {
        let mut waiting = self
            .waiting
            .lock()
            .map_err(|_| NotAnswerable::NothingWaiting)?;
        let held = waiting
            .get(&said.call)
            .ok_or(NotAnswerable::NothingWaiting)?;
        if !held.in_flight.offers.contains(&said.answer) {
            return Err(NotAnswerable::NotOffered);
        }
        if said.answer == HelmCallAnswer::AllowOnce
            && !answers_every(&held.in_flight, &said.answers)
        {
            return Err(NotAnswerable::Incomplete);
        }
        let always = matches!(
            said.answer,
            HelmCallAnswer::AlwaysAllow | HelmCallAnswer::AlwaysBlock | HelmCallAnswer::AllowForSession
        );
        if always && !folder_above(said.folder.as_deref(), held.in_flight.reaches.as_deref()) {
            return Err(NotAnswerable::NotAFolderAbove);
        }
        let held = waiting
            .remove(&said.call)
            .ok_or(NotAnswerable::NothingWaiting)?;
        let in_flight = held.in_flight.clone();
        // The receiver is gone where the hold already ran out. The ask is off
        // the table either way, and the call has answered itself.
        let _ = held.answer.send(Said::of(said));
        Ok(in_flight)
    }

    /// Put back an ask a restarted Fleet kept, under the id it had. Nothing
    /// waits on its channel: the answer is kept by whoever answers.
    pub fn restore(&self, in_flight: HelmCallInFlight) {
        if let Some(minted) = in_flight
            .call
            .strip_prefix("helm-")
            .and_then(|n| n.parse::<u64>().ok())
        {
            self.minted.fetch_max(minted, Ordering::SeqCst);
        }
        let (sender, _) = oneshot::channel();
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.insert(
                in_flight.call.clone(),
                Held {
                    in_flight,
                    answer: sender,
                },
            );
        }
    }

    /// Take one ask off the table without answering it — the hold running out,
    /// or the session going. `None` where a person got there first.
    pub fn withdraw(&self, call: &str) -> Option<HelmCallInFlight> {
        let mut waiting = self.waiting.lock().ok()?;
        waiting.remove(call).map(|held| held.in_flight)
    }

    /// Every ask waiting, oldest first — what `list_helm_calls` answers.
    pub fn waiting(&self) -> Vec<HelmCallInFlight> {
        let Ok(waiting) = self.waiting.lock() else {
            return Vec::new();
        };
        let mut asks: Vec<HelmCallInFlight> = waiting
            .values()
            .map(|held| held.in_flight.clone())
            .collect();
        asks.sort_by(|one, two| one.asked_at.cmp(&two.asked_at));
        asks
    }
}

/// What a card over a reach past a session's slots offers. **No `AllowAndRemember`**: its rule
/// matches a command, not a folder, and the guard never reads it.
const REACHING_OFFERS: &[HelmCallAnswer] = &[
    HelmCallAnswer::AllowOnce,
    HelmCallAnswer::Refuse,
    HelmCallAnswer::AlwaysAllow,
    HelmCallAnswer::AllowForSession,
    HelmCallAnswer::AlwaysBlock,
];

/// Whether `folder` is `reaches` or a folder above it, compared a part at a time. The root holds
/// everything and is never offered, so it is never taken.
fn folder_above(folder: Option<&str>, reaches: Option<&str>) -> bool {
    let (Some(folder), Some(reaches)) = (folder, reaches) else {
        return false;
    };
    let parts = |path: &str| -> Vec<String> {
        path.split('/').filter(|part| !part.is_empty()).map(str::to_string).collect()
    };
    let (folder, reaches) = (parts(folder), parts(reaches));
    !folder.is_empty()
        && !folder.iter().any(|part| part == "." || part == "..")
        && reaches.starts_with(&folder)
}

/// What a person may answer about one call, in the order a surface draws them.
///
/// **A rule is offered only where one can be written.** `allow_and_remember`
/// writes a settings rule, and a tool whose rule would be its bare name is
/// still a rule the CLI honours — so every tool offers all three, and the
/// narrowing that a Drone's `offers` does by policy has no equivalent here.
fn offers_for(tool: &str) -> Vec<HelmCallAnswer> {
    // A question is answered or skipped. There is no rule to remember.
    if tool == ipc::ASKS_A_QUESTION {
        return vec![HelmCallAnswer::AllowOnce, HelmCallAnswer::Refuse];
    }
    vec![
        HelmCallAnswer::AllowOnce,
        HelmCallAnswer::AllowAndRemember,
        HelmCallAnswer::Refuse,
    ]
}

/// Whether `answers` has an entry, with something chosen, for each question the
/// call asked and for no other. A call with no questions takes none.
fn answers_every(asked: &HelmCallInFlight, answers: &[QuestionAnswer]) -> bool {
    answers.len() == asked.questions.len()
        && asked.questions.iter().all(|question| {
            answers.iter().any(|one| {
                one.question == question.question
                    && one.chosen.iter().any(|chosen| !chosen.trim().is_empty())
            })
        })
}

/// The rule that would have to allow this call, in the CLI's own settings
/// spelling.
///
/// **The leading cut of a shell line, never the whole of one.** A rule holding
/// a whole command with its arguments would match that command and nothing
/// else, which is a rule a person would have to add again tomorrow;
/// `crate::allowed::always_allow_rules` already decides where a command
/// stops being a subcommand, and this is that answer in the CLI's syntax rather
/// than a second one.
pub fn rule_for(tool: &str, detail: &str) -> String {
    if tool != "Bash" {
        return tool.to_string();
    }
    match crate::allowed::always_allow_rules(detail).1 {
        Some(prefix) => format!("Bash({prefix}:*)"),
        // A line that chains, or starts with an operator-bearing word. There is
        // no safe prefix of it, so the rule is the command as it stands.
        None => format!("Bash({})", detail.trim()),
    }
}

/// What the session is told, out of what a person said.
///
/// **A refusal reaches the model and is written for one.** Spike 19: the deny
/// message arrives as the tool result with `is_error`, in place of what the
/// call would have returned, and it is the only thing the session learns about
/// why.
pub fn answering(asking: &AskingToRun, said: &Said) -> RunOrNot {
    match said.answer {
        HelmCallAnswer::AllowOnce if asking.tool_name == ipc::ASKS_A_QUESTION => RunOrNot::Allow {
            updated_input: asking.answered(&said.answers),
        },
        HelmCallAnswer::AllowOnce
        | HelmCallAnswer::AllowAndRemember
        | HelmCallAnswer::AlwaysAllow
        | HelmCallAnswer::AllowForSession => {
            RunOrNot::Allow {
                updated_input: asking.input.clone(),
            }
        }
        HelmCallAnswer::AlwaysBlock => RunOrNot::Deny {
            message: blocked_folder(said.folder.as_deref().unwrap_or_default()),
        },
        HelmCallAnswer::Refuse => RunOrNot::Deny {
            message: match &said.note {
                Some(note) => format!("The person refused this: {note}"),
                None => String::from(
                    "The person refused this. Say what you could not do, and carry on without it",
                ),
            },
        },
    }
}

/// What a session is told for a call into a folder the owner blocked.
pub fn blocked_folder(folder: &str) -> String {
    format!(
        "The owner blocked {folder} for every job, so this was not run. Do not read or write \
         under it; say in your reply what you needed from it"
    )
}

/// What a session is told where nobody answered inside the hold.
pub fn unanswered(seconds: u64) -> RunOrNot {
    RunOrNot::Deny {
        message: format!(
            "Nobody answered this within {seconds} seconds, so it was not run. Say what you were \
             waiting on, and ask again if it still matters"
        ),
    }
}

/// What a session is told where Fleet stopped before anybody answered. `armada mcp` waits out
/// the restart, so asking again reaches the Fleet that comes back.
pub fn restarted() -> RunOrNot {
    RunOrNot::Deny {
        message: String::from(
            "Armada restarted before anybody answered this, so it was not run. Ask again: the call \
             waits for Armada to come back",
        ),
    }
}
