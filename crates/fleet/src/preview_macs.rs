//! The preview across his Macs: the ready offers a paired Mac shares, held beside this Mac's own, checked
//! together with them and merged into one refresh. `docs/concepts/fleet.md`, *Offering a branch for the preview*.
//!
//! **Only this Mac's own ready offers leave it, and only once they are on origin**: `scripts/preview` on the
//! other Mac merges a tip it holds, and fetches origin for one it does not. What is held from another Mac never
//! goes back out. **Any linked Mac's offers count**, asleep or on another build: what is left out is an offer
//! that cannot build here, one whose tip origin does not have or that fails the check, and each is named.
//! **One check, for both**: the later offered of two that clash fails, so each Mac reaches the same answer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::preview_check::{self, CONFLICTS};
use api::Refusal;
use ipc::{PreviewLeftOut, PreviewOfferRow, PreviewOffers, PreviewState};
use store::{FromMac, KeptPreviewOffered, MacState};

use crate::daemon::Fleet;
use crate::previewing::MAIN;

/// What this Mac found of one offer held from another, kept until that offer or its tip moves.
struct PartnerVerdict {
    tip: String,
    state: PreviewState,
    why: Option<String>,
    files: Vec<String>,
}

/// What this Mac remembers across checks of the preview: in memory, since each is rebuilt by the next check.
#[derive(Default)]
pub(crate) struct Macs {
    /// By (Mac, branch).
    verdicts: Mutex<BTreeMap<(String, String), PartnerVerdict>>,
    /// The own (branch, tip) pairs on origin.
    pushed: Mutex<BTreeSet<(String, String)>>,
    /// The own (branch, tip) pairs that would not push, and why.
    push_failed: Mutex<BTreeMap<(String, String), String>>,
    /// The pairs whose Job was told they would not push.
    push_told: Mutex<BTreeSet<(String, String)>>,
    /// The (Mac, branch, tip) set the last check ran with.
    counted: Mutex<Vec<(String, String, String)>>,
    /// The cut last sent to other Macs, so a push that finishes moves it though the answer did not.
    pub(crate) cut_sent: Mutex<Option<PreviewOffers>>,
    /// How many times `verdicts` was rewritten, one of what `partners_kept` is read from.
    verdict_moves: AtomicU64,
    /// What `partners_read` last answered and what it read that from, so a tick that finds all of it the same
    /// answers without decoding the held rows again.
    partners_kept: Mutex<Option<(PartnersFrom, Partners)>>,
    /// How many times the held rows were decoded.
    #[cfg(test)]
    pub(crate) partners_decoded: AtomicU64,
}

/// Everything `partners_read` answers from that is not read afresh each time: when it is the same, so is the answer.
#[derive(PartialEq)]
struct PartnersFrom {
    /// `Store::held_moves`, which moves with every write of what another Mac shares.
    held: u64,
    verdicts: u64,
    /// Each paired Mac's id, name and whether it is linked.
    paired: Vec<(String, String, bool)>,
    /// The running build, which says whether it holds an offer.
    build: Option<(String, BTreeMap<String, String>)>,
}

/// What of a Job row held from another Mac names it: the rest of the row is not read.
#[derive(serde::Deserialize)]
struct TitleOnly {
    id: String,
    #[serde(default)]
    title: Option<String>,
}

/// One offer in the check, either Mac's, in the order they are merged.
#[derive(Clone)]
pub(crate) struct Entry {
    pub(crate) branch: String,
    pub(crate) tip: String,
    /// The Job's id where the offer is this Mac's own.
    pub(crate) own: Option<String>,
    /// The Mac's id and name where the offer is held from another.
    pub(crate) from: Option<(String, String)>,
}

/// The ready offers held from other Macs that count, and those left out.
#[derive(Clone, Default)]
pub(crate) struct Partners {
    /// Rows as Bridge draws them: `mac` set, `state` what this Mac's check found, `checking` where none yet.
    pub(crate) offers: Vec<PreviewOfferRow>,
    pub(crate) left_out: Vec<PreviewLeftOut>,
    names: BTreeMap<String, String>,
}

/// The why of an offer from another Mac whose tip is in neither this repository nor origin.
pub(crate) const NOT_ON_ORIGIN: &str = "its tip is not on origin";

