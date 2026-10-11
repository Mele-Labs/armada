//! What the Drone is told once the gate has ruled, and when it is told nothing
//! at all.
//!
//! **The wording here is drafted, not sanctioned.**
//! `docs/contracts/agent-prompt.md` governs every turn Fleet injects and lists
//! six of them; the gate's own outcome turn is not one — the mechanism was
//! decided and the wording never written. What is below is a draft in the shape
//! the contract's other drafts take, and is marked as one.
//!
//! **A failure produces a turn only where there is something to do with it.** A
//! failed Check used to end the Job outright, so the Drone was terminated
//! rather than told: a turn explaining a verdict to a process about to be
//! killed spends a Drone's remaining tool call — measured, an injected message
//! is delivered at the next turn boundary, so it costs whatever is left of the
//! current one — to deliver information nobody reads. That holds exactly while
//! the failure is terminal. [`handed_back`](OutcomeTurn::handed_back) is where
//! it is not: the retry budget has room, the Drone is about to work the step
//! again, and what the Check printed is the whole of what it needs. Where the
//! budget is spent the Job still ends, the Drone is still not told, and the
//! reason goes to the person who opens the branch.

use config::ResolvedStep;

use crate::answered::Printed;
use crate::mechanical::CheckFailed;
use crate::Wording;

mod words;
pub use words::*;

/// What a turn says after its opening: the next step, or that there is none.
/// The two read differently on purpose: a Drone told only "verified" at the
/// end of a workflow keeps working.
fn then(next: Option<&ResolvedStep>, wording: &Wording) -> String {
    match next {
        Some(next) => wording.fill(OUTCOME_GO_ON, &[("step", next.label())]),
        None => wording.get(OUTCOME_LAST).to_string(),
    }
}

/// What the mechanical tier did on a step that advanced.
///
/// **Two counts, because three sentences are true of three different steps**:
/// every declared check ran and passed, some were not run because they cover
/// paths this step did not touch, or none was run at all. A turn that said the
/// first about any of the three would be telling a Drone a check passed that
/// nobody ran — the same lie [`OutcomeTurn::approved`] exists to avoid at a
/// human gate.
///
/// **No number reaches the Drone.** The counts decide which sentence, and the
/// sentence carries none of them, for this module's own reason: a Drone given
/// an arithmetic has an incentive to satisfy it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verified {
    declared: usize,
    skipped: usize,
}

impl Verified {
    /// Read off the checks the gate ran. One constructor, taking the set — so
    /// nothing can assemble a pair of counts that no step produced.
    pub fn of(ran: &crate::mechanical::Ran) -> Verified {
        Verified {
            declared: ran.count(),
            skipped: ran.skipped(),
        }
    }

    fn told(&self, label: &str, wording: &Wording) -> String {
        let piece = match (self.declared, self.skipped) {
            // A step that declares nothing, and a step whose every check ran.
            // The sentence is vacuously true of the first and was already
            // being told to it.
            (_, 0) => OUTCOME_VERIFIED_ALL,
            (declared, skipped) if declared == skipped => OUTCOME_VERIFIED_NONE_COVERED,
            _ => OUTCOME_VERIFIED_SOME,
        };
        wording.fill(piece, &[("step", label)])
    }
}

/// A turn Fleet injects into a live session.
///
/// **It is not a verdict a Drone can act on selectively.** Three constructors,
/// each reached from one place in `fleet::gate`: two say the step moved on and
/// the third says it did not and is being worked again. There is none that
/// says a step failed and is over — that turn does not exist, because there is
/// nobody left to read it.
///
/// **No counter, ever.** It carries no attempt count, no remaining budget and
/// no consequence, and there is no constructor that takes a number. A Drone one
/// attempt from escalation has every incentive to satisfy a bar rather than do
/// the work, and "this is your last try" is the sentence most likely to produce
/// a weakened assertion instead of a fix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomeTurn {
    text: String,
}

impl OutcomeTurn {
    /// The step passed, and the Drone continues.
    ///
    /// `next` is the step that follows, or `None` where the one that passed was
    /// the last. The two cases read differently on purpose: a Drone told only
    /// "verified" at the end of a workflow keeps working.
    ///
    /// `verified` is what the mechanical tier actually did, and it changes the
    /// opening sentence for [`approved`](OutcomeTurn::approved)'s reason: a
    /// Drone told it "passed every check the step declared" when a Check
    /// covering paths it never touched was not run has been told a check passed
    /// that nobody ran.
    pub fn advanced(
        passed: &ResolvedStep,
        next: Option<&ResolvedStep>,
        verified: Verified,
        wording: &Wording,
    ) -> OutcomeTurn {
        let opening = verified.told(passed.label(), wording);
        OutcomeTurn {
            text: format!("{opening}\n\n{}", then(next, wording)),
        }
    }

    /// A person took the work at a human gate, and the Drone continues.
    ///
    /// **A turn of its own, because the other one would be a lie.**
    /// [`advanced`](OutcomeTurn::advanced) says the step passed every check it
    /// declared, which is what the mechanical gate ruled; a human gate is a
    /// person reading the work and deciding, and the Drone is told that instead
    /// of being told a check passed that nobody ran.
    ///
    /// It carries no part of what the person said. Where there is something to
    /// change, the act is `request_changes` and the words go with it — an
    /// approval that quoted a reviewer would be an instruction wearing a
    /// verdict's shape.
    pub fn approved(
        passed: &ResolvedStep,
        next: Option<&ResolvedStep>,
        wording: &Wording,
    ) -> OutcomeTurn {
        let opening = wording.fill(OUTCOME_ACCEPTED, &[("step", passed.label())]);
        OutcomeTurn {
            text: format!("{opening}\n\n{}", then(next, wording)),
        }
    }

