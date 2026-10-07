//! What the forge says about one pull request, and taking one out of draft.
//! `docs/concepts/session.md`, *Acts on a pull request*.
//!
//! **`--jq` and not a parse**, for [`crate::delivery::asked`]'s reason: the
//! forge does the reading and bytes enter as one line of text.

use adapter_traits::{PullRequestFacts, PullRequestStanding};

use crate::delivery::{asked, run_in, said, FORGE};
use crate::landing::fields;

/// Ask the forge. `None` is its silence: no tool, nobody signed in, no such
/// pull request, or a word for its state this build has no meaning for.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> Option<PullRequestFacts> {
    // A tab in a title would split the line, so the forge is asked to put a
    // space where one is.
    let said = asked(
        in_repo,
        pull_request,
        "state,isDraft,autoMergeRequest,headRefName,url,title",
        "[.state, (.isDraft | tostring), ((.autoMergeRequest != null) | tostring), \
         .headRefName, .url, (.title | gsub(\"\\t\"; \" \"))] | @tsv",
    )?;
    parsed(&said)
}

/// One line as [`read`] asks for it.
pub(crate) fn parsed(said: &str) -> Option<PullRequestFacts> {
    let [state, draft, auto, branch, url, title] = fields::<6>(said)?;
    let standing = match (state, draft) {
        ("OPEN", "true") => PullRequestStanding::Draft,
        ("OPEN", _) => PullRequestStanding::Open,
        ("MERGED", _) => PullRequestStanding::Merged,
        ("CLOSED", _) => PullRequestStanding::Closed,
        // A state this build has no word for is the forge's silence: guessing
        // one would let a merge be pressed on a pull request nobody understood.
        _ => return None,
    };
    Some(PullRequestFacts {
        standing,
        branch: branch.to_string(),
        auto_merge: auto == "true",
        title: title.to_string(),
        url: url.to_string(),
    })
}

/// Take a draft out of draft. **A write to the forge**, so the refusal is its
/// own sentence and nothing here retries.
pub(crate) fn mark_ready(in_repo: &str, pull_request: &str) -> Result<(), String> {
    let run = run_in(in_repo, FORGE, &["pr", "ready", pull_request])
        .map_err(|why| format!("`{FORGE}` would not run: {why}"))?;
    match run.status.success() {
        true => Ok(()),
        false => Err(said(&run)),
    }
}
