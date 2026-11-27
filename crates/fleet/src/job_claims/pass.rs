//! The pass that keeps each live Job's claims in its project's store, and what its answers do: a
//! wait on a held item, a word on a shared file, a retake after a lapse, a line while the store is
//! down. **Every call is made off the async workers** through [`Fleet::at_the_store_of`], so a
//! project with no store makes none.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use adapter_traits::{AgentHarness, ClaimKind, Claimant, Delivery, Holder, HolderKind, Vcs, WorkProduct};

use super::wanting::JobRead;
use crate::flaky_tests::{test_waits_kept, WaitEnded};
use super::words::{files_shared, item_closed, wait_over, waiting};
use super::{exchanged, Answered, Asked, JobHeld, Wait, WaitedOn, Wanted, PASS_EVERY};
use crate::daemon::Fleet;
use crate::red_claim::repository_key_of;

/// What a pass leaves to say: the words to each Job, sent as one notice, and each Job whose record
/// moved.
#[derive(Debug, Default)]
struct Said {
    told: BTreeMap<String, Vec<String>>,
    moved: BTreeSet<String>,
}

/// Whether two holders are the same Job. **`since` is left out**: a Job's claims taken in different
/// passes carry different times, and are still one Job to tell about.
fn same_job(one: &Holder, other: &Holder) -> bool {
    one.kind == other.kind && one.id == other.id && one.machine == other.machine
}

/// A holder as the store keeps what a Job was told of it: its Mac and Job, without `since`.
fn holder_key(holder: &Holder) -> String {
    format!("{}/{}", holder.machine, holder.id)
}

/// Who is told that `job` reached files `by` holds: `job` itself, and nobody where `by` is its own
/// parent or child. **The one place a shared file's recipients are chosen**; `by` hears the same
/// through its own pass, as a contest at its renewal.
fn shared_told_to(job: &JobRead, by: &Claimant) -> Vec<String> {
    match job.kin.contains(&by.holder.id) {
        true => Vec::new(),
        false => vec![job.id.clone()],
    }
}

/// What a Job's wait word does to what the store keeps it was told, so a restart says it once.
#[derive(Debug, PartialEq, Eq)]
enum Keeps {
    /// A wait started: kept as told, by the item and who holds it.
    Told { about: String, holder: String },
    /// A wait ended: what it was told of the item is forgotten, so a wait on it again is news.
    Forgets(String),
}

/// One word of a Job's waits, and what it keeps.
#[derive(Debug, PartialEq, Eq)]
struct Word {
    text: String,
    keeps: Keeps,
}

fn about_item(item: &str) -> String {
    format!("item:{item}")
}

fn about_file(file: &Wanted) -> String {
    format!("file:{}", file.named)
}

/// What the pass asks for `job`, with its held state brought up to date first: files a pull request
/// that merged or closed ended stay ended, and a claim held elsewhere is asked for again only when
/// the renewal is due.
fn asked_of(job: &JobRead, held: &mut JobHeld, person_held: &BTreeMap<String, String>, now: i64) -> (Asked, BTreeSet<Wanted>) {
    for number in job.pulls_ended.difference(&held.pulls_ended.clone()) {
        held.files_ended.extend(job.planned.union(&job.written).cloned());
        held.pulls_ended.insert(*number);
    }
    let wanted = job.wanted(person_held, &held.files_ended);
    held.elsewhere.retain(|one, _| wanted.contains(one));
    let renew = held.renewal_due(now) && !(held.sent.is_empty() && wanted.is_empty());
    let release = held.sent.difference(&wanted).cloned().collect();
    let claim = wanted.difference(&held.sent).filter(|one| renew || !held.elsewhere.contains_key(*one)).cloned().collect();
    (Asked { release, claim, renew, sent: held.sent.clone() }, wanted)
}

