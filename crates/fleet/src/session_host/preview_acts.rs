//! `offer_preview` and `withdraw_preview`: a Job offers its branch for the owner's preview, and takes
//! the offer back. What Fleet does with an offer is `crate::previewing`.

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{OfferApproved, OfferPreview, PreviewOffered, PreviewState, PreviewStep, PreviewWithdrawn, WireError, WithdrawPreview, PICTURES_AT_MOST};
use store::{Holder, KeptApproved, KeptCard, KeptStep};

use crate::daemon::Fleet;
use crate::previewing::MAIN;
use crate::repositories::Served;

const NO_BRANCH: &str = "fleet.preview_no_branch";
const BRANCH_UNKNOWN: &str = "fleet.preview_branch_unknown";
const NOT_AHEAD: &str = "fleet.preview_not_ahead";
const ABOUT_NEEDED: &str = "fleet.preview_about_needed";
const ABOUT_INVALID: &str = "fleet.preview_about_invalid";
const UNAVAILABLE: &str = "fleet.preview_unavailable";
const NOT_OFFERED: &str = "fleet.preview_not_offered";
const LOOK_INVALID: &str = "fleet.preview_look_invalid";
const STEPS_INVALID: &str = "fleet.preview_steps_invalid";
const STEP_ACT_UNKNOWN: &str = "fleet.preview_step_act_unknown";
const APPROVED_REVISION: &str = "fleet.preview_approved_revision_unknown";
const PICTURES_INVALID: &str = "fleet.preview_pictures_invalid";
const WORDS_INVALID: &str = "fleet.preview_words_invalid";

/// The most his quoted words say.
const WORDS_AT_MOST: usize = 2000;
/// What a picture may be, by its extension, as `get_repository_file` reads one.
const PICTURE_KINDS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "svg"];

/// The most an `about` says.
const ABOUT_AT_MOST: usize = 120;
/// The most a `look` says.
const LOOK_AT_MOST: usize = 300;
/// The most steps a note has: Bridge binds the keys 1 to 9.
const STEPS_AT_MOST: usize = 9;
/// The most a step says.
const STEP_AT_MOST: usize = 120;

/// What a Job is told when it offers a branch with no `look`.
fn look_wanted(branch: &str) -> String {
    format!(
        "Your offer of {branch} has no look, so his preview note says nothing on where to look. Offer it again with `look`, one line on what he should look at, and `steps`, each a `say` and the `act` id from {REGISTRY} that takes him there. The preview-app skill has an example."
    )
}

/// Where the acts registry is, in the repository.
const REGISTRY: &str = "crates/core-model/domain/actions.toml";
/// The most of the registry read from a branch; it is well under this.
const REGISTRY_AT_MOST: usize = 4 << 20;

/// The ids of the acts registry, `crates/core-model/domain/actions.toml`, read once from the copy built in: each
/// `[actions.<id>]` header, read by line as the gate's `rules_actions` reads it.
fn act_ids() -> &'static BTreeSet<String> {
    static IDS: OnceLock<BTreeSet<String>> = OnceLock::new();
    IDS.get_or_init(|| act_ids_in(include_str!("../../../core-model/domain/actions.toml")))
}

