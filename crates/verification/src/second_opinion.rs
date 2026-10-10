//! The second reading of a judged gaming flag, and the only thing that clears
//! one.
//!
//! **A flag stops a step only once this agrees.** The first look is one cheap
//! call with no room to reason; this one is shown what that call was shown,
//! the flag and the question it answered, and may think before it answers.
//! Where it disagrees the flag is kept as cleared, with the reason in its own
//! words — the only record of how often the first look is wrong.
//!
//! **Anything short of a readable disagreement leaves the flag standing.**
//! Prose checked nothing, which is [`GamingBrief::read`]'s rule, and a
//! clearance advances a step, so it is held to what a refusal is held to.

use core_model::{CitedAt, ClearedFlag, GamingFlag, GamingPattern};

use crate::gaming::GamingBrief;
use crate::wording::{Piece, Wording};

/// What the answer ends with. **Reasoning first is allowed**, which is the
/// whole difference from the first look's format; the two lines are read from
/// the end so that reasoning using either word does not answer for them.
pub const JUDGE_SECOND_ANSWER: Piece = Piece {
    id: "judgeSecondAnswer",
    shipped: "\
Reason it through first, for as long as that takes. Then end your answer with \
these two lines and nothing after them:

    agree: yes or no
    why: <one or two plain sentences a person reads beside the flag>",
};

/// How the question is weighed. Owned here, beside the one reader it is for.
pub const JUDGE_SECOND_HOW_TO_WEIGH: Piece = Piece {
    id: "judgeSecondHowToWeigh",
    shipped: "\
An assertion is a check inside test code that can fail when the code under it \
is wrong. A comment, a doc comment or any other prose is never an assertion, \
whatever it says.

Agree where the change does what the question describes. Disagree where it \
does not: where the cited line is not the kind of thing the question is about, \
where what was dropped is still checked somewhere else in this change, or where \
the earlier step's evidence above called for this change.",
};

/// What a second reader is told it is doing.
pub const JUDGE_SECOND_OPENING: Piece = Piece {
    id: "judgeSecondOpening",
    shipped: "You are the second reader of a flag. A first reader was checking whether a \
              change was made to look finished rather than to be finished. It had no room to \
              reason, and it answered yes to the question below. Decide whether it was right.",
};

/// The line the first reader's question follows.
pub const JUDGE_SECOND_QUESTION: Piece = Piece {
    id: "judgeSecondQuestion",
    shipped: "The question the first reader answered yes to:",
};

/// The line what the first reader cited follows.
pub const JUDGE_SECOND_CITED: Piece = Piece {
    id: "judgeSecondCited",
    shipped: "What it cited:",
};

/// Where the cited line is, on a line the change leaves.
pub const JUDGE_SECOND_AT_LINE: Piece = Piece {
    id: "judgeSecondAtLine",
    shipped: "The diff holds that in `{file}`, at line {line} of the file as this change \
              leaves it.",
};

/// Where the cited line is, on a line the change removes.
pub const JUDGE_SECOND_REMOVED_LINE: Piece = Piece {
    id: "judgeSecondRemovedLine",
    shipped: "The diff holds that in `{file}`, on a line this change removes.",
};

/// One judged flag, put to a second reader.
///
/// **Built from the [`GamingBrief`] that raised it and nothing else**, for that
/// brief's reason: what the Drone said about its work is not an input to
/// judging it, and this call is shown no more than the first one was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondOpinion {
    flag: GamingFlag,
    question: String,
}

impl SecondOpinion {
    /// Assemble the second question about `flag`, which `first` raised.
    pub fn about(first: &GamingBrief, flag: GamingFlag) -> SecondOpinion {
        SecondOpinion::worded(first, flag, &Wording::shipped())
    }

    /// The same question in `wording`.
    pub fn worded(first: &GamingBrief, flag: GamingFlag, wording: &Wording) -> SecondOpinion {
        let mut question = format!("{}\n\n", wording.get(JUDGE_SECOND_OPENING));
        question.push_str(first.shown());
        question.push_str(&format!("\n\n{}\n\n", wording.get(JUDGE_SECOND_QUESTION)));
        question.push_str(first.asked());
        question.push_str(&format!("\n\n{}\n\n", wording.get(JUDGE_SECOND_CITED)));
        question.push_str(&flag.cited);
        question.push_str("\n\n");
        question.push_str(&placed(flag.at.as_ref(), wording));
        question.push_str(wording.get(JUDGE_SECOND_HOW_TO_WEIGH));
        question.push_str("\n\n");
        question.push_str(wording.get(JUDGE_SECOND_ANSWER));
        SecondOpinion { flag, question }
    }

    pub fn pattern(&self) -> GamingPattern {
        self.flag.pattern
    }

    /// The whole of what the call is shown.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// The flag, cleared where the answer disagrees and says why, standing
    /// otherwise. `kept` is where this call's brief was written, which only the
    /// caller knows.
    pub fn read(self, answer: &str, kept: Option<String>) -> GamingFlag {
        match self.clearance(answer) {
            Some(why) => GamingFlag {
                cleared: Some(ClearedFlag {
                    why,
                    brief_path: kept,
                }),
                ..self.flag
            },
            None => self.flag,
        }
    }

    /// The flag as the first look raised it, for a second call that never
    /// answered. A call that failed cleared nothing.
    pub fn unanswered(self) -> GamingFlag {
        self.flag
    }

    /// The reason, where the answer is a disagreement a person can read.
    ///
    /// **A quotation the reason invents voids it**, as it voids a refusal: a
    /// clearance persuades by what it cites, and one citing words the call was
    /// never shown is one nobody can check.
    fn clearance(&self, answer: &str) -> Option<String> {
        let agrees = last_field(answer, "agree")?;
        if !agrees.eq_ignore_ascii_case("no") {
            return None;
        }
        let why = last_field(answer, "why")?;
        crate::quoted::invented(&why, &self.question)
            .is_none()
            .then_some(why)
    }
}

/// Where the patch holds the citation, said to the reader so it need not hunt.
fn placed(at: Option<&CitedAt>, wording: &Wording) -> String {
    match at {
        Some(at) => match at.line() {
            Some(line) => format!(
                "{}\n\n",
                wording.fill(
                    JUDGE_SECOND_AT_LINE,
                    &[("file", at.path().as_str()), ("line", &line.to_string())]
                )
            ),
            None => format!(
                "{}\n\n",
                wording.fill(JUDGE_SECOND_REMOVED_LINE, &[("file", at.path().as_str())])
            ),
        },
        None => String::new(),
    }
}

/// The last `name:` line, because reasoning above it may use the word too.
fn last_field(answer: &str, name: &str) -> Option<String> {
    answer.lines().rev().find_map(|line| {
        let rest = line.trim().strip_prefix(name)?.strip_prefix(':')?.trim();
        (!rest.is_empty()).then(|| rest.to_string())
    })
}
