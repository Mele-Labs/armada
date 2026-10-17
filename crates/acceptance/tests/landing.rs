//! Landing's claim: **a Job's work reaches the thing it was for.**
//!
//! Two halves, and the second is half of what the milestone is for. A workflow
//! whose step declares `delivers: true` sends its branch out **when that step
//! is entered**, holds while a person reads what went out, and ends when that
//! person merges. A workflow where **no** step declares it finishes with
//! nothing pushed and no pull request — `design-plan`, `code-review`, `epic`
//! and `prototype`. The apparatus is [`bench::landing`], over
//! [`bench::board`]'s round trip, so every assertion about what somebody is
//! shown is made against a value that has been through [`ipc::encode`] and back.
//!
//! **The press itself is not asserted, for `recovery.rs`'s reason.** Merging is
//! a `Fleet` method over a store, a repository and a forge that is a process,
//! so what stands in for it is the pair either side — that the record reaches
//! the state the act is taken from, and that Fleet serves a route for it.
//!
//! | Proved | Not proved |
//! |---|---|
//! | Which step of a frozen workflow sends the work out, and that at most one does | That Fleet *sends* it. `Fleet::sent_out_on_entry` commits, pushes and opens; none of the three is reachable without a repository |
//! | That the send is declared and never inferred — a workflow may name no step, and one that names none still finishes | That the four shipped workflows are among those. `config::tests::shipped` asks that of the real files, against the roster the adapter resolves |
//! | That the delivering step is entered while the Job is still `running`, so the branch goes out before anybody is asked about it — #520 | The order inside the entry: the rebase, the commit, then the push. That is one `Fleet` method, and `fleet`'s own tests drive it through fakes |
//! | That the Job holds at `awaiting_review` with that step at `awaiting_human`, that a Board is served both, and that it carries no `Stuck` — so the merge is not a recourse and cannot be drawn as one | That a person is *shown* the pull request, or that Bridge draws the act anywhere. Nothing here renders |
//! | That merging is what takes the Job to `completed_success`, recorded as a person's act, and that every answer at the gate is an operation Fleet serves | That the press does it, or that pressing one lands. `Fleet::merge_pull_request` writes to somebody else's repository |
//! | That the comments on the pull request are served for a person to choose from, and that the act they press with is one Fleet serves | That a comment reaches a Drone, or that the pull request is answered. Both need a forge, and `fleet::tests::remarks` drives them through fakes |
//! | That a run of the after-merge Checks cannot start against a tree nobody committed — #474 | That it runs **once** for one commit. The dedupe is `store::already_proved`, keyed by the commit, and `store` has no in-memory constructor |

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the five.
#[allow(dead_code)]
mod bench;

use adapter_traits::RepositoryStanding;




// ---------------------------------------------------------------------------
// The work goes out, on the step that says so
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// And holds while somebody reads it
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// Until a person merges it
// ---------------------------------------------------------------------------



/// **The Checks that follow a merge are run against the tree the merge left,
/// and there is no other way to reach a run.** #474.
///
/// A fast-forward that happened names the commit it left, and one Fleet declined
/// to make names none — a person's uncommitted work, a checkout on another
/// branch, a history that would not fast-forward. So a proof run over a tree
/// nobody committed is not refused by a check somebody remembered to write; the
/// argument it would need does not exist.
#[test]
fn a_proof_run_cannot_start_against_a_tree_nobody_committed() {
    let moved = RepositoryStanding::MovedOn {
        base: String::from("main"),
        commits: 1,
        head: String::from("5b4ec827"),
    };
    assert_eq!(
        moved.caught_up_to(),
        Some("5b4ec827"),
        "what is proved is the tip the fast-forward left, not the merge commit \
         the forge named — a merge that arrived behind two others is proved once"
    );
    assert_eq!(
        RepositoryStanding::LeftAlone {
            why: String::from("`main` is carrying 2 uncommitted change(s)"),
        }
        .caught_up_to(),
        None,
        "and a repository Fleet would not touch hands over nothing to run \
         against"
    );
}

// ---------------------------------------------------------------------------
// And a Job that was never for a branch delivers nothing
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// The Jobs, each moved to where it stands by the machine itself
// ---------------------------------------------------------------------------




