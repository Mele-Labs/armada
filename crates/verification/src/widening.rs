//! The one call a Drone's request for more scope costs.
//!
//! **It answers consistency, never desirability.** Whether widening is wise is
//! a person's, and this look exists precisely so that a person is not asked
//! about every request.
//!
//! **There is no parameter for the diff, the transcript or any count.**
//! [`WideningBrief::about`] takes the step, the request, the scope, the paths
//! and the Drone's reason — the question is about a plan rather than about
//! work, and a call handed the diff would be answering
//! [`ConvergenceBrief`](crate::ConvergenceBrief)'s question instead.
//!
//! **It is not a gate and cannot become one.** [`Widened`] shares no type with
//! [`Verdict`](crate::Verdict) or [`Refusals`](crate::Refusals): clearing a
//! widening advances nothing and refusing one fails nothing.

use core_model::{under, RepoPath, ResolvedStep, WriteTargets};

use crate::judge::{field, Unreadable};
use crate::request::Request;
use crate::standing::Standing;
use crate::wording::{Piece, Wording};

/// The two words the look may answer with, and the line the second owes.
pub const JUDGE_WIDEN_ANSWER: Piece = Piece {
    id: "judgeWidenAnswer",
    shipped: "\
Answer with nothing but the lines below.

If the paths belong to the step as it was described:

    answer: consistent

If they do not:

    answer: inconsistent
    because: <what the step was given to do, and what these paths are instead>

`because` is one line, names a path from the list above, and is read by the \
person this decision goes to. A reason that could be written about any other \
request is not a reason.",
};

/// What a widening look is told it is doing.
pub const JUDGE_WIDEN_OPENING: Piece = Piece {
    id: "judgeWidenOpening",
    shipped: "Somebody is part-way through a step of a larger task and has asked \
              to change where that task says its work is. Answer only the \
              question at the end.",
};

/// The line naming the step. `{step}` is its label.
pub const JUDGE_WIDEN_STEP: Piece = Piece {
    id: "judgeWidenStep",
    shipped: "The step being worked: {step}",
};

/// The line naming the file the step writes. `{file}` is its path.
pub const JUDGE_WIDEN_DELIVERABLE: Piece = Piece {
    id: "judgeWidenDeliverable",
    shipped: "The file that step was asked to write: {file}",
};

/// The line the declared paths follow.
pub const JUDGE_WIDEN_HELD: Piece = Piece {
    id: "judgeWidenHeld",
    shipped: "Where the task says its work is:",
};

/// The line the asked-for paths follow.
pub const JUDGE_WIDEN_ASKED: Piece = Piece {
    id: "judgeWidenAsked",
    shipped: "The paths being asked for on top of that:",
};

/// What the look is told about paths the step was fenced out of.
pub const JUDGE_WIDEN_FENCED: Piece = Piece {
    id: "judgeWidenFenced",
    shipped: "Of those, the step was written to stay out of the following. That \
              was decided before anybody had read this task's code, and it is not a \
              rule you are being asked to enforce — it is context for the one \
              question below:",
};

/// The line the asker's reason follows.
pub const JUDGE_WIDEN_WHY: Piece = Piece {
    id: "judgeWidenWhy",
    shipped: "Why, in the asker's own words. This is an argument rather than a \
              fact, and it is the asker's reading of its own work:",
};

/// The widening question.
pub const JUDGE_WIDEN_QUESTION: Piece = Piece {
    id: "judgeWidenQuestion",
    shipped: "The question: are those paths part of the step above? You are not \
              deciding whether the change is a good idea, whether the work is any \
              good, or whether anybody should be allowed to write there. You are \
              deciding one thing — whether editing those paths is part of doing \
              the step as it was described.",
};

/// Say which of the asked paths the step was written to stay out of.
///
/// **Material, and stated as a fact rather than as a verdict.** The whole
/// reason this call exists is that the fence was drawn before anybody read the
/// code, so a look that did not know a path was fenced would be answering a
/// narrower question than the one being asked — and a look told "the step
/// forbids this" would be answering a wider one. What it is given is when the
/// boundary was set, which is what makes the question decidable.
///
/// **It no longer says by whom, and that is the change `drone.exclude_paths`
/// forced.** A fence may now come from the step, from the repository's
/// `armada.yml` or from `config::resolve`'s default, and the resolved step
/// carries no tag saying which — deliberately, because the answer would be the
/// same either way and a tag nothing reads is a tag that goes stale. What every
/// tier shares is the only fact the question turns on: it was drawn before this
/// task's code was read.
///
/// Nothing is written where nothing is fenced, so the ordinary request carries
/// no paragraph about a list it does not touch.
fn fenced(question: &mut String, step: &ResolvedStep, asked: &[RepoPath], wording: &Wording) {
    let Some(scope) = step.evidence_scope() else {
        return;
    };
    let behind: Vec<&RepoPath> = asked
        .iter()
        .filter(|path| {
            scope
                .exclude_paths()
                .iter()
                .any(|fence| under(fence.as_str(), path.as_str()))
        })
        .collect();
    if behind.is_empty() {
        return;
    }
    question.push_str(&format!("\n{}\n", wording.get(JUDGE_WIDEN_FENCED)));
    for path in behind {
        question.push_str(&format!("  {}\n", path.as_str()));
    }
}