    /// The same turn, with what happened to the branch underneath added.
    ///
    /// `None` leaves it alone, which is how a base that did not move stays
    /// unannounced — a turn saying nothing happened spends a Drone's tool call
    /// to deliver nothing.
    pub fn and(self, moved: Option<TheBaseMoved>, wording: &Wording) -> OutcomeTurn {
        let Some(moved) = moved else {
            return self;
        };
        OutcomeTurn {
            text: format!("{}\n\n{}", self.text, moved.told(wording)),
        }
    }

    /// The step's mechanical gate failed, its budget has room, and the work is
    /// going back to the Drone that did it.
    ///
    /// **The output is the point.** A Drone told only that `test` failed knows
    /// less than a person reading the same row, and the run it is about to do
    /// would begin by running the check itself to find out — which is the turn
    /// paying for information it already had.
    ///
    /// `printed` carries what each named check put on its streams. A failure
    /// with nothing printed — an empty diff, a scope violation — has no entry
    /// and gets none: the expectation and what was produced already say the
    /// whole of it.
    ///
    /// It ends by naming the one thing a hand-back invites. A Drone that cannot
    /// make a test pass can always make the test stop asking, and this is the
    /// moment that becomes tempting; the gaming check catches it afterwards and
    /// saying so first is cheaper than catching it.
    pub fn handed_back(
        failed: &ResolvedStep,
        failures: &[CheckFailed],
        printed: &[Printed<'_>],
        wording: &Wording,
    ) -> OutcomeTurn {
        let said = failures
            .iter()
            .map(|failure| {
                format!(
                    "- expected {}, and {}",
                    failure.expected(),
                    failure.produced()
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        let opening = wording.fill(OUTCOME_HANDED_BACK, &[("step", failed.label())]);
        let mut text = format!("{opening}\n\n{said}\n\n");
        for one in printed {
            text.push_str(&one.quoted(wording));
        }
        text.push_str(wording.get(OUTCOME_WORK_AGAIN));
        OutcomeTurn { text }
    }

    /// The content of the injected message, exactly as it goes to the session.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// What happened to the branch a Drone is working on while it worked.
///
/// **Told, not asked.** The Drone has just submitted and holds no git, so
/// nothing here is a decision it could have taken part in — and a conflict is
/// work rather than a question, which is why the second variant reads as an
/// instruction and not as an apology.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TheBaseMoved {
    /// It moved and the branch was brought up to it cleanly.
    BroughtUpToDate { base: String, commits: usize },
    /// It moved, the branch was brought up to it, and files were left with
    /// conflict markers in them.
    Conflicted { base: String, files: Vec<String> },
    /// It moved and the branch could not be put on top of it, so nothing moved.
    /// **Nothing here is the Drone's to fix** — it is told because it is going
    /// on to work against a tree that is behind.
    CouldNotFollow { base: String },
}

impl TheBaseMoved {
    /// The paragraph that goes into the turn.
    fn told(&self, wording: &Wording) -> String {
        match self {
            TheBaseMoved::BroughtUpToDate { base, commits } => wording.fill(
                BASE_UP_TO_DATE,
                &[("base", base), ("commits", &commits.to_string())],
            ),
            TheBaseMoved::Conflicted { base, files } => {
                let files = files
                    .iter()
                    .map(|file| format!("- {file}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                wording.fill(BASE_CONFLICTED, &[("base", base), ("files", &files)])
            }
            TheBaseMoved::CouldNotFollow { base } => {
                wording.fill(BASE_NOT_MOVED, &[("base", base)])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advanced() -> OutcomeTurn {
        OutcomeTurn {
            text: String::from("Implement is verified."),
        }
    }

    #[test]
    fn a_base_that_did_not_move_leaves_the_turn_exactly_as_it_was() {
        assert_eq!(advanced().and(None, &Wording::shipped()), advanced());
    }

    #[test]
    fn a_clean_catch_up_tells_the_drone_what_moved_and_how_much() {
        let told = advanced()
            .and(
                Some(TheBaseMoved::BroughtUpToDate {
                    base: String::from("main"),
                    commits: 3,
                }),
                &Wording::shipped(),
            )
            .text()
            .to_string();
        assert!(told.starts_with("Implement is verified."), "{told}");
        assert!(
            told.contains("`main`") && told.contains("3 commit"),
            "{told}"
        );
        assert!(
            told.contains("re-read a file before you edit it"),
            "the reason it is being told at all: {told}"
        );
    }

    #[test]
    fn a_conflict_is_handed_over_as_work_and_names_every_file() {
        let told = advanced()
            .and(
                Some(TheBaseMoved::Conflicted {
                    base: String::from("main"),
                    files: vec![String::from("src/log.rs"), String::from("src/write.rs")],
                }),
                &Wording::shipped(),
            )
            .text()
            .to_string();
        assert!(told.contains("- src/log.rs"), "{told}");
        assert!(told.contains("- src/write.rs"), "{told}");
        assert!(
            told.contains("part of the work"),
            "a conflict is work, not a question: {told}"
        );
    }

    /// No count of anything the Drone could be measured on. The rule this type
    /// already carries about attempts holds for what moved underneath it too.
    #[test]
    fn nothing_told_here_is_a_number_a_drone_could_be_judged_on() {
        let told = advanced()
            .and(
                Some(TheBaseMoved::CouldNotFollow {
                    base: String::from("main"),
                }),
                &Wording::shipped(),
            )
            .text()
            .to_string();
        assert!(told.contains("exactly where you left it"), "{told}");
        assert!(!told.contains("attempt"), "{told}");
    }
}
