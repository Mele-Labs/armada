//! A [`Store`] that holds its rows in memory, for the tests that put two Fleets
//! on one store.
//!
//! # Two Fleets, one set of rows
//!
//! [`FakeStore::view`] is a second handle on the same rows with a switch and a
//! call log of its own. Machine A and machine B each hold one, so "the store
//! cannot be reached from B" and "A claimed nothing" are each a statement about
//! one machine, while a claim A makes is the claim B meets.
//!
//! # What it is faithful about
//!
//! The claim is atomic: the check and the write are one lock, as the real
//! store's one-request-at-a-time object makes them. It is **not** faithful
//! about time or signing, and a row keeps whatever `since` it was handed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use adapter_traits::{Claimed, Holder, OwnerKey, RedKey, RepositoryKey, RunRow, Store, StoreError};

#[derive(Debug, Default)]
struct Rows {
    claims: BTreeMap<(RepositoryKey, RedKey), Holder>,
    owners: BTreeMap<OwnerKey, Holder>,
    devices: BTreeSet<String>,
    runs: BTreeMap<RepositoryKey, Vec<RunRow>>,
}

/// A store that remembers what it was asked and reaches nothing.
#[derive(Debug, Default)]
pub struct FakeStore {
    rows: Arc<Mutex<Rows>>,
    unreachable: AtomicBool,
    calls: Mutex<Vec<&'static str>>,
}

impl FakeStore {
    pub fn new() -> FakeStore {
        FakeStore::default()
    }

    /// Another machine's handle on these rows: shared claims and owners, its own
    /// switch and its own record of calls.
    pub fn view(&self) -> FakeStore {
        FakeStore {
            rows: Arc::clone(&self.rows),
            ..FakeStore::default()
        }
    }

    /// Register a device as an enrolment would. The real store does this on
    /// `POST /v1/enrol`, which is not part of [`Store`].
    pub fn enrol(&self, device: &str) {
        self.rows.lock().unwrap_or_else(std::sync::PoisonError::into_inner).devices.insert(device.to_string());
    }

    /// The devices the store holds, whichever handle enrolled them.
    pub fn devices(&self) -> Vec<String> {
        self.rows.lock().unwrap_or_else(std::sync::PoisonError::into_inner).devices.iter().cloned().collect()
    }

    /// While set, every call from this handle fails as unreachable and changes
    /// nothing.
    pub fn set_unreachable(&self, unreachable: bool) {
        self.unreachable.store(unreachable, Ordering::SeqCst);
    }

    /// The methods this handle was called through, in order, **including calls
    /// that failed as unreachable**: a repository with its switch off must show
    /// none at all.
    pub fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    /// The runs the store holds for a repository, whichever handle wrote them.
    pub fn runs_of(&self, repository: &RepositoryKey) -> Vec<RunRow> {
        self.rows.lock().unwrap_or_else(std::sync::PoisonError::into_inner).runs.get(repository).cloned().unwrap_or_default()
    }