/// The sentence `left_out` names an offer of `name`'s with, that cannot build here for `why`.
fn left_out_why(name: &str, branch: &str, why: &str) -> String {
    let why = why.trim().trim_end_matches('.');
    if why == NOT_ON_ORIGIN {
        return format!("{name}'s {branch} is left out of this refresh: {why}.");
    }
    format!("{name}'s {branch} is left out of this refresh: it does not build here, {why}.")
}

/// The merged offers in the order they are checked: oldest offered first, then by branch. **An own offer of a
/// branch wins** over a held one of the same name. With nothing held, this Mac's own order stands.
pub(crate) fn in_check_order(own: &[KeptPreviewOffered], partners: &Partners) -> Vec<Entry> {
    let mut merged: Vec<(String, Entry, String)> = own
        .iter()
        .map(|one| {
            let entry = Entry { branch: one.branch.clone(), tip: one.tip.clone(), own: Some(one.session_id.clone()), from: None };
            (one.offered_at.clone(), entry, one.session_id.clone())
        })
        .collect();
    let held = partners.offers.iter().filter(|row| !own.iter().any(|one| one.branch == row.branch));
    let held: Vec<(String, Entry, String)> = held
        .map(|row| {
            let mac = row.mac.clone().unwrap_or_default();
            let name = partners.names.get(&mac).cloned().unwrap_or_else(|| mac.clone());
            let entry = Entry { branch: row.branch.clone(), tip: row.tip.clone(), own: None, from: Some((mac, name)) };
            (row.offered_at.as_str().to_string(), entry, row.session_id.as_str().to_string())
        })
        .collect();
    if held.is_empty() {
        return merged.into_iter().map(|(_, entry, _)| entry).collect();
    }
    merged.extend(held);
    merged.sort_by(|a, b| (&a.0, &a.1.branch, &a.2).cmp(&(&b.0, &b.1.branch, &b.2)));
    merged.into_iter().map(|(_, entry, _)| entry).collect()
}

/// `scripts/preview`'s file: a line `<branch>\t<tip>\t<session>\t<about>` a ready offer of either Mac, oldest first.
pub(crate) fn file_of(own: &[KeptPreviewOffered], partners: &Partners) -> String {
    let mut lines: Vec<(&str, &str, String)> = own
        .iter()
        .filter(|one| one.state == "ready")
        .map(|one| (one.offered_at.as_str(), one.branch.as_str(), line(&one.branch, &one.tip, &one.session_id, &one.about)))
        .collect();
    let held = partners.offers.iter().filter(|row| row.state == PreviewState::Ready && !own.iter().any(|one| one.branch == row.branch));
    let held: Vec<(&str, &str, String)> =
        held.map(|row| (row.offered_at.as_str(), row.branch.as_str(), line(&row.branch, &row.tip, row.session_id.as_str(), &row.about))).collect();
    if !held.is_empty() {
        lines.extend(held);
        lines.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    }
    lines.into_iter().map(|(_, _, line)| line).collect()
}

fn line(branch: &str, tip: &str, session: &str, about: &str) -> String {
    format!("{branch}\t{tip}\t{session}\t{}\n", about.replace(['\t', '\n'], " "))
}

/// Whether each tip is in `root`, after one fetch of origin for the branches whose tip is not. Best effort: a
/// tip still missing is not checked, and is asked for again at the next sweep.
pub(crate) fn tips_here(root: &Path, wanted: &[Entry]) -> Vec<bool> {
    let here = |tip: &str| preview_check::commit_of(root, tip).is_some();
    let missing: Vec<String> = wanted.iter().filter(|one| one.from.is_some() && !here(&one.tip)).map(|one| one.branch.clone()).collect();
    if !missing.is_empty() {
        adapters::onto_base::fetch_branches(root, "origin", &missing);
    }
    wanted.iter().map(|one| here(&one.tip)).collect()
}