/// Take in what the store answered, or `None` where no store applies to the Job's repository.
fn answered_in(held: &mut JobHeld, answered: Option<Answered>, wanted: &BTreeSet<Wanted>, now: i64, said: &mut Vec<(Vec<Wanted>, Claimant)>) {
    let Some(answered) = answered else {
        held.sent.clear();
        held.elsewhere.clear();
        held.unsent = false;
        held.storeless = true;
        return;
    };
    held.storeless = false;
    for one in answered.released {
        held.sent.remove(&one);
    }
    for one in answered.won {
        held.elsewhere.remove(&one);
        held.sent.insert(one);
    }
    let mut shared: Vec<(Vec<Wanted>, Claimant)> = Vec::new();
    let mut tell_shared = |one: Wanted, by: Claimant| match shared.iter_mut().find(|(_, who)| same_job(&who.holder, &by.holder)) {
        Some((files, _)) => files.push(one),
        None => shared.push((vec![one], by)),
    };
    for (one, by) in answered.held {
        let new = held.elsewhere.get(&one).is_none_or(|was| !same_job(&was.holder, &by.holder));
        held.elsewhere.insert(one.clone(), by.clone());
        if new && one.kind == ClaimKind::File {
            tell_shared(one, by);
        }
    }
    for (one, by) in answered.contested {
        if one.kind == ClaimKind::File {
            tell_shared(one, by);
        }
    }
    said.extend(shared);
    if let Some(claims) = answered.renewed {
        held.renewed_at = Some(now);
        // Its claims lapsed: the next pass takes them again, through the same rules as a first take.
        if claims == 0 {
            held.sent.clear();
        }
    }
    let owed = wanted.iter().any(|one| !held.sent.contains(one) && !held.elsewhere.contains_key(one)) || held.sent.iter().any(|one| !wanted.contains(one));
    held.unsent = answered.failed.is_some() && owed;
}

