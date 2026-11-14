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
//! about time or signing: a row keeps whatever `since` it was handed, and a
//! claim never lapses here, so a renewal only counts and answers the contests.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use adapter_traits::{
    ClaimKey, ClaimKind, Claimant, Claimed, Contest, Holder, HolderClaims, ListedClaim, ListedWait, OwnerKey, Renewed, RepositoryKey, RunRow, Store, StoreDevice, StoreError,
};

#[derive(Debug, Default)]
struct Rows {
    claims: BTreeMap<(RepositoryKey, ClaimKind, ClaimKey), Claimant>,
    /// Who asked for a held claim, and whether its holder's renewal has told of them yet. Kept
    /// while the claim stands, so the list can say what each asker waits on.
    contenders: BTreeMap<(RepositoryKey, ClaimKind, ClaimKey), Vec<(Claimant, bool)>>,
    owners: BTreeMap<OwnerKey, Holder>,
    devices: BTreeSet<String>,
    /// The Mac each device enrolled as, in the order they enrolled.
    machines: Vec<(String, String)>,
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
        self.enrol_as(device, "");
    }

    /// [`FakeStore::enrol`], as the Mac named `machine`.
    pub fn enrol_as(&self, device: &str, machine: &str) {
        let mut rows = self.rows.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if rows.devices.insert(device.to_string()) {
            rows.machines.push((device.to_string(), machine.to_string()));
        }
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

/// The same holder: kind, id and machine, whatever `since` each carries.
fn same_holder(one: &Holder, other: &Holder) -> bool {
    one.kind == other.kind && one.id == other.id && one.machine == other.machine
}

impl Store for FakeStore {
    fn claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey, claimant: &Claimant) -> Result<Claimed, StoreError> {
        let mut rows = self.rows("claim")?;
        let at = (repository.clone(), kind, key.clone());
        match rows.claims.get(&at).cloned() {
            Some(held) => {
                if !same_holder(&held.holder, &claimant.holder) {
                    let asked = rows.contenders.entry(at).or_default();
                    let told = asked.iter().any(|(one, told)| *told && same_holder(&one.holder, &claimant.holder));
                    asked.retain(|(one, _)| !same_holder(&one.holder, &claimant.holder));
                    asked.push((claimant.clone(), told));
                }
                Ok(Claimed::Held(held))
            }
            None => {
                rows.claims.insert((repository.clone(), kind, key.clone()), claimant.clone());
                Ok(Claimed::Won)
            }
        }
    }

    fn read_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey) -> Result<Option<Claimant>, StoreError> {
        Ok(self.rows("read_claim")?.claims.get(&(repository.clone(), kind, key.clone())).cloned())
    }

    fn release_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey) -> Result<(), StoreError> {
        let mut rows = self.rows("release_claim")?;
        let at = (repository.clone(), kind, key.clone());
        rows.claims.remove(&at);
        rows.contenders.remove(&at);
        Ok(())
    }

    fn replace_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey, claimant: &Claimant) -> Result<Claimed, StoreError> {
        let mut rows = self.rows("replace_claim")?;
        let at = (repository.clone(), kind, key.clone());
        rows.contenders.remove(&at);
        rows.claims.insert(at, claimant.clone());
        Ok(Claimed::Won)
    }

    fn clear_kind(&self, repository: &RepositoryKey, kind: ClaimKind) -> Result<(), StoreError> {
        let mut rows = self.rows("clear_kind")?;
        rows.claims.retain(|(under, of, _), _| under != repository || *of != kind);
        rows.contenders.retain(|(under, of, _), _| under != repository || *of != kind);
        Ok(())
    }

    fn renew(&self, holder: &Holder) -> Result<Renewed, StoreError> {
        let mut rows = self.rows("renew")?;
        let mine: Vec<_> = rows.claims.iter().filter(|(_, held)| same_holder(&held.holder, holder)).map(|(at, _)| at.clone()).collect();
        let mut contested = Vec::new();
        for at in &mine {
            for (by, told) in rows.contenders.get_mut(at).into_iter().flatten() {
                if !*told {
                    *told = true;
                    contested.push(Contest { repository: at.0.clone(), kind: at.1, key: at.2.clone(), by: by.clone() });
                }
            }
        }
        Ok(Renewed { claims: mine.len() as u64, contested })
    }

    /// Grouped by holder in the store's order: name, Mac, kind, id. **`renewed` is the newest
    /// `since` of its claims**, since the fake keeps no clock.
    fn claims_listed(&self) -> Result<Vec<HolderClaims>, StoreError> {
        let rows = self.rows("claims_listed")?;
        let mut groups: Vec<HolderClaims> = Vec::new();
        let group_of = |groups: &mut Vec<HolderClaims>, who: &Claimant| -> usize {
            match groups.iter().position(|group| same_holder(&group.holder.holder, &who.holder)) {
                Some(at) => at,
                None => {
                    let holder = Holder { since: 0, ..who.holder.clone() };
                    groups.push(HolderClaims { holder: Claimant { holder, ..who.clone() }, renewed: 0, claims: Vec::new(), waits: Vec::new() });
                    groups.len() - 1
                }
            }
        };
        for ((repository, kind, key), held) in &rows.claims {
            let at = group_of(&mut groups, held);
            groups[at].renewed = groups[at].renewed.max(held.holder.since);
            groups[at].claims.push(ListedClaim { repository: repository.clone(), kind: *kind, key: key.clone(), since: held.holder.since });
        }
        for ((repository, kind, key), asked) in &rows.contenders {
            let Some(held) = rows.claims.get(&(repository.clone(), *kind, key.clone())) else { continue };
            for (by, _) in asked {
                let at = group_of(&mut groups, by);
                groups[at].waits.push(ListedWait { repository: repository.clone(), kind: *kind, key: key.clone(), held_by: held.clone() });
            }
        }
        for group in &mut groups {
            group.claims.sort_by(|a, b| (a.kind, a.since, &a.key).cmp(&(b.kind, b.since, &b.key)));
            group.waits.sort_by(|a, b| (a.kind, &a.key).cmp(&(b.kind, &b.key)));
        }
        let order = |group: &HolderClaims| (group.holder.name.clone(), group.holder.holder.machine.clone(), group.holder.holder.id.clone());
        groups.sort_by_key(order);
        Ok(groups)
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
        let mut rows = self.rows("delete_device")?;
        rows.devices.remove(device);
        rows.machines.retain(|(id, _)| id != device);
        Ok(())
    }

    fn signed_in_devices(&self) -> Result<Vec<StoreDevice>, StoreError> {
        let rows = self.rows("signed_in_devices")?;
        Ok(rows.machines.iter().map(|(id, machine)| StoreDevice { id: id.clone(), machine: machine.clone() }).collect())
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

    fn claimant(machine: &str) -> Claimant {
        Claimant::new(holder(machine), "nick", "Fix the flaky queue test")
    }

    fn red() -> ClaimKey {
        ClaimKey::from_hash("bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222").unwrap()
    }

    #[test]
    fn two_holders_racing_for_one_red_leave_one_winner_who_both_then_see() {
        let store = FakeStore::new();
        let winners: Vec<Claimed> = std::thread::scope(|scope| {
            let racers: Vec<_> = ["mac-a", "mac-b"]
                .into_iter()
                .map(|machine| {
                    let store = store.view();
                    scope.spawn(move || store.claim(&repository(), ClaimKind::Red, &red(), &claimant(machine)).unwrap())
                })
                .collect();
            racers.into_iter().map(|r| r.join().unwrap()).collect()
        });
        let won = winners.iter().filter(|c| **c == Claimed::Won).count();
        assert_eq!(won, 1, "{winners:?}");
        let held = store.read_claim(&repository(), ClaimKind::Red, &red()).unwrap().expect("a row");
        let loser = winners.iter().find_map(|c| if let Claimed::Held(h) = c { Some(h) } else { None });
        assert_eq!(loser, Some(&held), "the loser is told who won");
    }

    #[test]
    fn a_view_meets_the_claim_another_view_made() {
        let a = FakeStore::new();
        let b = a.view();
        assert_eq!(a.claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-a")).unwrap(), Claimed::Won);
        assert_eq!(
            b.claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-b")).unwrap(),
            Claimed::Held(claimant("mac-a"))
        );
    }

    #[test]
    fn an_unreachable_view_fails_every_call_and_changes_nothing() {
        let a = FakeStore::new();
        let b = a.view();
        b.set_unreachable(true);
        let said = b.claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-b")).unwrap_err();
        assert!(said.is_unreachable());
        assert!(b.read_claim(&repository(), ClaimKind::Red, &red()).unwrap_err().is_unreachable());
        assert!(b.owners(&[]).unwrap_err().is_unreachable());
        assert_eq!(a.calls(), Vec::<&str>::new(), "the other view's log is its own");
        assert_eq!(a.read_claim(&repository(), ClaimKind::Red, &red()).unwrap(), None, "nothing was written");
        assert_eq!(b.calls().len(), 3, "a call that failed is still a call");
    }

    #[test]
    fn replace_takes_it_over_and_release_gives_it_back() {
        let store = FakeStore::new();
        store.claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-a")).unwrap();
        assert_eq!(store.replace_claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-b")).unwrap(), Claimed::Won);
        assert_eq!(store.read_claim(&repository(), ClaimKind::Red, &red()).unwrap(), Some(claimant("mac-b")));
        store.release_claim(&repository(), ClaimKind::Red, &red()).unwrap();
        assert_eq!(store.read_claim(&repository(), ClaimKind::Red, &red()).unwrap(), None);
    }

    #[test]
    fn green_clears_every_red_under_one_repository_and_no_other_kind_or_repository() {
        let store = FakeStore::new();
        let other = RepositoryKey::from_hash("cccccccc33333333cccccccc33333333cccccccc33333333cccccccc33333333").unwrap();
        let older = ClaimKey::from_hash("dddddddd44444444dddddddd44444444dddddddd44444444dddddddd44444444").unwrap();
        store.claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-a")).unwrap();
        store.claim(&repository(), ClaimKind::Red, &older, &claimant("mac-a")).unwrap();
        store.claim(&repository(), ClaimKind::Item, &red(), &claimant("mac-a")).unwrap();
        store.claim(&other, ClaimKind::Red, &red(), &claimant("mac-a")).unwrap();
        store.clear_kind(&repository(), ClaimKind::Red).unwrap();
        assert_eq!(store.read_claim(&repository(), ClaimKind::Red, &red()).unwrap(), None);
        assert_eq!(store.read_claim(&repository(), ClaimKind::Red, &older).unwrap(), None);
        assert!(store.read_claim(&repository(), ClaimKind::Item, &red()).unwrap().is_some());
        assert!(store.read_claim(&other, ClaimKind::Red, &red()).unwrap().is_some());
    }

    #[test]
    fn a_renewal_counts_the_claims_of_one_holder_on_one_machine() {
        let store = FakeStore::new();
        store.claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-a")).unwrap();
        store.claim(&repository(), ClaimKind::File, &red(), &claimant("mac-a")).unwrap();
        assert_eq!(store.renew(&holder("mac-a")).unwrap().claims, 2);
        assert_eq!(store.renew(&holder("mac-b")).unwrap().claims, 0);
    }

    #[test]
    fn a_claim_held_for_another_holder_is_a_contest_its_holder_hears_once() {
        let store = FakeStore::new();
        store.claim(&repository(), ClaimKind::File, &red(), &claimant("mac-a")).unwrap();
        store.claim(&repository(), ClaimKind::File, &red(), &claimant("mac-a")).unwrap();
        assert_eq!(store.claim(&repository(), ClaimKind::File, &red(), &claimant("mac-b")).unwrap(), Claimed::Held(claimant("mac-a")));
        store.claim(&repository(), ClaimKind::File, &red(), &claimant("mac-b")).unwrap();

        let heard = store.renew(&holder("mac-b")).unwrap();
        assert_eq!((heard.claims, heard.contested), (0, Vec::new()), "the one refused is not told");

        let heard = store.renew(&holder("mac-a")).unwrap();
        assert_eq!(heard.claims, 1);
        assert_eq!(
            heard.contested,
            vec![Contest { repository: repository(), kind: ClaimKind::File, key: red(), by: claimant("mac-b") }]
        );
        assert!(store.renew(&holder("mac-a")).unwrap().contested.is_empty(), "told once");
    }

    #[test]
    fn a_claim_that_ends_takes_its_contests_with_it() {
        let store = FakeStore::new();
        let older = ClaimKey::from_hash("dddddddd44444444dddddddd44444444dddddddd44444444dddddddd44444444").unwrap();
        for (kind, key) in [(ClaimKind::File, red()), (ClaimKind::Red, red()), (ClaimKind::Test, older.clone())] {
            store.claim(&repository(), kind, &key, &claimant("mac-a")).unwrap();
            store.claim(&repository(), kind, &key, &claimant("mac-b")).unwrap();
        }
        store.release_claim(&repository(), ClaimKind::File, &red()).unwrap();
        store.clear_kind(&repository(), ClaimKind::Red).unwrap();
        store.replace_claim(&repository(), ClaimKind::Test, &older, &claimant("mac-c")).unwrap();
        store.claim(&repository(), ClaimKind::File, &red(), &claimant("mac-a")).unwrap();
        store.claim(&repository(), ClaimKind::Red, &red(), &claimant("mac-a")).unwrap();
        assert!(store.renew(&holder("mac-a")).unwrap().contested.is_empty());
        assert!(store.renew(&holder("mac-c")).unwrap().contested.is_empty());
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