/// What the look said about a request for more scope.
///
/// **Two variants and no third.** A call that could not be made is Fleet's
/// `CallFailed`, one level up, and it is neither of these: a machine that
/// cannot answer must not produce an answer in either direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Widened {
    /// The paths belong to the step. **Nothing has been granted** — the
    /// declaration never bound writes — what has happened is that the Job's
    /// statement of where its work is has been corrected, and the drift check
    /// now measures against the corrected one.
    Consistent,
    /// They do not, and here is why.
    Inconsistent(NotWidened),
}

/// Why a request for more scope was not consistent with the step.
///
/// **One field, not the refusal's three.** `expected` and `produced` describe a
/// difference between work and a bar; there is no work here and no bar — there
/// is a request, and what is owed is why it does not belong to the step. A
/// three-field shape would be filled in by inventing two of them.
///
/// It reaches the Drone *and* the person, which is the other departure: a
/// Drone told only that it was refused would ask again for the same paths in
/// other words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotWidened {
    because: String,
}

impl NotWidened {
    /// One reason, cited. Public so a test can build the value the Judge would
    /// have produced without a model.
    pub fn because(because: &str) -> NotWidened {
        NotWidened {
            because: because.to_string(),
        }
    }

    /// Why the paths do not belong to the step.
    pub fn reason(&self) -> &str {
        &self.because
    }
}

/// What the one scope call is asked, assembled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WideningBrief {
    question: String,
}

impl WideningBrief {
    /// Assemble the question about a request for more scope.
    ///
    /// `held` is the scope the Job declared and `asked` the paths being added
    /// to it. Both are given whole: the look cannot answer whether an addition
    /// belongs to the step without seeing what the step was already working
    /// inside.
    pub fn about(
        step: &ResolvedStep,
        request: Request<'_>,
        standing: &Standing,
        held: &WriteTargets,
        asked: &[RepoPath],
        reason: &str,
    ) -> WideningBrief {
        WideningBrief::worded(
            step,
            request,
            standing,
            held,
            asked,
            reason,
            &Wording::shipped(),
        )
    }

    /// The same question in `wording`.
    pub fn worded(
        step: &ResolvedStep,
        request: Request<'_>,
        standing: &Standing,
        held: &WriteTargets,
        asked: &[RepoPath],
        reason: &str,
        wording: &Wording,
    ) -> WideningBrief {
        let mut question = format!("{}\n\n", wording.get(JUDGE_WIDEN_OPENING));
        question.push_str(&request.told(wording));
        question.push_str(&standing.told(wording));
        question.push_str(&format!(
            "{}\n",
            wording.fill(JUDGE_WIDEN_STEP, &[("step", step.label())])
        ));
        if let Some(deliverable) = step.deliverable() {
            question.push_str(&format!(
                "{}\n",
                wording.fill(JUDGE_WIDEN_DELIVERABLE, &[("file", deliverable)])
            ));
        }
        question.push_str(&format!("\n{}\n", wording.get(JUDGE_WIDEN_HELD)));
        match held.paths() {
            [] => question.push_str("  (it says it will change nothing)\n"),
            paths => {
                for path in paths {
                    question.push_str(&format!("  {}\n", path.as_str()));
                }
            }
        }
        question.push_str(&format!("\n{}\n", wording.get(JUDGE_WIDEN_ASKED)));
        for path in asked {
            question.push_str(&format!("  {}\n", path.as_str()));
        }
        fenced(&mut question, step, asked, wording);
        question.push_str(&format!("\n{}\n\n", wording.get(JUDGE_WIDEN_WHY)));
        question.push_str(&format!("  {}\n", reason.trim()));
        question.push_str(&format!("\n{}\n\n", wording.get(JUDGE_WIDEN_QUESTION)));
        question.push_str(wording.get(JUDGE_WIDEN_ANSWER));
        WideningBrief { question }
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read one answer back.
    ///
    /// **A `Result`, and the error is not an answer.** A call that replied in
    /// prose has established nothing, and reading it either way would put a
    /// parse failure in front of a person as a Judge's decision.
    pub fn read(&self, answer: &str) -> Result<Widened, Unreadable> {
        match field(answer, "answer").as_deref() {
            Some("consistent") => Ok(Widened::Consistent),
            Some("inconsistent") => {
                let because = field(answer, "because").ok_or(Unreadable::ScopeAnswerSaysNoWhy)?;
                Ok(Widened::Inconsistent(NotWidened { because }))
            }
            _ => Err(Unreadable::NoScopeAnswer),
        }
    }
}
