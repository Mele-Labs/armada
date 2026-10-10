//! The words of the turns a Drone is given once the gate has ruled, as Armada
//! ships them. Moved here byte for byte from `super`; a caller holding a
//! settings.json override hands a [`Wording`](crate::Wording) in.

use crate::wording::Piece;

/// The opening where every declared check ran and passed. `{step}` is its label.
pub const OUTCOME_VERIFIED_ALL: Piece = Piece {
    id: "outcomeVerifiedAll",
    shipped: "{step} is verified. It passed every check the step declared.",
};

/// The opening where no declared check covers what changed.
pub const OUTCOME_VERIFIED_NONE_COVERED: Piece = Piece {
    id: "outcomeVerifiedNoneCovered",
    shipped: "{step} is verified. No check the step declares covers what you \
              changed, so none was run.",
};

/// The opening where some declared checks were not run.
pub const OUTCOME_VERIFIED_SOME: Piece = Piece {
    id: "outcomeVerifiedSome",
    shipped: "{step} is verified. It passed every check that covers what you \
              changed; the rest cover paths this step did not touch and were \
              not run.",
};

/// The opening where a person accepted the step at a human gate.
pub const OUTCOME_ACCEPTED: Piece = Piece {
    id: "outcomeAccepted",
    shipped: "{step} was reviewed and accepted.",
};

/// What follows the opening where another step comes next. `{step}` is its label.
pub const OUTCOME_GO_ON: Piece = Piece {
    id: "outcomeGoOn",
    shipped: "Go on to {step}. Submit when it is done, then wait.",
};

/// What follows the opening where the step was the last.
pub const OUTCOME_LAST: Piece = Piece {
    id: "outcomeLast",
    shipped: "That was the last part of this task. Nothing further is yours. Stop here.",
};

/// The line the failed checks follow on a hand-back. `{step}` is its label.
pub const OUTCOME_HANDED_BACK: Piece = Piece {
    id: "outcomeHandedBack",
    shipped: "{step} did not pass. This is what the checks found:",
};

/// The close of a hand-back.
pub const OUTCOME_WORK_AGAIN: Piece = Piece {
    id: "outcomeWorkAgain",
    shipped: "Work the same step again and submit when it is done. Fix what the output says is \
              wrong.\n\nDo not change what a check runs, and do not weaken, narrow, skip or \
              delete a test to get past it. A check that stops asking is not a check that passed, \
              and it is looked for.",
};

/// The base moved and the branch was put on top of it cleanly.
pub const BASE_UP_TO_DATE: Piece = Piece {
    id: "baseUpToDate",
    shipped: "While you worked, `{base}` moved on by {commits} commit(s) and your branch \
              has been put on top of it. Anything you are about to change may have \
              changed underneath you — re-read a file before you edit it.",
};

/// The base moved and files were left with conflict markers. `{files}` lists them.
pub const BASE_CONFLICTED: Piece = Piece {
    id: "baseConflicted",
    shipped: "While you worked, `{base}` moved on and your branch has been put on top of \
              it. These files were left with conflict markers in them and resolving them \
              is part of the work:\n\n{files}\n\nOpen each one, keep what belongs, and \
              remove every marker before you submit again.",
};

/// The base moved and the branch could not follow it.
pub const BASE_NOT_MOVED: Piece = Piece {
    id: "baseNotMoved",
    shipped: "While you worked, `{base}` moved on, and this branch could not be put on \
              top of it. It is exactly where you left it. Carry on — somebody will \
              reconcile the two.",
};
