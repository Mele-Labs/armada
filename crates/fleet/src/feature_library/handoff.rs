//! Handing a Job the pages of the features it works in, **by mechanism and once**. The mod's hook asks as a Job starts and
//! before it reads or searches a file; Fleet answers each feature's condensed page the first time and nothing after, so
//! the agent never has to remember to ask. "Already handed" is a row per Job, holder and feature, not a thing the mod
//! keeps, so it holds across turns and a restart.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FeaturesHandedOver, HandOverFeatures};
use store::{KeptFeature, SessionState};

use super::reading::Loaded;
use crate::daemon::Fleet;
use crate::my_pulls::Mine;

/// How the page came to be handed over, as `feature_handoffs.via` names it.
const BRIEF: &str = "brief";
const READ: &str = "read";

/// The most pages one call hands over. A file several features own hands the rest over at the next read that reaches them.
pub(crate) const MOST_A_CALL: usize = 3;

const HEADER: &str = "Feature pages from Armada's library, handed once per feature:";

/// A path as the map reads it: no leading `./`, no trailing slash.
fn tidy(path: &str) -> &str {
    path.trim().trim_start_matches("./").trim_end_matches('/')
}

/// The slugs a brief names: `Feature:` lines and backticked words, in the order written, each once.
pub(crate) fn named_in(brief: &str) -> Vec<String> {
    let mut named = adapters::feature_commits::feature_lines_in(brief);
    for slug in adapters::feature_commits::backticked_slugs_in(brief) {
        if !named.contains(&slug) {
            named.push(slug);
        }
    }
    named
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
    /// `hand_over_features`. **Quiet where there is nothing to hand**: a Job Fleet does not know, an ended one and a project
    /// with no library answer an empty text and keep nothing.
    pub(super) async fn features_handed_over(&self, hand: HandOverFeatures) -> Result<FeaturesHandedOver, Refusal> {
        let nothing = FeaturesHandedOver { features: Vec::new(), text: String::new() };
        let job = hand.session_id.as_str().trim().to_string();
        if job.is_empty() || (hand.paths.is_empty() && hand.brief.as_deref().is_none_or(|brief| brief.trim().is_empty())) {
            return Ok(nothing);
        }
        let holder = hand.agent_id.as_deref().map(str::trim).unwrap_or_default().to_string();
        // A file the Job wrote is mapped by the filing, so it is read after the writes are filed.
        self.feature_writes_filed().await;
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let Some(session) = store.session(&job).map_err(|why| self.ledger_fault(why))? else {
            return Ok(nothing);
        };
        if session.state == SessionState::Ended {
            return Ok(nothing);
        }
        let manifest = session.manifest_id.clone().filter(|manifest| !manifest.is_empty());
        let before = store.feature_handoffs(&job, &holder).map_err(|why| self.ledger_fault(why))?;
        let mut features = Vec::new();
        let mut pages = Vec::new();
        // Asked once, and only when a page is to be handed: the hook asks before every file read.
        let mut mine: Option<Mine> = None;
        for project in self.projects_of_job_in(&store, &job) {
            if !store.project_has_features(&project).map_err(|why| self.ledger_fault(why))? {
                continue;
            }
            let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
            // What the brief names comes first, then what the files belong to.
            let mut candidates: Vec<(KeptFeature, &str)> = Vec::new();
            for slug in hand.brief.as_deref().map(named_in).unwrap_or_default() {
                if let Some(feature) = loaded.feature(&slug) {
                    candidates.push((feature.clone(), BRIEF));
                }
            }
            for path in &hand.paths {
                for owner in loaded.map.owners(manifest.as_deref(), tidy(path)) {
                    candidates.push((owner.feature.clone(), READ));
                }
            }
            for (feature, via) in candidates {
                if features.len() >= MOST_A_CALL {
                    break;
                }
                if before.contains(&feature.id) || features.contains(&feature.slug) {
                    continue;
                }
                // The page is read before it is counted, so one that could not be read is owed again.
                if mine.is_none() {
                    mine = Some(self.owner_logins().await);
                }
                let page = self.feature_brief_of(&store, &loaded, &feature, mine.get_or_insert_with(|| Mine::of([])))?;
                if store.keep_feature_handoff(&job, &holder, &feature.id, via, &now).map_err(|why| self.ledger_fault(why))? {
                    features.push(feature.slug.clone());
                    pages.push(page.text);
                }
            }
        }
        let text = match pages.is_empty() {
            true => String::new(),
            false => format!("{HEADER}\n\n{}", pages.join("\n\n")),
        };
        Ok(FeaturesHandedOver { features, text })
    }
}
