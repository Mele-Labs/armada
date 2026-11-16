//! One Job's calls to the store in a pass: give back what it no longer holds, claim what it newly
//! holds, then renew. **Runs on a blocking thread** and stops at the first call that fails, so a
//! store that is down costs one timeout a Job, and what was not sent is sent by a later pass.

use std::collections::BTreeSet;

use adapter_traits::{ClaimKey, ClaimKind, Claimant, Claimed, RepositoryKey, Store, StoreError};
use sha2::{Digest, Sha256};

use super::{Wanted, CALLS_A_PASS};

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The key a claim is under: a hash of its name, and for a pull request of `owner/name#number`, as
/// the contract says a pull request's key is. The same on every Mac.
pub(crate) fn key_of(wanted: &Wanted, slug: &str) -> Option<ClaimKey> {
    let named = match wanted.kind {
        ClaimKind::PullRequest => format!("{slug}#{}", wanted.named),
        _ => wanted.named.clone(),
    };
    ClaimKey::from_hash(&sha256_hex(&named))
}

/// What one Job asks of the store in a pass.
#[derive(Clone, Debug, Default)]
pub(crate) struct Asked {
    pub release: Vec<Wanted>,
    pub claim: Vec<Wanted>,
    pub renew: bool,
    /// What it already holds, so a contest answered on a renewal is named.
    pub sent: BTreeSet<Wanted>,
}

/// What the store answered one Job.
#[derive(Clone, Debug, Default)]
pub(crate) struct Answered {
    pub released: Vec<Wanted>,
    pub won: Vec<Wanted>,
    pub held: Vec<(Wanted, Claimant)>,
    /// How many claims a renewal said it still has, where one was made.
    pub renewed: Option<u64>,
    /// Who else reached what it holds, since its last renewal.
    pub contested: Vec<(Wanted, Claimant)>,
    /// The call that failed, after which nothing more was asked.
    pub failed: Option<StoreError>,
}

/// Ask `asked` of the store for `claimant` in the repository `slug`. `live_here` is every live Job
/// on this Mac: **a claim this Mac's own ended Job left is taken over**, never waited on.
pub(crate) fn exchanged(store: &dyn Store, slug: &str, repository: &RepositoryKey, claimant: &Claimant, asked: Asked, live_here: &BTreeSet<String>) -> Answered {
    let mut answered = Answered::default();
    let mut calls = 0usize;
    let ours = |held: &Claimant| held.holder.kind == claimant.holder.kind && held.holder.id == claimant.holder.id && held.holder.machine == claimant.holder.machine;
    let left_here = |held: &Claimant| held.holder.machine == claimant.holder.machine && !live_here.contains(&held.holder.id);
    for wanted in asked.release {
        if calls >= CALLS_A_PASS {
            return answered;
        }
        let Some(key) = key_of(&wanted, slug) else { continue };
        calls += 1;
        match store.release_claim(repository, wanted.kind, &key) {
            Ok(()) => answered.released.push(wanted),
            Err(why) => {
                answered.failed = Some(why);
                return answered;
            }
        }
    }
    for wanted in asked.claim {
        if calls >= CALLS_A_PASS {
            return answered;
        }
        let Some(key) = key_of(&wanted, slug) else { continue };
        calls += 1;
        let claimed = match store.claim(repository, wanted.kind, &key, claimant) {
            Ok(Claimed::Held(held)) if left_here(&held) && !ours(&held) => {
                calls += 1;
                store.replace_claim(repository, wanted.kind, &key, claimant)
            }
            other => other,
        };
        match claimed {
            Ok(Claimed::Won) => answered.won.push(wanted),
            Ok(Claimed::Held(held)) if ours(&held) => answered.won.push(wanted),
            Ok(Claimed::Held(held)) => answered.held.push((wanted, held)),
            Err(why) => {
                answered.failed = Some(why);
                return answered;
            }
        }
    }
    if asked.renew {
        match store.renew(&claimant.holder) {
            Ok(renewed) => {
                answered.renewed = Some(renewed.claims);
                for contest in renewed.contested.into_iter().filter(|contest| &contest.repository == repository) {
                    let named = asked.sent.iter().chain(answered.won.iter()).find(|wanted| wanted.kind == contest.kind && key_of(wanted, slug).as_ref() == Some(&contest.key));
                    if let Some(wanted) = named {
                        answered.contested.push((wanted.clone(), contest.by));
                    }
                }
            }
            Err(why) => answered.failed = Some(why),
        }
    }
    answered
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_traits::{Holder, HolderKind};
    use testkit::FakeStore;

    fn claimant(id: &str, machine: &str) -> Claimant {
        Claimant::new(Holder::new(HolderKind::Session, id, machine, 1).expect("a holder"), "nick", id)
    }

    fn repository() -> RepositoryKey {
        RepositoryKey::from_hash(&sha256_hex("mele-labs/armada")).expect("a key")
    }

    #[test]
    fn a_pull_requests_key_takes_in_the_repository_and_a_files_does_not() {
        let file = key_of(&Wanted::file("src/a.rs"), "mele-labs/armada").expect("a key");
        assert_eq!(file.as_str(), sha256_hex("src/a.rs"));
        let pull = key_of(&Wanted::pull_request(12), "mele-labs/armada").expect("a key");
        assert_eq!(pull.as_str(), sha256_hex("mele-labs/armada#12"));
    }

    #[test]
    fn a_claim_held_by_this_macs_ended_job_is_taken_over_and_one_held_by_a_live_job_is_not() {
        let store = FakeStore::new();
        let gone = claimant("gone", "home");
        let asked = Asked { claim: vec![Wanted::item("armada-1")], ..Asked::default() };
        exchanged(&store, "mele-labs/armada", &repository(), &gone, asked.clone(), &BTreeSet::new());
        let live_here = BTreeSet::from(["new".to_string()]);
        let answered = exchanged(&store, "mele-labs/armada", &repository(), &claimant("new", "home"), asked.clone(), &live_here);
        assert_eq!(answered.won, vec![Wanted::item("armada-1")]);
        let answered = exchanged(&store, "mele-labs/armada", &repository(), &claimant("other", "work"), asked, &live_here);
        assert_eq!(answered.held.len(), 1);
        assert_eq!(answered.held[0].1.holder.id, "new");
    }

    #[test]
    fn a_store_that_is_down_stops_the_exchange_at_the_first_call() {
        let store = FakeStore::new();
        store.set_unreachable(true);
        let asked = Asked { claim: vec![Wanted::file("a"), Wanted::file("b")], renew: true, ..Asked::default() };
        let answered = exchanged(&store, "mele-labs/armada", &repository(), &claimant("j", "home"), asked, &BTreeSet::new());
        assert!(answered.failed.as_ref().is_some_and(StoreError::is_unreachable));
        assert!(answered.won.is_empty());
        assert_eq!(store.calls(), vec!["claim"]);
    }
}