fn act_ids_in(registry: &str) -> BTreeSet<String> {
    registry
        .lines()
        .filter_map(|line| line.trim().strip_prefix("[actions.")?.strip_suffix(']').map(str::to_string))
        .collect()
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
    /// `offer_preview`: keep the Job's offer of a branch at the tip it stands at, and ask for a check.
    /// **The answer is that the check has begun**: what it found reaches the Job as a notice.
    pub(crate) async fn preview_offered(self: &Arc<Self>, caller: Option<api::Caller>, offer: OfferPreview) -> Result<PreviewOffered, Refusal> {
        let id = self.waiting_session(caller, offer.session_id.as_ref())?;
        let Some((checkout, served)) = self.preview_place() else {
            return Err(self.hosted_refusal(UNAVAILABLE, "the repository this Fleet serves has no scripts/preview to offer a branch to"));
        };
        let held = self.preview_branch(&id, offer.branch.as_deref(), Some(&served)).await?;
        let cwd = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(NO_BRANCH, "a job Fleet knows"))?;
            if session.state == store::SessionState::Ended {
                return Err(self.closed(&id));
            }
            if session.manifest_id.as_deref() != Some(served.manifest().id().as_str()) {
                return Err(self.hosted_refusal(UNAVAILABLE, "a job of the repository whose preview this is"));
            }
            session.cwd
        };
        let (root, branch) = (checkout, held.clone());
        let (tip, ahead, acts) = tokio::task::spawn_blocking(move || {
            let tip = adapters::branch_tip(&root.to_string_lossy(), &branch)?;
            let ahead = !adapters::onto_base::is_ancestor(&root, &tip, MAIN);
            // The registry as the branch holds it, since that is the tree his preview build runs.
            let acts = adapters::territory::text_at(&root, &tip, REGISTRY, REGISTRY_AT_MOST)
                .ok()
                .flatten()
                .filter(|read| !read.cut)
                .map(|read| act_ids_in(&read.text));
            Some((tip, ahead, acts))
        })
        .await
        .ok()
        .flatten()
        .ok_or_else(|| self.hosted_refusal(BRANCH_UNKNOWN, &format!("a branch this repository holds: {held} is not one")))?;
        if !ahead {
            return Err(self.hosted_refusal(NOT_AHEAD, &format!("a branch with commits main lacks: {held} has none past main")));
        }
        let about = self.preview_about(&id, &held, offer.about.as_deref()).await?;
        let acts = acts.unwrap_or_else(|| act_ids().clone());
        let (look, steps) = self.preview_note(&id, &held, offer.look.as_deref(), offer.steps.as_deref(), &acts).await?;
        let card = self.preview_card(&id, &held, &cwd, &offer).await?;
        let at = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            store.keep_preview_offered(&id, &held, &tip, &about, look.as_deref(), &steps, &at).map_err(|why| self.ledger_fault(why))?;
            store.keep_build_note_card(&held, &tip, &card).map_err(|why| self.ledger_fault(why))?;
        }
        // An offer with nothing to look at leaves his preview note empty: the Job is asked for one.
        if look.is_none() {
            let _ = self.told_by_fleet(&id, &look_wanted(&held), ipc::Notice::BuildMoved).await;
        }
        self.preview_moved().await;
        self.ready_cards_moved(&id).await;
        self.preview_check_wanted();
        Ok(PreviewOffered { branch: held, tip, state: PreviewState::Checking })
    }

    /// `withdraw_preview`: take the Job's offer of a branch away and check the rest again.
    pub(crate) async fn preview_withdrawn(self: &Arc<Self>, caller: Option<api::Caller>, withdraw: WithdrawPreview) -> Result<PreviewWithdrawn, Refusal> {
        let id = self.waiting_session(caller, withdraw.session_id.as_ref())?;
        let served = self.preview_place().map(|(_, served)| served);
        let branch = self.preview_branch(&id, withdraw.branch.as_deref(), served.as_ref()).await?;
        let at = self.now().as_str().to_string();
        // The tip the running build holds the branch at, which the next refresh would otherwise carry forward.
        let held = self.preview_build().and_then(|(_, tips)| tips.get(&branch).cloned());
        let removed = {
            let mut store = self.store().lock().await;
            let offered = store.preview_offered_of(&id, &branch).map_err(|why| self.ledger_fault(why))?;
            // Its card says so: the note of the tip the offer stood at.
            if let Some(offered) = &offered {
                store.keep_build_note_withdrawn(&branch, &offered.tip, &at).map_err(|why| self.ledger_fault(why))?;
            }
            let removed = store.remove_preview_offered(&id, &branch).map_err(|why| self.ledger_fault(why))?;
            // A branch whose offer already left the list still holds its open cards: they are taken back too.
            let noted = store.keep_build_notes_withdrawn_of(&id, &branch, &at).map_err(|why| self.ledger_fault(why))?;
            if removed || noted > 0 {
                for tip in offered.iter().map(|one| one.tip.clone()).chain(held) {
                    store.keep_preview_withdrawn(&branch, &tip, &at).map_err(|why| self.ledger_fault(why))?;
                }
            }
            removed || noted > 0
        };
        if !removed {
            return Err(Refusal::NotFound(WireError::raised(NOT_OFFERED, format!("{branch} is not on the preview list from this job"), self.run_id())));
        }
        self.preview_moved().await;
        self.ready_cards_moved(&id).await;
        self.preview_check_wanted();
        Ok(PreviewWithdrawn { branch })
    }

    /// The branch named, or the one branch the Job holds a slot on.
    async fn preview_branch(&self, id: &str, named: Option<&str>, served: Option<&Served>) -> Result<String, Refusal> {
        if let Some(named) = named.map(str::trim).filter(|named| !named.is_empty()) {
            return Ok(named.to_string());
        }
        let held = match served {
            Some(served) => crate::build_notices::branches_held(&*self.store().lock().await, served).remove(id).unwrap_or_default(),
            None => Default::default(),
        };
        let mut held = held.into_iter();
        match (held.next(), held.next()) {
            (Some(only), None) => Ok(only),
            (None, _) => Err(self.hosted_refusal(NO_BRANCH, "a branch to name in `branch`: this job holds none")),
            (Some(_), Some(_)) => Err(self.hosted_refusal(NO_BRANCH, "a branch to name in `branch`: this job holds more than one")),
        }
    }

    /// What to look at and the steps there: the ones given, checked, else the ones the last offer kept. **An empty
    /// `look` clears it and an empty list of steps clears them**; none given keeps.
    async fn preview_note(&self, id: &str, branch: &str, look: Option<&str>, steps: Option<&[PreviewStep]>, acts: &BTreeSet<String>) -> Result<(Option<String>, Vec<KeptStep>), Refusal> {
        let look = match look.map(str::trim) {
            Some(look) if look.chars().count() > LOOK_AT_MOST || look.contains(['\n', '\r']) => {
                return Err(self.hosted_refusal(LOOK_INVALID, &format!("look as one line of at most {LOOK_AT_MOST} characters")));
            }
            given => given,
        };
        let given = steps.map(|steps| self.preview_steps(steps, acts)).transpose()?;
        let kept = self.store().lock().await.preview_offered_of(id, branch).map_err(|why| self.ledger_fault(why))?;
        let look = match look {
            Some(look) => Some(look.to_string()).filter(|look| !look.is_empty()),
            None => kept.as_ref().and_then(|one| one.look.clone()),
        };
        let steps = given.unwrap_or_else(|| kept.map(|one| one.steps).unwrap_or_default());
        Ok((look, steps))
    }

    /// `steps` as kept, or the refusal naming what is wrong with them. `acts` are the registry's ids at the offered tip.
    fn preview_steps(&self, steps: &[PreviewStep], acts: &BTreeSet<String>) -> Result<Vec<KeptStep>, Refusal> {
        if steps.len() > STEPS_AT_MOST {
            return Err(self.hosted_refusal(STEPS_INVALID, &format!("at most {STEPS_AT_MOST} steps: {} were given", steps.len())));
        }
        let mut kept = Vec::new();
        for (at, step) in steps.iter().enumerate() {
            let say = step.say.trim();
            if say.is_empty() || say.chars().count() > STEP_AT_MOST || say.contains(['\n', '\r']) {
                return Err(self.hosted_refusal(STEPS_INVALID, &format!("step {} with a `say` of one line of at most {STEP_AT_MOST} characters", at + 1)));
            }
            let act = step.act.as_deref().map(str::trim).filter(|act| !act.is_empty());
            if let Some(act) = act.filter(|act| !acts.contains(*act)) {
                return Err(self.hosted_refusal(STEP_ACT_UNKNOWN, &format!("an act id from {REGISTRY} on the offered branch: step {} names `{act}`", at + 1)));
            }
            kept.push(KeptStep { say: say.to_string(), act: act.map(str::to_string) });
        }
        Ok(kept)
    }

    /// What the offer gives its Ready to try card: each part given, checked, else the one the Job's last offer of
    /// the branch kept. **Empty pictures or words clear them**; none given keeps.
    async fn preview_card(&self, id: &str, branch: &str, cwd: &str, offer: &OfferPreview) -> Result<KeptCard, Refusal> {
        let kept = self.store().lock().await.build_note_newest(id, branch).map_err(|why| self.ledger_fault(why))?.map(|one| one.card).unwrap_or_default();
        let approved = match &offer.approved {
            Some(named) => Some(self.preview_approved(id, named).await?),
            None => kept.approved,
        };
        let pictures = match &offer.pictures {
            Some(given) => self.preview_pictures(cwd, given)?,
            None => kept.pictures,
        };
        let words = match offer.words.as_deref().map(str::trim) {
            Some(words) if words.chars().count() > WORDS_AT_MOST => {
                return Err(self.hosted_refusal(WORDS_INVALID, &format!("words of at most {WORDS_AT_MOST} characters")));
            }
            Some(words) => Some(words.to_string()).filter(|words| !words.is_empty()),
            None => kept.words,
        };
        Ok(KeptCard { approved, pictures, words })
    }

    /// The artifact the Job names as the one he approved, as it reads now: refused where the Job holds none by
    /// that id or title, or no such revision.
    async fn preview_approved(&self, id: &str, named: &OfferApproved) -> Result<KeptApproved, Refusal> {
        let artifact = self.artifact_named(id, &named.artifact).await?;
        let artifact = self.artifact_held(id, &artifact).await?;
        let newest = artifact.revisions.last().map_or(1, |one| one.number);
        let revision = match named.revision {
            None => newest,
            Some(asked) if artifact.revisions.iter().any(|one| one.number == asked) => asked,
            Some(asked) => {
                return Err(self.hosted_refusal(APPROVED_REVISION, &format!("a revision `{}` holds: {asked} is not one, and {newest} is the newest", artifact.id)));
            }
        };
        // When he approved it is what the ledger kept of his answer.
        let ledger = self.store().lock().await.attachments_of(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
        let approved_at = ledger
            .iter()
            .find(|one| one.target == artifact.id && one.detail.get("settled").map(String::as_str) == Some("approved"))
            .and_then(|one| one.detail.get("settled_at").cloned());
        let form = match artifact.form {
            ipc::ArtifactForm::Sketch => "sketch",
            ipc::ArtifactForm::Draft => "draft",
            ipc::ArtifactForm::Page => "page",
        };
        Ok(KeptApproved { artifact: artifact.id, title: artifact.title, form: form.to_string(), revision, approved_at })
    }

    /// The pictures, each a picture file, resolved against the Job's checkout where relative.
    fn preview_pictures(&self, cwd: &str, given: &[String]) -> Result<Vec<String>, Refusal> {
        if given.len() > PICTURES_AT_MOST {
            return Err(self.hosted_refusal(PICTURES_INVALID, &format!("at most {PICTURES_AT_MOST} pictures: {} were given", given.len())));
        }
        given
            .iter()
            .map(|path| {
                let path = path.trim();
                let at = std::path::Path::new(cwd).join(path);
                let kind = at.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase);
                let picture = kind.is_some_and(|kind| PICTURE_KINDS.contains(&kind.as_str()));
                if path.is_empty() || !picture || !at.is_file() {
                    return Err(self.hosted_refusal(PICTURES_INVALID, &format!("pictures that are png, jpeg, gif, webp or svg files: `{path}` is not one")));
                }
                Ok(at.to_string_lossy().into_owned())
            })
            .collect()
    }

    /// The line on what the branch holds: the one given, else the one kept from the last offer.
    async fn preview_about(&self, id: &str, branch: &str, given: Option<&str>) -> Result<String, Refusal> {
        let given = given.map(str::trim).filter(|about| !about.is_empty());
        if let Some(about) = given {
            return if about.chars().count() > ABOUT_AT_MOST || about.contains(['\n', '\r']) {
                Err(self.hosted_refusal(ABOUT_INVALID, &format!("about as one line of at most {ABOUT_AT_MOST} characters")))
            } else {
                Ok(about.to_string())
            };
        }
        let kept = self.store().lock().await.preview_offered_of(id, branch).map_err(|why| self.ledger_fault(why))?;
        kept.map(|one| one.about).ok_or_else(|| self.hosted_refusal(ABOUT_NEEDED, "about, one line on what this branch holds, with a first offer of it"))
    }
}
