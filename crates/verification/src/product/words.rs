//! The authored pieces of the product part of a Judge brief, as they ship.

use crate::wording::Piece;

/// The line a deliverable a step wrote follows. `{file}` is where Fleet read it.
pub const JUDGE_DELIVERABLE: Piece = Piece {
    id: "judgeDeliverable",
    shipped: "What this step produced, which is the document it was asked \
              for. Fleet read it from {file}:",
};

/// The line the Job's plan follows.
pub const JUDGE_PLAN: Piece = Piece {
    id: "judgePlan",
    shipped: "The Job's plan, as Fleet's own record holds it now. Read \
              separately from what this step delivers, and never in \
              place of it:",
};

/// The line the submitted summary follows, where a document is above it.
pub const JUDGE_SUMMARY_AFTER: Piece = Piece {
    id: "judgeSummaryAfterDocument",
    shipped: "The summary submitted with it. The document is above; \
              these three lines are not it:",
};

/// The line the submitted summary follows, where it is the product.
pub const JUDGE_SUMMARY: Piece = Piece {
    id: "judgeSummary",
    shipped: "What this step produced, which is the document it was asked for:",
};

/// The line the diff follows, where a document is above it.
pub const JUDGE_ALSO_CHANGED: Piece = Piece {
    id: "judgeAlsoChanged",
    shipped: "The step also changed these files:",
};

/// The line the diff follows, where it is the product.
pub const JUDGE_DIFF: Piece = Piece {
    id: "judgeDiff",
    shipped: "The change, as a diff:",
};

/// The line what earlier steps established follows.
pub const JUDGE_REFERENCES: Piece = Piece {
    id: "judgeReferences",
    shipped: "What earlier steps established, which this work is measured against \
              and is not itself under judgment:",
};
