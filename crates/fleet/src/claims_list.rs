//! The claims list on a project's page: every live claim in the project's store, grouped by who holds
//! it, as `GET /v1/claims` answers. `docs/contracts/central-store.md`, *Wire*.
//!
//! **The store holds hashes, so a name is matched here, never sent.** For each repository of the
//! project this Mac serves, Fleet hashes what it knows the names of: the files at its head and their
//! folders, the items, pull requests and tests its Jobs' ledgers name, what its live Jobs claimed or
//! met, and main's red commit and failing tests. A claim whose hash none of them makes, a teammate's
//! in a repository this Mac does not have, is shown by its Job's title alone. The owner's ruling of
//! 13 Nov 2026 (`.claude/decisions/2026-11-13-claims-store.md`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use adapter_traits::{AgentHarness, ClaimKind, Claimant, Delivery, HolderClaims, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;

use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;
use crate::flaky_tests::{TEST, TEST_ASKED, TEST_WAIT};
use crate::job_claims::{key_of, Wanted};
use crate::pull_requesting::PR;
use crate::red_claim::repository_key_of;
use crate::session_issues::ISSUE;
use crate::signing_in::this_machine;

/// What this Mac knows the names of in one repository, before they are hashed.
#[derive(Clone, Debug, Default)]
struct Known {
    root: String,
    items: BTreeSet<String>,
    files: BTreeSet<String>,
    tests: BTreeSet<String>,
    pulls: BTreeSet<String>,
    reds: BTreeSet<String>,
}

/// The names found, by repository hash, then kind and key hash; and each repository's name.
#[derive(Debug, Default)]
struct Named {
    keys: BTreeMap<String, BTreeMap<(ClaimKind, String), String>>,
    repositories: BTreeMap<String, String>,
}

impl Named {
    fn of(&self, repository: &str, kind: ClaimKind, key: &str) -> Option<String> {
        self.keys.get(repository)?.get(&(kind, key.to_string())).cloned()
    }
}

/// Hash every name `known` holds, off the async threads: `slug` reads the checkout, and a file list is
/// as long as the repository.
fn hashed(known: Vec<Known>, slug: impl Fn(&str) -> Option<String>) -> Named {
    let mut named = Named::default();
    for one in known {
        let Some(slug) = slug(&one.root) else { continue };
        let Some(repository) = repository_key_of(&slug) else { continue };
        let mut files = one.files.clone();
        for file in adapters::territory::files_at(std::path::Path::new(&one.root), "HEAD").unwrap_or_default() {
            let mut folder = file.as_str();
            while let Some((up, _)) = folder.rsplit_once('/') {
                files.insert(up.to_string());
                folder = up;
            }
            files.insert(file);
        }
        let keys = named.keys.entry(repository.as_str().to_string()).or_default();
        let kinds = [
            (ClaimKind::Item, &one.items),
            (ClaimKind::File, &files),
            (ClaimKind::Test, &one.tests),
            (ClaimKind::PullRequest, &one.pulls),
            (ClaimKind::Red, &one.reds),
        ];
        for (kind, names) in kinds {
            for name in names {
                if let Some(key) = key_of(&Wanted { kind, named: name.clone() }, &slug) {
                    keys.insert((kind, key.as_str().to_string()), name.clone());
                }
            }
        }
        named.repositories.insert(repository.as_str().to_string(), slug);
    }
    named
}

fn kind_of(kind: ClaimKind) -> ipc::HeldClaimKind {
    match kind {
        ClaimKind::Item => ipc::HeldClaimKind::Item,
        ClaimKind::File => ipc::HeldClaimKind::File,
        ClaimKind::Test => ipc::HeldClaimKind::Test,
        ClaimKind::PullRequest => ipc::HeldClaimKind::PullRequest,
        ClaimKind::Red => ipc::HeldClaimKind::RedMain,
    }
}

fn instant(seconds: i64) -> ipc::Instant {
    ipc::Instant::from(&core_model::Timestamp::from_rfc3339(rfc3339_utc(seconds.saturating_mul(1000))))
}