/// The waits `held` has after a pass: each item a person or another Job holds. Answers the words
/// that start and end them.
fn waits_kept(job: &JobRead, held: &mut JobHeld, person_held: &BTreeMap<String, String>, since: &str) -> Vec<Word> {
    let mut now_waiting: BTreeMap<String, WaitedOn> = person_held.iter().map(|(item, person)| (item.clone(), WaitedOn::Person(person.clone()))).collect();
    for (one, by) in &held.elsewhere {
        if one.kind == ClaimKind::Item {
            now_waiting.entry(one.named.clone()).or_insert_with(|| WaitedOn::Job(by.clone()));
        }
    }
    let mut words = Vec::new();
    // A failing test's wait is `test_waits_kept`'s.
    let ended: Vec<String> = held.waits.iter().filter(|(item, wait)| wait.requeues.is_none() && !now_waiting.contains_key(*item)).map(|(item, _)| item.clone()).collect();
    for item in ended {
        held.waits.remove(&item);
        let text = match job.items.contains_key(&item) {
            true => wait_over(&item),
            false => item_closed(&item),
        };
        words.push(Word { text, keeps: Keeps::Forgets(about_item(&item)) });
    }
    for (item, on) in now_waiting {
        let same = held.waits.get(&item).is_some_and(|wait| match (&wait.on, &on) {
            (WaitedOn::Job(was), WaitedOn::Job(is)) => same_job(&was.holder, &is.holder),
            (was, is) => was == is,
        });
        if !same {
            let holder = match &on {
                WaitedOn::Job(by) => holder_key(&by.holder),
                WaitedOn::Person(person) => format!("person:{person}"),
                WaitedOn::Main => "main".to_string(),
            };
            words.push(Word { text: waiting(&item, &on), keeps: Keeps::Told { about: about_item(&item), holder } });
            held.waits.insert(item, Wait { on, since: since.to_string(), requeues: None });
        }
    }
    words
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
    /// Keep every live Job's claims, once [`PASS_EVERY`] has gone by since the last pass.
    pub(crate) async fn claims_kept(self: &Arc<Self>) {
        let now = self.now().epoch_millis().unwrap_or_default();
        let due = self.job_claims().with(|kept| {
            let due = kept.passed_at.is_none_or(|at| now.saturating_sub(at) >= PASS_EVERY.as_millis() as i64);
            if due {
                kept.passed_at = Some(now);
            }
            due
        });
        if due {
            self.claims_passed().await;
        }
    }

    /// One pass over every live Job, whatever the time.
    pub(crate) async fn claims_passed(self: &Arc<Self>) {
        let now = self.now().epoch_millis().unwrap_or_default();
        let since = self.now().as_str().to_string();
        let configured: String = self.machine_settings().now().get(config::settings::MACHINE_NAME);
        let machine = tokio::task::spawn_blocking(move || crate::signing_in::this_machine(&configured)).await.unwrap_or_default();
        self.job_claims().with(|kept| kept.machine = machine);
        let mut jobs = self.jobs_read().await;
        self.fixes_ended(&mut jobs).await;
        let live_here: BTreeSet<String> = jobs.iter().map(|job| job.id.clone()).collect();
        let assignees = self.job_claims().with(|kept| {
            kept.jobs.retain(|id, _| live_here.contains(id));
            kept.assignees.clone()
        });
        let mut said = Said::default();
        let mut ended: Vec<WaitEnded> = Vec::new();
        for job in jobs {
            let person_held = job.person_held(&assignees);
            let (asked, wanted, before) = self.job_claims().with(|kept| {
                let held = kept.jobs.entry(job.id.clone()).or_default();
                let before = (held.waits.clone(), held.unsent);
                let (asked, wanted) = asked_of(&job, held, &person_held, now);
                (asked, wanted, before)
            });
            let quiet = asked.release.is_empty() && asked.claim.is_empty() && !asked.renew;
            let answered = match quiet {
                true => Some(Answered::default()),
                false => self.claims_exchanged(&job, asked, &live_here).await,
            };
            let mut shared = Vec::new();
            let failed = answered.as_ref().and_then(|one| one.failed.as_ref()).map(adapter_traits::StoreError::said);
            let calls = answered.as_ref().map_or(0, |one| one.calls);
            let mut went_unsent = false;
            let mut over = None;
            let words = self.job_claims().with(|kept| {
                let held = kept.jobs.entry(job.id.clone()).or_default();
                over = held.called(calls, now);
                answered_in(held, answered, &wanted, now, &mut shared);
                went_unsent = held.unsent && !before.1;
                let words = waits_kept(&job, held, &person_held, &since);
                ended.extend(test_waits_kept(&job, held, &since));
                if (held.waits.clone(), held.unsent) != before {
                    said.moved.insert(job.id.clone());
                }
                words
            });
            // The card says only that the claims are unsent; the store's own words are kept here, once per spell,
            // so a refusal (a device it does not know, a clock off by over a minute) is not read as an outage.
            if let (true, Some(why)) = (went_unsent, failed) {
                self.session_warned(&job.id, "a Job's claims did not reach the store", why);
            }
            // Every call writes rows the store's plan counts by the day, so a Job that keeps calling is named here.
            if let Some(calls) = over {
                self.session_warned(&job.id, "a Job made more store calls this hour than it should", format!("{calls} calls since the hour began"));
            }
            // The Job's own waits go to it alone; a shared file goes to whoever `shared_told_to` names.
            let mut routed: BTreeMap<String, Vec<(Vec<Wanted>, Claimant)>> = BTreeMap::from([(job.id.clone(), Vec::new())]);
            for (files, by) in shared {
                for to in shared_told_to(&job, &by) {
                    routed.entry(to).or_default().push((files.clone(), by.clone()));
                }
            }
            let mut waits = Some(words);
            for (to, shared) in routed {
                let waits = match to == job.id {
                    true => waits.take().unwrap_or_default(),
                    false => Vec::new(),
                };
                let words = self.claims_unrepeated(&to, waits, shared).await;
                if !words.is_empty() {
                    said.told.entry(to).or_default().extend(words);
                }
            }
        }
        self.claims_said(said).await;
        self.test_waits_ended(ended).await;
    }

    /// Ask `asked` of the store for `job`. `None` where no store applies; a store that would not
    /// answer is an [`Answered`] whose `failed` says why.
    async fn claims_exchanged(&self, job: &JobRead, asked: Asked, live_here: &BTreeSet<String>) -> Option<Answered> {
        let (id, title, live, since) = (job.id.clone(), job.title.clone(), live_here.clone(), self.unix_seconds());
        let reached = self
            .at_the_store_switched(&job.root, true, move |opened| {
                let Some(repository) = repository_key_of(&opened.slug) else { return Ok(None) };
                let Some(holder) = Holder::new(HolderKind::Session, &id, &opened.machine, since) else { return Ok(None) };
                let claimant = Claimant::new(holder, &opened.login, &title);
                Ok(Some(exchanged(&*opened.store, &opened.slug, &repository, &claimant, asked, &live)))
            })
            .await;
        match reached {
            Ok(answered) => answered.flatten(),
            Err(why) => Some(Answered { failed: Some(why), ..Answered::default() }),
        }
    }

    /// The words to `session` that it was not told before, as the store keeps it, so a restart, which
    /// forgets `held`, does not repeat them. A store that cannot say tells again rather than stays quiet.
    async fn claims_unrepeated(&self, session: &str, waits: Vec<Word>, shared: Vec<(Vec<Wanted>, Claimant)>) -> Vec<String> {
        if waits.is_empty() && shared.is_empty() {
            return Vec::new();
        }
        let now = self.now();
        let mut store = self.store().lock().await;
        let mut words = Vec::new();
        for word in waits {
            match &word.keeps {
                Keeps::Told { about, holder } => {
                    if store.claims_told_before(session, about, holder).unwrap_or(false) {
                        continue;
                    }
                    let _ = store.keep_claims_told(session, about, holder, &now);
                }
                Keeps::Forgets(about) => {
                    let _ = store.forget_claims_told(session, Some(about));
                }
            }
            words.push(word.text);
        }
        for (files, by) in shared {
            let holder = holder_key(&by.holder);
            let new: Vec<&Wanted> = files.iter().filter(|file| !store.claims_told_before(session, &about_file(file), &holder).unwrap_or(false)).collect();
            if new.is_empty() {
                continue;
            }
            for file in &new {
                let _ = store.keep_claims_told(session, &about_file(file), &holder, &now);
            }
            words.push(files_shared(&new, &by));
        }
        words
    }

    async fn claims_said(self: &Arc<Self>, said: Said) {
        // One notice a Job, a line for each word, so a pass that finds several Jobs on its files
        // wakes it once.
        for (session, words) in said.told {
            let _ = self.told_by_fleet(&session, &words.join("\n\n"), ipc::Notice::Claims).await;
        }
        for session in said.moved {
            let record = {
                let store = self.store().lock().await;
                store.session(&session).ok().flatten().and_then(|one| self.ledger_row(&store, &one).ok())
            };
            if let Some(record) = record {
                self.publish(ipc::Event::SessionChanged(record));
            }
        }
    }

    /// A Job ended: give back every claim it holds, so another Job need not wait the 30 minutes
    /// of its lapse. A store that does not answer leaves them to lapse.
    pub(crate) async fn claims_given_back(&self, session: &str) {
        let _ = self.store().lock().await.forget_claims_told(session, None);
        let Some(held) = self.job_claims().with(|kept| kept.jobs.remove(session)) else { return };
        if held.sent.is_empty() {
            return;
        }
        let manifest = self.store().lock().await.session(session).ok().flatten().and_then(|one| one.manifest_id);
        let Some(served) = manifest.and_then(|id| self.served_named(Some(&ipc::ManifestId::carried(&id))).ok()) else { return };
        let job = JobRead { id: session.to_string(), root: served.root().to_string(), ..JobRead::default() };
        let asked = Asked { release: held.sent.into_iter().collect(), ..Asked::default() };
        let _ = self.claims_exchanged(&job, asked, &BTreeSet::new()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_traits::{Holder, HolderKind};

    fn other() -> Claimant {
        Claimant::new(Holder::new(HolderKind::Session, "j2", "work", 0).expect("a holder"), "nick", "Stage 4 walks across")
    }

    #[test]
    fn a_held_item_starts_a_wait_once_and_ends_it_when_the_claim_is_won() {
        let job = JobRead { items: BTreeMap::from([("armada-1".to_string(), None)]), ..JobRead::default() };
        let mut held = JobHeld::default();
        let wanted = BTreeSet::from([Wanted::item("armada-1")]);
        let held_answer = Answered { held: vec![(Wanted::item("armada-1"), other())], ..Answered::default() };
        answered_in(&mut held, Some(held_answer.clone()), &wanted, 0, &mut Vec::new());
        assert_eq!(waits_kept(&job, &mut held, &BTreeMap::new(), "t0").len(), 1);
        answered_in(&mut held, Some(held_answer), &wanted, 0, &mut Vec::new());
        assert!(waits_kept(&job, &mut held, &BTreeMap::new(), "t1").is_empty(), "told once");
        assert_eq!(held.waits["armada-1"].since, "t0");
        answered_in(&mut held, Some(Answered { won: vec![Wanted::item("armada-1")], ..Answered::default() }), &wanted, 0, &mut Vec::new());
        let over = waits_kept(&job, &mut held, &BTreeMap::new(), "t2");
        assert_eq!(over, vec![Word { text: wait_over("armada-1"), keeps: Keeps::Forgets(about_item("armada-1")) }]);
        assert!(held.waits.is_empty());
    }

    #[test]
    fn a_held_file_is_told_and_waits_on_nothing() {
        let job = JobRead::default();
        let mut held = JobHeld::default();
        let mut shared = Vec::new();
        let answer = Answered { held: vec![(Wanted::file("src/a.rs"), other())], ..Answered::default() };
        answered_in(&mut held, Some(answer), &BTreeSet::from([Wanted::file("src/a.rs")]), 0, &mut shared);
        assert_eq!(shared.len(), 1);
        assert!(waits_kept(&job, &mut held, &BTreeMap::new(), "t0").is_empty());
    }

    #[test]
    fn files_one_job_took_in_different_passes_are_told_as_one_word() {
        let later = Claimant::new(Holder::new(HolderKind::Session, "j2", "work", 60).expect("a holder"), "nick", "Stage 4 walks across");
        let mut held = JobHeld::default();
        let mut shared = Vec::new();
        let answer = Answered { held: vec![(Wanted::file("src/a.rs"), other()), (Wanted::file("src/b.rs"), later.clone())], ..Answered::default() };
        let wanted = BTreeSet::from([Wanted::file("src/a.rs"), Wanted::file("src/b.rs")]);
        answered_in(&mut held, Some(answer), &wanted, 0, &mut shared);
        assert_eq!(shared.len(), 1, "one Job, one word");
        assert_eq!(shared[0].0.len(), 2);
        let retaken = Answered { held: vec![(Wanted::file("src/a.rs"), later)], ..Answered::default() };
        let mut again = Vec::new();
        answered_in(&mut held, Some(retaken), &wanted, 0, &mut again);
        assert!(again.is_empty(), "the same Job retaking a file is not news");
    }

    #[test]
    fn a_file_held_by_the_jobs_own_parent_or_child_is_told_to_nobody() {
        let job = JobRead { id: "j1".into(), kin: BTreeSet::from(["j2".to_string()]), ..JobRead::default() };
        assert!(shared_told_to(&job, &other()).is_empty(), "j2 is its parent or child");
        let stranger = Claimant::new(Holder::new(HolderKind::Session, "j3", "work", 0).expect("a holder"), "nick", "Quiet the log");
        assert_eq!(shared_told_to(&job, &stranger), vec!["j1".to_string()]);
    }

    #[test]
    fn a_job_that_started_others_claims_what_it_writes_and_not_its_plans_paths() {
        let job = JobRead { planned: BTreeSet::from(["crates/ipc".to_string()]), written: BTreeSet::from(["src/a.rs".to_string()]), started_others: true, ..JobRead::default() };
        let (asked, _) = asked_of(&job, &mut JobHeld::default(), &BTreeMap::new(), 0);
        assert_eq!(asked.claim, vec![Wanted::file("src/a.rs")]);
    }

    #[test]
    fn a_store_that_did_not_answer_leaves_the_claims_unsent_and_asks_again() {
        let job = JobRead { written: BTreeSet::from(["src/a.rs".to_string()]), ..JobRead::default() };
        let mut held = JobHeld::default();
        let (asked, wanted) = asked_of(&job, &mut held, &BTreeMap::new(), 0);
        assert_eq!(asked.claim, vec![Wanted::file("src/a.rs")]);
        let down = Answered { failed: Some(adapter_traits::StoreError::Unreachable("down".into())), ..Answered::default() };
        answered_in(&mut held, Some(down), &wanted, 0, &mut Vec::new());
        assert!(held.unsent);
        let (asked, _) = asked_of(&job, &mut held, &BTreeMap::new(), 1_000);
        assert_eq!(asked.claim, vec![Wanted::file("src/a.rs")], "the difference is asked again");
    }

    #[test]
    fn a_merged_pull_request_ends_the_files_claimed_before_it() {
        let mut job = JobRead { written: BTreeSet::from(["src/a.rs".to_string()]), ..JobRead::default() };
        let mut held = JobHeld { sent: BTreeSet::from([Wanted::file("src/a.rs")]), renewed_at: Some(0), ..JobHeld::default() };
        job.pulls_ended.insert(9);
        let (asked, _) = asked_of(&job, &mut held, &BTreeMap::new(), 1_000);
        assert_eq!(asked.release, vec![Wanted::file("src/a.rs")]);
        assert!(asked.claim.is_empty());
    }
}