    fn rows(&self, call: &'static str) -> Result<MutexGuard<'_, Rows>, StoreError> {
        self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(call);
        if self.unreachable.load(Ordering::SeqCst) {
            return Err(StoreError::Unreachable("the fake store is switched off".to_string()));
        }
        Ok(self.rows.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

impl Store for FakeStore {
    fn claim(&self, repository: &RepositoryKey, red: &RedKey, holder: &Holder) -> Result<Claimed, StoreError> {
        let mut rows = self.rows("claim")?;
        match rows.claims.get(&(repository.clone(), red.clone())) {
            Some(held) => Ok(Claimed::Held(held.clone())),
            None => {
                rows.claims.insert((repository.clone(), red.clone()), holder.clone());
                Ok(Claimed::Won)
            }
        }
    }

    fn read_claim(&self, repository: &RepositoryKey, red: &RedKey) -> Result<Option<Holder>, StoreError> {
        Ok(self.rows("read_claim")?.claims.get(&(repository.clone(), red.clone())).cloned())
    }

    fn release_claim(&self, repository: &RepositoryKey, red: &RedKey) -> Result<(), StoreError> {
        self.rows("release_claim")?.claims.remove(&(repository.clone(), red.clone()));
        Ok(())
    }

    fn replace_claim(&self, repository: &RepositoryKey, red: &RedKey, holder: &Holder) -> Result<Claimed, StoreError> {
        self.rows("replace_claim")?
            .claims
            .insert((repository.clone(), red.clone()), holder.clone());
        Ok(Claimed::Won)
    }

    fn clear_repository(&self, repository: &RepositoryKey) -> Result<(), StoreError> {
        self.rows("clear_repository")?.claims.retain(|(under, _), _| under != repository);
        Ok(())
    }

    fn put_owner(&self, key: &OwnerKey, holder: &Holder) -> Result<(), StoreError> {
        self.rows("put_owner")?.owners.insert(key.clone(), holder.clone());
        Ok(())
    }

    fn owners(&self, keys: &[OwnerKey]) -> Result<BTreeMap<OwnerKey, Holder>, StoreError> {
        let rows = self.rows("owners")?;
        Ok(keys
            .iter()
            .filter_map(|key| rows.owners.get(key).map(|holder| (key.clone(), holder.clone())))
            .collect())
    }

    fn delete_owner(&self, key: &OwnerKey) -> Result<(), StoreError> {
        self.rows("delete_owner")?.owners.remove(key);
        Ok(())
    }

    fn delete_device(&self, device: &str) -> Result<(), StoreError> {
        self.rows("delete_device")?.devices.remove(device);
        Ok(())
    }

    fn put_runs(&self, repository: &RepositoryKey, runs: &[RunRow]) -> Result<(), StoreError> {
        let mut rows = self.rows("put_runs")?;
        match runs.is_empty() {
            true => rows.runs.remove(repository),
            false => rows.runs.insert(repository.clone(), runs.to_vec()),
        };
        Ok(())
    }

    fn read_runs(&self, repository: &RepositoryKey) -> Result<Option<Vec<RunRow>>, StoreError> {
        Ok(self.rows("read_runs")?.runs.get(repository).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_traits::HolderKind;

    fn holder(machine: &str) -> Holder {
        Holder { kind: HolderKind::Session, id: "j1".into(), machine: machine.into(), since: 100 }
    }

    fn repository() -> RepositoryKey {
        RepositoryKey::from_hash("aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111").unwrap()
    }

    fn red() -> RedKey {
        RedKey::from_hash("bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222").unwrap()
    }

    #[test]
    fn two_holders_racing_for_one_red_leave_one_winner_who_both_then_see() {
        let store = FakeStore::new();
        let winners: Vec<Claimed> = std::thread::scope(|scope| {
            let racers: Vec<_> = ["mac-a", "mac-b"]
                .into_iter()
                .map(|machine| {
                    let store = store.view();
                    scope.spawn(move || store.claim(&repository(), &red(), &holder(machine)).unwrap())
                })
                .collect();
            racers.into_iter().map(|r| r.join().unwrap()).collect()
        });
        let won = winners.iter().filter(|c| **c == Claimed::Won).count();
        assert_eq!(won, 1, "{winners:?}");
        let held = store.read_claim(&repository(), &red()).unwrap().expect("a row");
        let loser = winners.iter().find_map(|c| if let Claimed::Held(h) = c { Some(h) } else { None });
        assert_eq!(loser, Some(&held), "the loser is told who won");
    }

    #[test]
    fn a_view_meets_the_claim_another_view_made() {
        let a = FakeStore::new();
        let b = a.view();
        assert_eq!(a.claim(&repository(), &red(), &holder("mac-a")).unwrap(), Claimed::Won);
        assert_eq!(
            b.claim(&repository(), &red(), &holder("mac-b")).unwrap(),
            Claimed::Held(holder("mac-a"))
        );
    }

    #[test]
    fn an_unreachable_view_fails_every_call_and_changes_nothing() {
        let a = FakeStore::new();
        let b = a.view();
        b.set_unreachable(true);
        let said = b.claim(&repository(), &red(), &holder("mac-b")).unwrap_err();
        assert!(said.is_unreachable());
        assert!(b.read_claim(&repository(), &red()).unwrap_err().is_unreachable());
        assert!(b.owners(&[]).unwrap_err().is_unreachable());
        assert_eq!(a.calls(), Vec::<&str>::new(), "the other view's log is its own");
        assert_eq!(a.read_claim(&repository(), &red()).unwrap(), None, "nothing was written");
        assert_eq!(b.calls().len(), 3, "a call that failed is still a call");
    }

    #[test]
    fn replace_takes_it_over_and_release_gives_it_back() {
        let store = FakeStore::new();
        store.claim(&repository(), &red(), &holder("mac-a")).unwrap();
        assert_eq!(store.replace_claim(&repository(), &red(), &holder("mac-b")).unwrap(), Claimed::Won);
        assert_eq!(store.read_claim(&repository(), &red()).unwrap(), Some(holder("mac-b")));
        store.release_claim(&repository(), &red()).unwrap();
        assert_eq!(store.read_claim(&repository(), &red()).unwrap(), None);
    }

    #[test]
    fn green_clears_every_claim_under_one_repository_and_no_other() {
        let store = FakeStore::new();
        let other = RepositoryKey::from_hash("cccccccc33333333cccccccc33333333cccccccc33333333cccccccc33333333").unwrap();
        let older = RedKey::from_hash("dddddddd44444444dddddddd44444444dddddddd44444444dddddddd44444444").unwrap();
        store.claim(&repository(), &red(), &holder("mac-a")).unwrap();
        store.claim(&repository(), &older, &holder("mac-a")).unwrap();
        store.claim(&other, &red(), &holder("mac-a")).unwrap();
        store.clear_repository(&repository()).unwrap();
        assert_eq!(store.read_claim(&repository(), &red()).unwrap(), None);
        assert_eq!(store.read_claim(&repository(), &older).unwrap(), None);
        assert!(store.read_claim(&other, &red()).unwrap().is_some());
    }

    #[test]
    fn signing_a_device_out_removes_that_device_and_a_missing_one_is_done() {
        let store = FakeStore::new();
        store.enrol("1.aa");
        store.enrol("1.bb");
        store.delete_device("1.aa").unwrap();
        store.delete_device("1.zz").unwrap();
        assert_eq!(store.devices(), ["1.bb"]);
        store.set_unreachable(true);
        assert!(store.delete_device("1.bb").unwrap_err().is_unreachable());
        assert_eq!(store.devices(), ["1.bb"], "nothing changed");
    }

    #[test]
    fn owners_come_back_for_the_keys_asked_and_only_those_owned() {
        let store = FakeStore::new();
        let one = OwnerKey::from_hash("eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555").unwrap();
        let two = OwnerKey::from_hash("ffffffff66666666ffffffff66666666ffffffff66666666ffffffff66666666").unwrap();
        store.put_owner(&one, &holder("mac-a")).unwrap();
        let found = store.owners(&[one.clone(), two.clone()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[&one], holder("mac-a"));
        store.delete_owner(&one).unwrap();
        assert!(store.owners(&[one, two]).unwrap().is_empty());
    }
}