/// One group of the store's answer, named where this Mac can, for the wire.
fn group_wire(group: HolderClaims, named: &Named, machine: &str) -> ipc::ClaimsHolder {
    let wait = |kind: ClaimKind, repository: &str, key: &str, by: &Claimant| ipc::ClaimsWait {
        kind: kind_of(kind),
        named: named.of(repository, kind, key),
        name: by.name.clone(),
        machine: by.holder.machine.clone(),
        title: by.title.clone(),
    };
    let waits: Vec<ipc::ClaimsWait> = group.waits.iter().map(|one| wait(one.kind, one.repository.as_str(), one.key.as_str(), &one.held_by)).collect();
    // A pull request waits on what its Job waits on that is not a file: a file is shared, never waited on.
    let pull_waits: Vec<ipc::ClaimsWait> = group.waits.iter().filter(|one| one.kind != ClaimKind::File).map(|one| wait(one.kind, one.repository.as_str(), one.key.as_str(), &one.held_by)).collect();
    let claims = group
        .claims
        .iter()
        .map(|claim| ipc::HeldClaim {
            kind: kind_of(claim.kind),
            named: named.of(claim.repository.as_str(), claim.kind, claim.key.as_str()),
            repository: named.repositories.get(claim.repository.as_str()).cloned(),
            since: instant(claim.since),
            waits_on: match claim.kind {
                ClaimKind::PullRequest => pull_waits.clone(),
                _ => Vec::new(),
            },
        })
        .collect();
    ipc::ClaimsHolder {
        name: group.holder.name,
        this_mac: group.holder.holder.machine == machine,
        machine: group.holder.holder.machine,
        title: group.holder.title,
        job: group.holder.holder.id,
        renewed: instant(group.renewed),
        claims,
        waits,
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_project_claims`: the project's store's claims, grouped by holder and named where this Mac can.
    pub(crate) async fn project_claims(&self, project: &str) -> Result<ipc::ProjectClaims, Refusal> {
        let held = self.project_read(project)?;
        let none = |store: bool, unread: Option<String>| ipc::ProjectClaims { project: project.to_string(), store, unread, holders: Vec::new() };
        if held.store().is_none() {
            return Ok(none(false, None));
        }
        let roots = self.project_roots(&held);
        let mut listed = None;
        let mut unread = Some("This Mac has not joined it.".to_string());
        for root in &roots {
            match self.at_the_store_switched(root, true, |opened| opened.store.claims_listed().map(Some)).await {
                Ok(Some(Some(groups))) => {
                    listed = Some(groups);
                    unread = None;
                    break;
                }
                Ok(_) => continue,
                Err(why) => {
                    unread = Some(why.said());
                    break;
                }
            }
        }
        let Some(groups) = listed else { return Ok(none(true, unread)) };
        let mut known = Vec::new();
        for root in &roots {
            known.push(self.names_known(root).await);
        }
        let reaching = Arc::clone(self.reaching());
        let named = tokio::task::spawn_blocking(move || hashed(known, |root| reaching.slug(root))).await.unwrap_or_default();
        let configured: String = self.machine_settings().now().get(keys::MACHINE_NAME);
        let machine = this_machine(&configured);
        let holders = groups.into_iter().map(|group| group_wire(group, &named, &machine)).collect();
        Ok(ipc::ProjectClaims { project: project.to_string(), store: true, unread: None, holders })
    }

    /// The names this Mac knows in the repository at `root`, other than its files at the head.
    async fn names_known(&self, root: &str) -> Known {
        let mut known = Known { root: root.to_string(), ..Known::default() };
        let manifest = self.manifest_id_at(root).map(|id| id.to_string()).unwrap_or_default();
        {
            let store = self.store().lock().await;
            let targets = |kind: &str| store.attachment_targets(kind, &manifest).unwrap_or_default();
            known.items.extend(targets(ISSUE));
            known.pulls.extend(targets(PR));
            for kind in [TEST, TEST_WAIT, TEST_ASKED] {
                known.tests.extend(targets(kind));
            }
            if let Some(main) = store.main_ci(root).ok().flatten() {
                known.reds.insert(main.red_commit().to_string());
                known.reds.insert(main.decided().to_string());
                known.tests.extend(main.failed.iter().flat_map(|job| job.tests.iter().cloned()));
            }
        }
        self.job_claims().with(|kept| {
            let met = kept.jobs.values().flat_map(|held| held.sent.iter().chain(held.elsewhere.keys()));
            for wanted in met {
                let names = match wanted.kind {
                    ClaimKind::Item => &mut known.items,
                    ClaimKind::File => &mut known.files,
                    ClaimKind::Test => &mut known.tests,
                    ClaimKind::PullRequest => &mut known.pulls,
                    ClaimKind::Red => &mut known.reds,
                };
                names.insert(wanted.named.clone());
            }
            known.tests.extend(kept.queue_failed.keys().filter(|(at, _)| at == root).map(|(_, test)| test.clone()));
        });
        known
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_traits::{ClaimKey, Holder, HolderKind, ListedClaim, ListedWait, RepositoryKey};

    #[test]
    fn a_claim_is_named_only_where_its_hash_was_matched_and_a_pull_request_carries_its_jobs_waits() {
        let slug = "mele-labs/armada";
        let known = Known { root: "/no/such/root".into(), tests: BTreeSet::from(["tests::a".into()]), pulls: BTreeSet::from(["12".into()]), ..Known::default() };
        let named = hashed(vec![known], |_| Some(slug.to_string()));
        let repository = repository_key_of(slug).expect("a key");
        let key = |wanted: Wanted| key_of(&wanted, slug).expect("a key");
        let fixer = Claimant::new(Holder::new(HolderKind::Session, "f", "home", 0).expect("a holder"), "nick", "Fix the flaky test a");
        let other = RepositoryKey::from_hash(&"c".repeat(64)).expect("a key");
        let group = HolderClaims {
            holder: Claimant::new(Holder::new(HolderKind::Session, "w", "work", 0).expect("a holder"), "nick", "Land the queue race fix"),
            renewed: 120,
            claims: vec![
                ListedClaim { repository: repository.clone(), kind: ClaimKind::PullRequest, key: key(Wanted::pull_request(12)), since: 60 },
                ListedClaim { repository: other, kind: ClaimKind::Item, key: ClaimKey::from_hash(&"d".repeat(64)).expect("a key"), since: 60 },
            ],
            waits: vec![ListedWait { repository, kind: ClaimKind::Test, key: key(Wanted::test("tests::a")), held_by: fixer }],
        };

        let wire = group_wire(group, &named, "work");

        assert!(wire.this_mac);
        assert_eq!((wire.claims[0].named.as_deref(), wire.claims[0].repository.as_deref()), (Some("12"), Some(slug)));
        assert_eq!(wire.claims[0].waits_on.len(), 1);
        assert_eq!(wire.claims[0].waits_on[0].named.as_deref(), Some("tests::a"));
        assert_eq!(wire.claims[0].waits_on[0].title, "Fix the flaky test a");
        assert_eq!((wire.claims[1].named.as_deref(), wire.claims[1].repository.as_deref()), (None, None), "a teammate's repository this Mac lacks");
        assert!(wire.claims[1].waits_on.is_empty());
    }
}
