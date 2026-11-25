//! The branches Jobs offer for the owner's preview: kept, checked together, told to the Job, watched, and
//! served as one Refresh row. `docs/concepts/fleet.md`, *Offering a branch for the preview*. The acts
//! that make and withdraw an offer are `session_host::preview_acts`.
//!
//! **One check at a time, for the whole Fleet.** A trigger that arrives while one runs sets a flag and
//! the check runs once more when it ends, however many arrived. What is compiled is the Compiler a test
//! may plant, `adapters::preview_check`'s.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::preview_check::{self, Broke, Compiler, Verdict, CONFLICTS};
use api::Refusal;
use ipc::{
    BuildSource, Event, Instant, PreviewKept, PreviewOfferRow, PreviewOffers, PreviewRefresh, PreviewRefreshSkipped, PreviewState, SessionId,
    SkipPreviewRefresh,
};
use sha2::{Digest, Sha256};
use store::{KeptPreviewOffered, SessionState};

use adapters::BuildMerges;

use crate::building::{commit_in, held_by_merges, held_in, restarting_in, source_in, Held};
use crate::daemon::Fleet;
use crate::preview_carried::{carried_lines, Carried};
use crate::preview_macs::{clash_why, clashed_with, file_of, in_check_order, tips_here, Partners, NOT_ON_ORIGIN};
use crate::pull_notices::short;
use crate::repositories::Served;

/// What `scripts/preview` reads: one line a branch the running build holds or he picked, then one a ready offer, in
/// `crate::runtime::support_dir`.
const OFFERED: &str = "preview-offered";

/// What every offer is merged onto and measured against.
pub(crate) const MAIN: &str = "origin/main";

const CHECKING: &str = "checking";
const READY: &str = "ready";
const FAILED: &str = "failed";

/// The check's single flight and what Fleet last said of the offers. In memory: the offers are in the store.
pub(crate) struct Previewing {
    running: AtomicBool,
    again: AtomicBool,
    compiler: Mutex<Arc<dyn Compiler>>,
    /// The answer last published, so only a change goes out.
    sent: Mutex<Option<PreviewOffers>>,
    /// The notes last published, so only a change goes out.
    pub(crate) notes_sent: Mutex<Option<ipc::BuildNotes>>,
    /// The main commit the last check ran against, so a move of main asks for another.
    main_seen: Mutex<Option<String>>,
    /// What the merges of the last build commit read said it holds, so git is asked once per build.
    walked: Mutex<Option<(String, Option<BuildMerges>)>>,
    /// Whether each tip asked about is under the last build commit, so git is asked once per build and tip.
    reached: Mutex<(String, BTreeMap<String, bool>)>,
    /// What this Mac holds across checks of the offers of its other Macs: `preview_macs`.
    pub(crate) macs: crate::preview_macs::Macs,
}

impl Default for Previewing {
    fn default() -> Self {
        Previewing {
            running: AtomicBool::new(false),
            again: AtomicBool::new(false),
            compiler: Mutex::new(Arc::new(preview_check::Tools)),
            sent: Mutex::new(None),
            notes_sent: Mutex::new(None),
            main_seen: Mutex::new(None),
            walked: Mutex::new(None),
            reached: Mutex::new((String::new(), BTreeMap::new())),
            macs: crate::preview_macs::Macs::default(),
        }
    }
}

impl Previewing {
    /// What compiles, from here on. A test plants one; Fleet runs the shipped tools.
    #[cfg(test)]
    pub(crate) fn compiling_with(&self, compiler: Arc<dyn Compiler>) {
        *self.compiler.lock().expect("not poisoned") = compiler;
    }

    /// No check is running or owed.
    #[cfg(test)]
    pub(crate) fn idle(&self) -> bool {
        !self.running.load(Ordering::SeqCst) && !self.again.load(Ordering::SeqCst)
    }

    /// Whether the answer last published named a Mac left out.
    pub(crate) fn sent_left_out(&self) -> bool {
        self.sent.lock().expect("not poisoned").as_ref().is_some_and(|sent| !sent.left_out.is_empty())
    }

    fn compiler(&self) -> Arc<dyn Compiler> {
        Arc::clone(&self.compiler.lock().expect("not poisoned"))
    }
}

/// What the Job is told of an offer, once a (tip, state).
fn told_text(offer: &KeptPreviewOffered) -> Option<String> {
    let branch = &offer.branch;
    match offer.state.as_str() {
        READY => Some(format!("{branch} is on the preview list at {}.", short(&offer.tip))),
        FAILED => {
            let why = offer.why.as_deref().unwrap_or("it did not check").trim_end_matches('.');
            let files = if offer.files.is_empty() || why.starts_with(CONFLICTS) { String::new() } else { format!(" Files: {}.", offer.files.join(", ")) };
            Some(format!("{branch} is off the preview list: {why}.{files} Offer it again once it is fixed."))
        }
        _ => None,
    }
}

