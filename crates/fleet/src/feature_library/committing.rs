//! A commit names the feature it serves. **The mod's hook asks before `git commit` runs**, and Fleet refuses a commit whose
//! message line carries no `Feature:` line, saying which features the files it takes belong to so the agent can copy one.
//!
//! **Only a Job started after its project's catalog was seeded is refused**: approving a plan is not consent to change a
//! Job already running, so a Job older than the catalog's first feature is never held to it.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{CheckCommitFeature, CommitFeatureChecked};
use store::{SessionState, Store};

use super::reading::Loaded;
use crate::daemon::Fleet;

/// The most features a refusal lists; the rest are counted.
const MOST_LISTED: usize = 5;

/// Whether the line is a `git commit` that gives its message inline, with `-m` or `--message`, so the message can be read.
/// A commit that takes its message from a file, another commit or an editor is not read, and so not refused.
pub(crate) fn gives_message(command: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    let Some(commit) = words.iter().enumerate().position(|(at, word)| *word == "commit" && words[..at].contains(&"git")) else {
        return false;
    };
    words[commit + 1..].iter().any(|word| {
        if let Some(long) = word.strip_prefix("--") {
            return long == "message" || long.starts_with("message=");
        }
        // A cluster of short flags, such as `-am`, ends where a letter does not: `-m"text"` is `-m`.
        word.strip_prefix('-').is_some_and(|flags| flags.chars().take_while(char::is_ascii_alphabetic).any(|flag| flag == 'm'))
    })
}

/// What an agent is told when its commit names no feature. **Each feature is a line to copy whole**: a trailer's slug
/// stops at its comma, so nothing but the slug follows `Feature:`.
fn refusal(owning: &[String]) -> String {
    let mut says = String::from("No Feature: line in this commit. Fleet files each commit under the features it serves.\n");
    if owning.is_empty() {
        says.push_str("No feature owns the files you are committing. Name the feature this commit serves, or the one that does not exist yet and the Operator will be asked to confirm it. list_features names every feature.\n");
    } else {
        says.push_str("The files you are committing belong to these features, the most files first:\n");
        for slug in owning.iter().take(MOST_LISTED) {
            says.push_str(&format!("Feature: {slug}\n"));
        }
        if owning.len() > MOST_LISTED {
            says.push_str(&format!("and {} more.\n", owning.len() - MOST_LISTED));
        }
    }
    says.push_str("Add the line you mean to the last paragraph of the message, one or more slugs separated by commas, and commit again.");
    says
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
    /// `check_commit_feature`.
    pub(super) async fn commit_feature_checked(&self, check: CheckCommitFeature) -> Result<CommitFeatureChecked, Refusal> {
        let allowed = CommitFeatureChecked { allowed: true, says: String::new(), features: Vec::new() };
        self.commit_words_flagged(&check).await;
        if !gives_message(&check.command) {
            return Ok(allowed);
        }
        let job = check.session_id.as_str().trim().to_string();
        let named = adapters::feature_commits::feature_lines_in(&check.command);
        if !named.is_empty() {
            let mut store = self.store().lock().await;
            self.naming_kept(&mut store, &job, &named);
            return Ok(allowed);
        }
        let store = self.store().lock().await;
        let Some(session) = store.session(&job).map_err(|why| self.ledger_fault(why))? else {
            return Ok(allowed);
        };
        if session.state == SessionState::Ended {
            return Ok(allowed);
        }
        let manifest = session.manifest_id.clone().filter(|manifest| !manifest.is_empty());
        let mut owning: BTreeMap<String, usize> = BTreeMap::new();
        let mut held_to = false;
        for project in self.projects_of_job_in(&store, &job) {
            let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
            // The catalog was seeded when its first feature entered it. A Job started before that is never refused.
            let Some(seeded) = loaded.map.features().iter().map(|feature| feature.added_at.as_str()).filter(|at| !at.is_empty()).min() else {
                continue;
            };
            if seeded > session.started_at.as_str() {
                continue;
            }
            held_to = true;
            for file in &check.files {
                for owner in loaded.map.owners(manifest.as_deref(), file.trim().trim_start_matches("./")) {
                    *owning.entry(owner.feature.slug.clone()).or_default() += 1;
                }
            }
        }
        if !held_to || check.files.is_empty() {
            return Ok(allowed);
        }
        let mut owning: Vec<(String, usize)> = owning.into_iter().collect();
        owning.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let features: Vec<String> = owning.into_iter().map(|(slug, _)| slug).collect();
        Ok(CommitFeatureChecked { allowed: false, says: refusal(&features), features })
    }

    /// The features a commit the gate let through named, kept against the Job: **what the Job owes sections for**. A slug the
    /// library lacks is a pending feature, as a trailer's is. A repeat keeps the first naming.
    fn naming_kept(&self, store: &mut Store, job: &str, slugs: &[String]) {
        let Ok(Some(session)) = store.session(job) else {
            return;
        };
        if session.state == SessionState::Ended {
            return;
        }
        let now = self.now().as_str().to_string();
        for project in self.projects_of_job_in(store, job) {
            let Ok(loaded) = Loaded::read(store, &project) else {
                continue;
            };
            for slug in slugs {
                if let Some(feature) = self.feature_or_pending(store, &loaded, slug, &now) {
                    let _ = store.keep_feature_job_named(&feature.id, job, &now);
                }
            }
        }
    }
}
