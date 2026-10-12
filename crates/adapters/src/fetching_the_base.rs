//! The base a catch-up reads: the remote's copy of it, fetched, and the local
//! branch only where there is no remote copy to read.
//!
//! **Why not the local branch.** It is the one checked out in a person's own
//! checkout, and it is rarely fast-forwarded, so a Job's branch merged with it
//! is current against a base the forge no longer has. Job 14 (12 Oct 2026) was
//! reported "behind its base by 0 commits" while its pull request conflicted.
//!
//! **The fetch is its own call**, [`Delivery::fetch_base`](adapter_traits::Delivery::fetch_base),
//! and nothing else here runs it: a catch-up that fetched silently would put
//! the network, and a credential, inside a step boundary with nobody told when
//! it failed. [`base_ref`] only reads what the last fetch left.

use std::time::Duration;

use adapter_traits::BaseFetched;
use git2::Repository;

use crate::build_standing::fetch_within;
use crate::delivery::a_remote;

/// How long a fetch of one branch may run before it is stopped and reported.
const FETCH_WITHIN: Duration = Duration::from_secs(60);

/// What git says when the remote has no such branch.
const NO_SUCH_BRANCH: &str = "couldn't find remote ref";

/// Fetch `base` from the repository's remote into its remote-tracking ref.
///
/// `dir` is the repository or any worktree of it. The refspec names the
/// tracking ref outright, so it moves whatever the remote's configured refspec
/// is.
pub(crate) fn fetched(dir: &str, base: &str) -> BaseFetched {
    let Some(remote) = Repository::open(dir).ok().and_then(|repo| a_remote(&repo)) else {
        return BaseFetched::Local;
    };
    let refspec = format!("+refs/heads/{base}:{}", tracking_ref(&remote, base));
    let args = ["fetch", "--quiet", "--no-tags", &remote, &refspec];
    match fetch_within(dir, &args, FETCH_WITHIN) {
        Ok(()) => BaseFetched::Fetched { remote },
        // A base nobody has pushed has no second reading, the same as in
        // `Delivery::base_on_the_remote`.
        Err(said) if said.contains(NO_SUCH_BRANCH) => BaseFetched::Local,
        Err(said) => BaseFetched::Failed { remote, said },
    }
}

/// The ref a catch-up reads for `base`: `origin/main` where the repository has
/// a remote and has fetched that branch, `main` where it has not.
///
/// **A name `git merge` and `git rev-parse` take as it is**, so the merge it
/// makes is worded "Merge remote-tracking branch 'origin/main'".
pub(crate) fn base_ref(repo: &Repository, base: &str) -> String {
    let Some(remote) = a_remote(repo) else {
        return base.to_string();
    };
    match repo.find_reference(&tracking_ref(&remote, base)).is_ok() {
        true => format!("{remote}/{base}"),
        false => base.to_string(),
    }
}

fn tracking_ref(remote: &str, base: &str) -> String {
    format!("refs/remotes/{remote}/{base}")
}