fn told_key(offer: &KeptPreviewOffered) -> String {
    format!("{} {}", offer.tip, offer.state)
}

fn state_of(word: &str) -> PreviewState {
    match word {
        READY => PreviewState::Ready,
        FAILED => PreviewState::Failed,
        _ => PreviewState::Checking,
    }
}

/// A stable name for a set of ready (branch, tip) pairs, which a skip is kept against.
fn refresh_id(ready: &[(&str, &str)]) -> String {
    let mut hash = Sha256::new();
    for (branch, tip) in ready {
        hash.update(format!("{branch}\t{tip}\n"));
    }
    format!("{:x}", hash.finalize()).chars().take(16).collect()
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
    /// The checkout the preview is built from, the repository it is served as, where `scripts/preview` is.
    pub(crate) fn preview_place(&self) -> Option<(PathBuf, Served)> {
        let checkout = self.build_checkout()?;
        let root = checkout.to_string_lossy().into_owned();
        let served = self.repositories().served().into_iter().find(|one| one.root() == root)?;
        checkout.join("scripts/preview").is_file().then_some((checkout, served))
    }

    /// Ask for a check. **One runs at a time**: asked while one runs, it runs once more after it.
    pub(crate) fn preview_check_wanted(self: &Arc<Self>) {
        let held = self.previewing();
        held.again.store(true, Ordering::SeqCst);
        if held.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                while fleet.previewing().again.swap(false, Ordering::SeqCst) {
                    fleet.preview_checked().await;
                }
                fleet.previewing().running.store(false, Ordering::SeqCst);
                // A trigger between the last look and the store found the flight still taken.
                if !fleet.previewing().again.load(Ordering::SeqCst) || fleet.previewing().running.swap(true, Ordering::SeqCst) {
                    break;
                }
            }
        });
    }

    /// One check of every live offer of both Macs, then what it found written, published and told. **Nothing
    /// here fails the turn**: a check that could not run leaves the offers as they were, to be asked for again.
    async fn preview_checked(self: &Arc<Self>) {
        self.preview_offers_swept().await;
        let Some((checkout, _)) = self.preview_place() else {
            return;
        };
        let offers = self.store().lock().await.preview_offered_all().unwrap_or_default();
        let partners = self.partners_read().await;
        if offers.is_empty() && partners.offers.is_empty() {
            self.partner_verdicts_kept(Vec::new());
            self.preview_pushed(&checkout, Vec::new()).await;
            self.preview_moved().await;
            return;
        }
        let order = in_check_order(&offers, &partners);
        let compiler = self.previewing().compiler();
        let (root, main, entries) = (checkout.clone(), checkout.clone(), order.clone());
        let checked = tokio::task::spawn_blocking(move || {
            let seen = preview_check::commit_of(&main, MAIN);
            let here = tips_here(&root, &entries);
            let runnable: Vec<usize> = (0..entries.len()).filter(|at| here[*at]).collect();
            let tips: Vec<String> = runnable.iter().map(|at| entries[*at].tip.clone()).collect();
            let verdicts = preview_check::check(&root, MAIN, &tips, compiler.as_ref())?;
            let mut aligned: Vec<Option<Verdict>> = vec![None; entries.len()];
            for (at, verdict) in runnable.into_iter().zip(verdicts) {
                aligned[at] = Some(verdict);
            }
            // An own offer that clashed with an offer of another Mac names that Mac.
            for at in 0..entries.len() {
                let Some(Verdict::Failed(broke)) = &aligned[at] else { continue };
                if entries[at].own.is_none() || !broke.why.starts_with(CONFLICTS) {
                    continue;
                }
                let clashed = clashed_with(&root, &entries[..at], &broke.files);
                if let Some(why) = clashed.and_then(|with| clash_why(&broke.files, &with)) {
                    aligned[at] = Some(Verdict::Failed(Broke { why, files: broke.files.clone() }));
                }
            }
            Ok::<_, String>((aligned, seen))
        })
        .await;
        let Ok(Ok((verdicts, seen))) = checked else {
            return;
        };
        let at = self.now().as_str().to_string();
        let mut ready = Vec::new();
        let mut theirs = Vec::new();
        {
            let mut store = self.store().lock().await;
            for (entry, verdict) in order.into_iter().zip(verdicts) {
                // A tip neither here nor on origin cannot build here: another Mac's offer is left out and named.
                let Some(verdict) = verdict else {
                    if entry.own.is_none() {
                        theirs.push((entry, PreviewState::Failed, Some(NOT_ON_ORIGIN.to_string()), Vec::new()));
                    }
                    continue;
                };
                let Some(session) = entry.own.as_deref() else {
                    let (state, why, files) = match verdict {
                        Verdict::Ready => (PreviewState::Ready, None, Vec::new()),
                        Verdict::Failed(broke) => (PreviewState::Failed, Some(broke.why), broke.files),
                    };
                    theirs.push((entry, state, why, files));
                    continue;
                };
                let _ = match &verdict {
                    Verdict::Ready => store.keep_preview_checked(session, &entry.branch, &entry.tip, READY, None, &[], &at),
                    Verdict::Failed(broke) => store.keep_preview_checked(session, &entry.branch, &entry.tip, FAILED, Some(&broke.why), &broke.files, &at),
                };
                if verdict == Verdict::Ready {
                    ready.push((entry.branch, entry.tip));
                }
            }
        }
        *self.previewing().main_seen.lock().expect("not poisoned") = seen;
        self.partner_verdicts_kept(theirs);
        self.preview_pushed(&checkout, ready).await;
        self.preview_moved().await;
        self.preview_offers_told().await;
        self.preview_pushes_told().await;
    }

    /// Tell each Job what its offer's check found, once a (tip, state).
    async fn preview_offers_told(self: &Arc<Self>) {
        let offers = self.store().lock().await.preview_offered_all().unwrap_or_default();
        for offer in offers {
            let Some(text) = told_text(&offer) else {
                continue;
            };
            if offer.told.as_deref() == Some(told_key(&offer).as_str()) {
                continue;
            }
            if matches!(self.told_news_by_fleet(&offer.session_id, &text, ipc::Notice::BuildMoved).await, Ok(true)) {
                let _ = self.store().lock().await.keep_preview_told(&offer.session_id, &offer.branch, &told_key(&offer));
            }
        }
    }

    /// Drop the offers that are over: tip on main, Job ended, or a branch that moved since it was offered.
    /// **Only a moved branch is told.** Whether any went.
    async fn preview_offers_swept(self: &Arc<Self>) -> bool {
        let Some((checkout, _)) = self.preview_place() else {
            return false;
        };
        let offers = self.store().lock().await.preview_offered_all().unwrap_or_default();
        let mut went = false;
        for offer in offers {
            let live = self.store().lock().await.session(&offer.session_id).ok().flatten().is_some_and(|one| one.state != SessionState::Ended);
            let (root, branch, tip) = (checkout.clone(), offer.branch.clone(), offer.tip.clone());
            let (now, merged) = tokio::task::spawn_blocking(move || {
                let now = adapters::branch_tip(&root.to_string_lossy(), &branch);
                (now, adapters::onto_base::is_ancestor(&root, &tip, MAIN))
            })
            .await
            .unwrap_or_default();
            let gone = now.is_none();
            let moved_to = now.filter(|now| *now != offer.tip);
            if live && !merged && !gone && moved_to.is_none() {
                continue;
            }
            let removed = self.store().lock().await.remove_preview_offered(&offer.session_id, &offer.branch).unwrap_or(false);
            went |= removed;
            if let (true, false, Some(now)) = (live, merged, moved_to) {
                let text = format!(
                    "{} moved to {} since you offered it, so it is off the preview list. Offer it again with offer_preview.",
                    offer.branch,
                    short(&now)
                );
                let _ = self.told_news_by_fleet(&offer.session_id, &text, ipc::Notice::BuildMoved).await;
            }
        }
        if went {
            self.preview_moved().await;
        }
        went
    }

    /// The sweep's part: drop what is over, and ask for a check where main moved or one never finished.
    pub(crate) async fn previews_watched(self: &Arc<Self>) {
        let went = self.preview_offers_swept().await;
        // A card whose offer is no longer on the list has nothing to try, so Fleet withdraws it and no Job need.
        let at = self.now().as_str().to_string();
        let _ = self.store().lock().await.keep_build_notes_withdrawn_without_offer(&at);
        // Before the early returns: a build holds notes whether or not an offer is left, and a Job ending is seen here.
        self.build_notes_moved().await;
        let Some((checkout, _)) = self.preview_place() else {
            return;
        };
        let offers = self.store().lock().await.preview_offered_all().unwrap_or_default();
        let partners = self.partners_read().await;
        let shifted = self.partners_moved(&partners);
        if offers.is_empty() && partners.offers.is_empty() && !shifted {
            // A Mac left out, or back, is something the answer says though no offer is left to check.
            if !partners.left_out.is_empty() || self.left_out_shown() {
                self.preview_moved_with(&partners).await;
            }
            return;
        }
        let main = tokio::task::spawn_blocking(move || preview_check::commit_of(&checkout, MAIN)).await.unwrap_or_default();
        let moved = main.is_some() && *self.previewing().main_seen.lock().expect("not poisoned") != main;
        let unfinished = offers.iter().any(|one| one.state == CHECKING);
        let owed = self.pushes_owed(&offers).await;
        if went || moved || unfinished || shifted || owed {
            self.preview_check_wanted();
        }
        self.preview_moved_with(&partners).await;
    }

    /// The offers and the Refresh row as Bridge draws them.
    pub(crate) async fn preview_offers_read(&self) -> PreviewOffers {
        let partners = self.partners_read().await;
        self.preview_offers_read_with(&partners).await
    }

    /// `preview_offers_read` where the offers held from other Macs are already read, so a pass reads them once.
    async fn preview_offers_read_with(&self, partners: &Partners) -> PreviewOffers {
        let build = self.preview_build();
        let carried = self.preview_carried().await;
        let held: Vec<&str> = carried.iter().filter(|one| one.held).map(|one| one.branch.as_str()).collect();
        let titles = self.titles_of_last(self.build_checkout().as_deref(), &held).await;
        let store = self.store().lock().await;
        let kept = store.preview_offered_all().unwrap_or_default();
        let mut rows: Vec<PreviewOfferRow> = kept
            .iter()
            .map(|one| PreviewOfferRow {
                branch: one.branch.clone(),
                tip: one.tip.clone(),
                session_id: SessionId::carried(one.session_id.as_str()),
                title: store.session(&one.session_id).ok().flatten().and_then(|row| row.title).unwrap_or_else(|| one.session_id.clone()),
                about: one.about.clone(),
                offered_at: Instant::carried(&one.offered_at),
                state: state_of(&one.state),
                why: one.why.clone(),
                files: one.files.clone(),
                in_build: self.build_holds(build.as_ref(), &one.branch, &one.tip),
                mac: None,
            })
            .collect();
        if !partners.offers.is_empty() {
            // Both Macs' offers in the order they are checked, which is the order they merge.
            rows.extend(partners.offers.iter().filter(|row| !kept.iter().any(|one| one.branch == row.branch)).cloned());
            rows.sort_by(|a, b| (a.offered_at.as_str(), &a.branch).cmp(&(b.offered_at.as_str(), &b.branch)));
        }
        let offered = ready_branches(&rows);
        // What the running build holds and he picked is carried forward; a ready offer of the branch brings its own tip.
        let carried: Vec<&Carried> = carried.iter().filter(|one| !offered.contains(&one.branch)).collect();
        let picked = carried.iter().filter(|one| !one.held);
        let mut ready: Vec<(&str, &str)> = rows.iter().filter(|one| one.state == PreviewState::Ready).map(|one| (one.branch.as_str(), one.tip.as_str())).collect();
        ready.extend(picked.clone().map(|one| (one.branch.as_str(), one.tip.as_str())));
        let mut new: Vec<String> = rows.iter().filter(|one| one.state == PreviewState::Ready && !one.in_build).map(|one| one.branch.clone()).collect();
        new.extend(picked.map(|one| one.branch.clone()));
        let keeps = carried
            .iter()
            .filter(|one| one.held)
            .map(|one| PreviewKept { branch: one.branch.clone(), title: titles.get(&one.branch).cloned().unwrap_or_else(|| one.branch.clone()) })
            .collect();
        let id = refresh_id(&ready);
        let skipped = store.preview_refresh_skipped().ok().flatten().is_some_and(|skipped| skipped == id);
        let refresh = (!new.is_empty() && !skipped).then_some(PreviewRefresh { id, branches: new, keeps });
        PreviewOffers { offers: rows, refresh, left_out: partners.left_out.clone() }
    }

    /// `skip_preview_refresh`: keep the id skipped, so the row is hidden until the ready set changes.
    pub(crate) async fn preview_refresh_skipped_now(&self, asked: SkipPreviewRefresh) -> Result<PreviewRefreshSkipped, Refusal> {
        let at = self.now().as_str().to_string();
        self.store().lock().await.keep_preview_refresh_skipped(&asked.id, &at).map_err(|why| self.ledger_fault(why))?;
        self.preview_moved().await;
        Ok(PreviewRefreshSkipped { id: asked.id })
    }

    /// Whether the running preview `build` holds `branch` at `tip`: it merged it, or `tip` is already under its
    /// commit, as where the branch landed on the main the build was cut from. `None` is no preview, which holds nothing.
    pub(crate) fn build_holds(&self, build: Option<&(String, BTreeMap<String, String>)>, branch: &str, tip: &str) -> bool {
        let Some((commit, tips)) = build else { return false };
        tips.get(branch).is_some_and(|held| held == tip) || self.tip_under_build(commit, tip)
    }

    /// Whether `tip` is a commit under the build at `commit`. Git is asked once per build commit and tip.
    fn tip_under_build(&self, commit: &str, tip: &str) -> bool {
        let mut reached = self.previewing().reached.lock().expect("not poisoned");
        if reached.0 != commit {
            *reached = (commit.to_string(), BTreeMap::new());
        }
        if let Some(known) = reached.1.get(tip) {
            return *known;
        }
        let root = self.build_checkout().map(|checkout| checkout.to_string_lossy().into_owned());
        let under = root.is_some_and(|root| adapters::tip_under_build(&root, commit, tip));
        reached.1.insert(tip.to_string(), under);
        under
    }

    /// The running build's commit and what it holds, where the build is the preview. `None` off it, and while
    /// a restart is under way.
    pub(crate) fn preview_build(&self) -> Option<(String, BTreeMap<String, String>)> {
        let support = crate::runtime::support_dir(&self.host().home);
        if restarting_in(&support, SystemTime::now()) || !matches!(source_in(&support), Ok(BuildSource::Preview)) {
            return None;
        }
        let commit = commit_in(&support)?;
        let tips = self.preview_held(&support, &commit).map(|held| held.tips).unwrap_or_default();
        Some((commit, tips))
    }

    /// What the preview build at `commit` holds: its own merges, where it has any, else `preview-held`.
    /// The merges are read from git once per build commit.
    pub(crate) fn preview_held(&self, support: &std::path::Path, commit: &str) -> Option<Held> {
        let mut walked = self.previewing().walked.lock().expect("not poisoned");
        if walked.as_ref().map(|(seen, _)| seen.as_str()) != Some(commit) {
            let root = self.build_checkout().map(|checkout| checkout.to_string_lossy().into_owned());
            *walked = Some((commit.to_string(), root.and_then(|root| adapters::build_merges(&root, commit))));
        }
        held_by_merges(held_in(support, commit), walked.as_ref().and_then(|(_, merges)| merges.clone()))
    }

    /// The set of offers moved: write what `scripts/preview` reads and publish the answer, each only where
    /// it differs from what was last written or sent.
    pub(crate) async fn preview_moved(&self) {
        let partners = self.partners_read().await;
        self.preview_moved_with(&partners).await;
    }

    /// `preview_moved` where the offers held from other Macs are already read.
    async fn preview_moved_with(&self, partners: &Partners) {
        let kept = self.store().lock().await.preview_offered_all().unwrap_or_default();
        let offers = file_of(&kept, partners);
        let offered: BTreeSet<String> = offers.lines().filter_map(|line| line.split('\t').next()).map(str::to_string).collect();
        let wanted = carried_lines(&self.preview_carried().await, &offered) + &offers;
        let path = crate::runtime::support_dir(&self.host().home).join(OFFERED);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(wanted.as_str()) {
            let _ = written_atomically(&path, &wanted);
        }
        let now = self.preview_offers_read_with(partners).await;
        let changed = {
            let mut sent = self.previewing().sent.lock().expect("not poisoned");
            let changed = sent.as_ref() != Some(&now);
            if changed {
                *sent = Some(now.clone());
            }
            changed
        };
        // What crosses to another Mac moves when a push finishes, though no row did.
        let changed = self.cut_changed(&now) || changed;
        if changed {
            self.publish(Event::PreviewOffersChanged(now));
        }
        // An offer or a withdrawal moves its own Job's cards; what else moved here is not on a card.
        self.build_notes_published().await;
    }
}

/// The branches of `rows` that are ready, which bring their own tip to a refresh.
fn ready_branches(rows: &[PreviewOfferRow]) -> BTreeSet<String> {
    rows.iter().filter(|one| one.state == PreviewState::Ready).map(|one| one.branch.clone()).collect()
}

/// Written beside and renamed over, so a reader never sees half.
fn written_atomically(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let beside = path.with_extension("tmp");
    std::fs::write(&beside, text)?;
    std::fs::rename(&beside, path)
}
