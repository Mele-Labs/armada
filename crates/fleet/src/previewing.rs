//! The branches Jobs offer for the owner's preview: kept, checked together, told to the Job, watched, and
//! served as one Refresh row. `docs/concepts/fleet.md`, *Offering a branch for the preview*. The acts
//! that make and withdraw an offer are `session_host::preview_acts`.
//!
//! **One check at a time, for the whole Fleet.** A trigger that arrives while one runs sets a flag and
//! the check runs once more when it ends, however many arrived. What is compiled is the Compiler a test
//! may plant, `adapters::preview_check`'s.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::preview_check::{self, Compiler, Verdict, CONFLICTS};
use api::Refusal;
use ipc::{
    BuildSource, Event, Instant, PreviewOfferRow, PreviewOffers, PreviewRefresh, PreviewRefreshSkipped, PreviewState, SessionId,
    SkipPreviewRefresh,
};
use sha2::{Digest, Sha256};
use store::{KeptPreviewOffered, SessionState};

use crate::building::{commit_in, held_in, restarting_in, source_in};
use crate::daemon::Fleet;
use crate::pull_notices::short;
use crate::repositories::Served;

/// What `scripts/preview` reads: one line a ready offer, in `crate::runtime::support_dir`.
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
    /// The main commit the last check ran against, so a move of main asks for another.
    main_seen: Mutex<Option<String>>,
}

impl Default for Previewing {
    fn default() -> Self {
        Previewing {
            running: AtomicBool::new(false),
            again: AtomicBool::new(false),
            compiler: Mutex::new(Arc::new(preview_check::Tools)),
            sent: Mutex::new(None),
            main_seen: Mutex::new(None),
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

/// `scripts/preview`'s file: a line `<branch>\t<tip>\t<session>\t<about>` a ready offer, oldest first.
fn file_of(offers: &[KeptPreviewOffered]) -> String {
    offers
        .iter()
        .filter(|one| one.state == READY)
        .map(|one| format!("{}\t{}\t{}\t{}\n", one.branch, one.tip, one.session_id, one.about.replace(['\t', '\n'], " ")))
        .collect()
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

    /// One check of every live offer, then what it found written, published and told. **Nothing here fails
    /// the turn**: a check that could not run leaves the offers as they were, to be asked for again.
    async fn preview_checked(self: &Arc<Self>) {
        self.preview_offers_swept().await;
        let Some((checkout, _)) = self.preview_place() else {
            return;
        };
        let offers = self.store().lock().await.preview_offered_all().unwrap_or_default();
        if offers.is_empty() {
            self.preview_moved().await;
            return;
        }
        let tips: Vec<String> = offers.iter().map(|one| one.tip.clone()).collect();
        let compiler = self.previewing().compiler();
        let (root, main) = (checkout.clone(), checkout);
        let checked = tokio::task::spawn_blocking(move || {
            let seen = preview_check::commit_of(&main, MAIN);
            preview_check::check(&root, MAIN, &tips, compiler.as_ref()).map(|verdicts| (verdicts, seen))
        })
        .await;
        let Ok(Ok((verdicts, seen))) = checked else {
            return;
        };
        let at = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            for (offer, verdict) in offers.iter().zip(&verdicts) {
                let _ = match verdict {
                    Verdict::Ready => store.keep_preview_checked(&offer.session_id, &offer.branch, &offer.tip, READY, None, &[], &at),
                    Verdict::Failed(broke) => {
                        store.keep_preview_checked(&offer.session_id, &offer.branch, &offer.tip, FAILED, Some(&broke.why), &broke.files, &at)
                    }
                };
            }
        }
        *self.previewing().main_seen.lock().expect("not poisoned") = seen;
        self.preview_moved().await;
        self.preview_offers_told().await;
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
        let Some((checkout, _)) = self.preview_place() else {
            return;
        };
        let offers = self.store().lock().await.preview_offered_all().unwrap_or_default();
        if offers.is_empty() {
            return;
        }
        let main = tokio::task::spawn_blocking(move || preview_check::commit_of(&checkout, MAIN)).await.unwrap_or_default();
        let moved = main.is_some() && *self.previewing().main_seen.lock().expect("not poisoned") != main;
        let unfinished = offers.iter().any(|one| one.state == CHECKING);
        if went || moved || unfinished {
            self.preview_check_wanted();
        }
        self.preview_moved().await;
    }

    /// The offers and the Refresh row as Bridge draws them.
    pub(crate) async fn preview_offers_read(&self) -> PreviewOffers {
        let held = self.preview_build_holds();
        let store = self.store().lock().await;
        let kept = store.preview_offered_all().unwrap_or_default();
        let rows: Vec<PreviewOfferRow> = kept
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
                in_build: held.get(&one.branch).is_some_and(|tip| *tip == one.tip),
            })
            .collect();
        let ready: Vec<(&str, &str)> = rows.iter().filter(|one| one.state == PreviewState::Ready).map(|one| (one.branch.as_str(), one.tip.as_str())).collect();
        let new: Vec<String> = rows.iter().filter(|one| one.state == PreviewState::Ready && !one.in_build).map(|one| one.branch.clone()).collect();
        let id = refresh_id(&ready);
        let skipped = store.preview_refresh_skipped().ok().flatten().is_some_and(|skipped| skipped == id);
        let refresh = (!new.is_empty() && !skipped).then_some(PreviewRefresh { id, branches: new });
        PreviewOffers { offers: rows, refresh }
    }

    /// `skip_preview_refresh`: keep the id skipped, so the row is hidden until the ready set changes.
    pub(crate) async fn preview_refresh_skipped_now(&self, asked: SkipPreviewRefresh) -> Result<PreviewRefreshSkipped, Refusal> {
        let at = self.now().as_str().to_string();
        self.store().lock().await.keep_preview_refresh_skipped(&asked.id, &at).map_err(|why| self.ledger_fault(why))?;
        self.preview_moved().await;
        Ok(PreviewRefreshSkipped { id: asked.id })
    }

    /// The branches the running build holds, at the tips it holds them, where the build is the preview.
    fn preview_build_holds(&self) -> BTreeMap<String, String> {
        let support = crate::runtime::support_dir(&self.host().home);
        if restarting_in(&support, SystemTime::now()) || !matches!(source_in(&support), Ok(BuildSource::Preview)) {
            return BTreeMap::new();
        }
        commit_in(&support).and_then(|commit| held_in(&support, &commit)).map(|held| held.tips).unwrap_or_default()
    }

    /// The set of offers moved: write what `scripts/preview` reads and publish the answer, each only where
    /// it differs from what was last written or sent.
    pub(crate) async fn preview_moved(&self) {
        let kept = self.store().lock().await.preview_offered_all().unwrap_or_default();
        let wanted = file_of(&kept);
        let path = crate::runtime::support_dir(&self.host().home).join(OFFERED);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(wanted.as_str()) {
            let _ = written_atomically(&path, &wanted);
        }
        let now = self.preview_offers_read().await;
        let changed = {
            let mut sent = self.previewing().sent.lock().expect("not poisoned");
            let changed = sent.as_ref() != Some(&now);
            if changed {
                *sent = Some(now.clone());
            }
            changed
        };
        if changed {
            self.publish(Event::PreviewOffersChanged(now));
        }
    }
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