/// The first of `earlier` that changes a file of `files`, which is what an offer that failed on those files
/// clashed with.
pub(crate) fn clashed_with(root: &Path, earlier: &[Entry], files: &[String]) -> Option<Entry> {
    earlier.iter().find(|one| adapters::onto_base::files_changed(root, MAIN, &one.tip).iter().any(|file| files.contains(file))).cloned()
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
    /// The offers held from other Macs. **Every linked Mac counts**, asleep or on another build: a wire that
    /// differs does not stop a branch building here. An offer this Mac's check failed, or whose tip origin lacks, is
    /// a row as it stands and is named in `left_out`; only a ready one is in the file.
    pub(crate) async fn partners_read(&self) -> Partners {
        let macs = &self.previewing().macs;
        let (held, paired) = {
            let store = self.store().lock().await;
            (store.held_moves(), store.paired_macs().unwrap_or_default())
        };
        let paired: Vec<(String, String, bool)> = paired.into_iter().map(|one| (one.id, one.name, one.state == MacState::Linked)).collect();
        let build = self.preview_build();
        let now = PartnersFrom { held, verdicts: macs.verdict_moves.load(Ordering::SeqCst), paired, build };
        if let Some((from, partners)) = macs.partners_kept.lock().expect("not poisoned").as_ref() {
            if *from == now {
                return partners.clone();
            }
        }
        self.partners_decoded(now.build).await
    }

    /// `partners_read` from the held rows, kept with what it was read from. The read of the store and of the
    /// verdicts is the one the key is taken from, so a write after it makes the next call read again.
    async fn partners_decoded(&self, build: Option<(String, BTreeMap<String, String>)>) -> Partners {
        #[cfg(test)]
        self.previewing().macs.partners_decoded.fetch_add(1, Ordering::SeqCst);
        let (held_moves, held, paired, titled) = {
            let store = self.store().lock().await;
            let held = store.held_from_macs(FromMac::PreviewOffers).unwrap_or_default();
            // Only (Mac, id) -> title is read of the Job rows, and only where an offer is held to be titled.
            let titled = if held.is_empty() { Vec::new() } else { store.held_from_macs(FromMac::Sessions).unwrap_or_default() };
            (store.held_moves(), held, store.paired_macs().unwrap_or_default(), titled)
        };
        let from = |verdicts| PartnersFrom {
            held: held_moves,
            verdicts,
            paired: paired.iter().map(|one| (one.id.clone(), one.name.clone(), one.state == MacState::Linked)).collect(),
            build: build.clone(),
        };
        if held.is_empty() {
            let verdicts = self.previewing().macs.verdict_moves.load(Ordering::SeqCst);
            *self.previewing().macs.partners_kept.lock().expect("not poisoned") = Some((from(verdicts), Partners::default()));
            return Partners::default();
        }
        let titles: BTreeMap<(String, String), String> = titled
            .into_iter()
            .filter_map(|one| {
                let row = ipc::decode::<TitleOnly>("a Job row from another Mac", one.body.as_bytes()).ok()?;
                Some(((one.mac, row.id), row.title?))
            })
            .collect();
        let verdicts = self.previewing().macs.verdicts.lock().expect("not poisoned");
        let verdict_moves = self.previewing().macs.verdict_moves.load(Ordering::SeqCst);
        let mut found = Partners::default();
        let mut by_mac: BTreeMap<String, Vec<PreviewOfferRow>> = BTreeMap::new();
        for one in held {
            if let Ok(row) = ipc::decode::<PreviewOfferRow>("a preview offer from another Mac", one.body.as_bytes()) {
                by_mac.entry(one.mac).or_default().push(row);
            }
        }
        for (mac, rows) in by_mac {
            let Some(linked) = paired.iter().find(|one| one.id == mac && one.state == MacState::Linked) else { continue };
            let name = linked.name.clone();
            found.names.insert(mac.clone(), name.clone());
            for mut row in rows {
                let key = (mac.clone(), row.branch.clone());
                row.mac = Some(mac.clone());
                if let Some(title) = titles.get(&(mac.clone(), row.session_id.as_str().to_string())) {
                    row.title = title.clone();
                }
                row.in_build = self.build_holds(build.as_ref(), &row.branch, &row.tip);
                let kept = verdicts.get(&key).filter(|kept| kept.tip == row.tip);
                row.state = kept.map_or(PreviewState::Checking, |kept| kept.state);
                row.why = kept.and_then(|kept| kept.why.clone());
                row.files = kept.map(|kept| kept.files.clone()).unwrap_or_default();
                if row.state == PreviewState::Failed {
                    let why = left_out_why(&name, &row.branch, row.why.as_deref().unwrap_or("it did not check"));
                    found.left_out.push(PreviewLeftOut { mac: mac.clone(), name: name.clone(), why });
                }
                found.offers.push(row);
            }
        }
        drop(verdicts);
        *self.previewing().macs.partners_kept.lock().expect("not poisoned") = Some((from(verdict_moves), found.clone()));
        found
    }

    /// Keep the ready offers of Mac `mac`, in place of what was held from it. A change asks for a check of both
    /// Macs' offers together, and the answer is published.
    pub(crate) async fn preview_offers_held(self: &Arc<Self>, mac: &str, offers: Vec<PreviewOfferRow>) -> Result<(), Refusal> {
        let kept = offers
            .into_iter()
            .filter(|row| row.state == PreviewState::Ready)
            .map(|mut row| {
                row.mac = Some(mac.to_string());
                Ok((row.branch.clone(), self.encoded(&row)?))
            })
            .collect::<Result<Vec<_>, Refusal>>()?;
        let before: Vec<(String, String)> = {
            let held = self.store().lock().await.held_from_macs(FromMac::PreviewOffers).map_err(|why| self.ledger_fault(why))?;
            held.into_iter().filter(|one| one.mac == mac).map(|one| (one.id, one.body)).collect()
        };
        let mut after = kept.clone();
        after.sort();
        if before == after {
            return Ok(());
        }
        self.store().lock().await.replace_from_mac(FromMac::PreviewOffers, mac, &kept).map_err(|why| self.ledger_fault(why))?;
        self.preview_check_wanted();
        self.preview_moved().await;
        Ok(())
    }

    /// What of `offers` another Mac may merge: this Mac's own ready offers that are on origin. Never a row held
    /// from another Mac, and never a refresh or a left-out Mac, which are this Mac's to say.
    pub(crate) fn preview_offers_cut(&self, offers: PreviewOffers) -> PreviewOffers {
        let pushed = self.previewing().macs.pushed.lock().expect("not poisoned");
        let rows = offers
            .offers
            .into_iter()
            .filter(|row| row.mac.is_none() && row.state == PreviewState::Ready && pushed.contains(&(row.branch.clone(), row.tip.clone())))
            .map(|mut row| {
                row.in_build = false;
                row
            })
            .collect();
        PreviewOffers { offers: rows, refresh: None, left_out: Vec::new() }
    }

    /// Whether another Mac is paired, which is what makes an offer worth pushing.
    async fn another_mac_paired(&self) -> bool {
        self.store().lock().await.paired_macs().unwrap_or_default().iter().any(|one| one.state == MacState::Linked)
    }

    /// Whether the set of offers held from other Macs that count differs from the one the last check ran with, or
    /// one has no verdict yet.
    pub(crate) fn partners_moved(&self, partners: &Partners) -> bool {
        let now: Vec<(String, String, String)> =
            partners.offers.iter().map(|row| (row.mac.clone().unwrap_or_default(), row.branch.clone(), row.tip.clone())).collect();
        *self.previewing().macs.counted.lock().expect("not poisoned") != now || partners.offers.iter().any(|row| row.state == PreviewState::Checking)
    }

    /// Whether an own ready offer is yet to be pushed, another Mac being paired. One that would not push is not
    /// asked again until something else asks for a check.
    pub(crate) async fn pushes_owed(&self, own: &[KeptPreviewOffered]) -> bool {
        if !self.another_mac_paired().await {
            return false;
        }
        let macs = &self.previewing().macs;
        let (pushed, failed) = (macs.pushed.lock().expect("not poisoned"), macs.push_failed.lock().expect("not poisoned"));
        own.iter().filter(|one| one.state == "ready").any(|one| {
            let key = (one.branch.clone(), one.tip.clone());
            !pushed.contains(&key) && !failed.contains_key(&key)
        })
    }

    /// What a check found of the offers of other Macs, kept, and the set that was checked.
    pub(crate) fn partner_verdicts_kept(&self, found: Vec<(Entry, PreviewState, Option<String>, Vec<String>)>) {
        let macs = &self.previewing().macs;
        let mut verdicts = macs.verdicts.lock().expect("not poisoned");
        verdicts.clear();
        let mut counted = Vec::new();
        for (entry, state, why, files) in found {
            let Some((mac, _)) = entry.from else { continue };
            counted.push((mac.clone(), entry.branch.clone(), entry.tip.clone()));
            verdicts.insert((mac, entry.branch), PartnerVerdict { tip: entry.tip, state, why, files });
        }
        macs.verdict_moves.fetch_add(1, Ordering::SeqCst);
        *macs.counted.lock().expect("not poisoned") = counted;
    }

    /// Push each own offer that passed its check to origin, so another Mac can merge it. Best effort: a push that
    /// fails leaves the offer ready here, unshared, and is told to its Job once.
    pub(crate) async fn preview_pushed(self: &Arc<Self>, checkout: &Path, ready: Vec<(String, String)>) {
        let macs = &self.previewing().macs;
        let wanted = self.another_mac_paired().await;
        macs.pushed.lock().expect("not poisoned").retain(|key| ready.contains(key));
        macs.push_failed.lock().expect("not poisoned").retain(|key, _| ready.contains(key));
        if !wanted {
            return;
        }
        for (branch, tip) in ready {
            let key = (branch.clone(), tip.clone());
            if macs.pushed.lock().expect("not poisoned").contains(&key) {
                continue;
            }
            let root = checkout.to_path_buf();
            let pushed = tokio::task::spawn_blocking(move || adapters::onto_base::push_branch(&root, "origin", &branch, &tip)).await;
            match pushed {
                Ok(Ok(())) => {
                    macs.push_failed.lock().expect("not poisoned").remove(&key);
                    macs.pushed.lock().expect("not poisoned").insert(key);
                }
                Ok(Err(why)) => {
                    macs.push_failed.lock().expect("not poisoned").insert(key, why);
                }
                Err(_) => {}
            }
        }
    }

    /// Tell each Job whose offer would not push why, once a (branch, tip).
    pub(crate) async fn preview_pushes_told(self: &Arc<Self>) {
        let macs = &self.previewing().macs;
        let failed: Vec<((String, String), String)> = macs.push_failed.lock().expect("not poisoned").iter().map(|(key, why)| (key.clone(), why.clone())).collect();
        let offers = self.store().lock().await.preview_offered_all().unwrap_or_default();
        for ((branch, tip), why) in failed {
            if macs.push_told.lock().expect("not poisoned").contains(&(branch.clone(), tip.clone())) {
                continue;
            }
            let Some(offer) = offers.iter().find(|one| one.branch == branch && one.tip == tip) else { continue };
            let text = format!(
                "{branch} is on the preview list here, but it could not be pushed to origin, so your other Mac cannot merge it: {}.",
                why.trim().trim_end_matches('.')
            );
            if matches!(self.told_news_by_fleet(&offer.session_id, &text, ipc::Notice::BuildMoved).await, Ok(true)) {
                macs.push_told.lock().expect("not poisoned").insert((branch, tip));
            }
        }
    }

    /// Whether the answer last published named a Mac left out, which is something to retract once it is back.
    pub(crate) fn left_out_shown(&self) -> bool {
        self.previewing().sent_left_out()
    }

    /// Whether the cut another Mac may merge differs from the one last sent, and keep it. A push that finishes
    /// changes it though no row did.
    pub(crate) fn cut_changed(&self, now: &PreviewOffers) -> bool {
        let cut = self.preview_offers_cut(now.clone());
        let mut sent = self.previewing().macs.cut_sent.lock().expect("not poisoned");
        let changed = sent.as_ref() != Some(&cut);
        if changed {
            *sent = Some(cut);
        }
        changed
    }
}

/// The why of an own offer that failed on `files`, where it clashed with an offer held from another Mac that comes
/// before it: it names the Mac and the branch. `None` where the earlier offer is this Mac's own.
pub(crate) fn clash_why(files: &[String], with: &Entry) -> Option<String> {
    let (_, name) = with.from.as_ref()?;
    Some(format!("{CONFLICTS} {}. {name}'s {} changes it too", files.join(", "), with.branch))
}
