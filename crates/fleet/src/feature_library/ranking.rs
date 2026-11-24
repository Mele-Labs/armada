//! The issues he might pick up and the pull requests he might be interested in: each mapped to the features it touches
//! and ranked by how many of the owner's Jobs worked on those features lately. `docs/concepts/feature-catalog.md`,
//! *Ranking the issues he might pick up*.
//!
//! **No model and no forge call of its own.** The issue lists are the ones the tracker was last read for, the pull
//! requests are the ones the review lists left with their files, and the Jobs are what the library recorded. The first
//! half of this file needs no Fleet: what a row matches, which Jobs count, and the order the rows go in.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Issues, Refusal};
use core_model::Timestamp;
use ipc::{ManifestId, MatchedBy, MatchedFeature, RankedIssue, RankedIssues, RankedPull};
use store::{feature_state, KeptFeature, KeptOpenPull};

use super::mapping::FeatureMap;
use crate::daemon::Fleet;

/// How far back a Job counts, in days.
pub(crate) const DAYS: u32 = 30;
const DAY_MS: i64 = 86_400_000;

/// The Jobs that filed a file under each feature within [`DAYS`] of `now` (epoch milliseconds), by feature id. A time that
/// does not read counts for nothing, so a row Fleet cannot date never lifts a feature.
pub(crate) fn recent_jobs(times: &[(String, String, String)], now: i64) -> BTreeMap<String, BTreeSet<String>> {
    let since = now - i64::from(DAYS) * DAY_MS;
    let mut recent: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (feature, job, last_at) in times {
        if Timestamp::from_rfc3339(last_at.as_str()).epoch_millis().is_some_and(|at| at >= since) {
            recent.entry(feature.clone()).or_default().insert(job.clone());
        }
    }
    recent
}

/// What a row is matched against.
pub(crate) struct Ground<'a> {
    pub(crate) map: &'a FeatureMap,
    /// The Jobs that filed a file under each feature in the window, by feature id.
    pub(crate) recent: &'a BTreeMap<String, BTreeSet<String>>,
    /// The project's open pull requests, with their files.
    pub(crate) pulls: &'a [KeptOpenPull],
}

/// The matches of one row as they build up, by feature id.
#[derive(Default)]
struct Matches<'a>(BTreeMap<&'a str, (&'a KeptFeature, BTreeSet<MatchedBy>)>);

impl<'a> Matches<'a> {
    fn add(&mut self, feature: &'a KeptFeature, by: MatchedBy) {
        self.0.entry(feature.id.as_str()).or_insert_with(|| (feature, BTreeSet::new())).1.insert(by);
    }

    /// The matched features, the most Jobs first and then by title, and the Jobs across them each counted once.
    fn settle(self, recent: &BTreeMap<String, BTreeSet<String>>) -> (Vec<MatchedFeature>, u32) {
        let mut jobs: BTreeSet<&str> = BTreeSet::new();
        let mut found: Vec<MatchedFeature> = self
            .0
            .into_values()
            .map(|(feature, by)| {
                let here = recent.get(&feature.id);
                jobs.extend(here.into_iter().flatten().map(String::as_str));
                MatchedFeature {
                    slug: feature.slug.clone(),
                    title: feature.title.clone(),
                    by: by.into_iter().collect(),
                    jobs: here.map_or(0, |here| u32::try_from(here.len()).unwrap_or(u32::MAX)),
                }
            })
            .collect();
        found.sort_by(|a, b| b.jobs.cmp(&a.jobs).then_with(|| a.title.cmp(&b.title)).then_with(|| a.slug.cmp(&b.slug)));
        (found, u32::try_from(jobs.len()).unwrap_or(u32::MAX))
    }
}

/// `text` with its case and separators taken off, so `Project-Dashboard` and `project dashboard` read alike.
fn plain(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()).map(str::to_lowercase).collect::<Vec<_>>().join(" ")
}

/// Whether `name` stands in `text` as whole words, both being [`plain`].
fn names(text: &str, name: &str) -> bool {
    !name.is_empty() && format!(" {text} ").contains(&format!(" {name} "))
}

/// Whether `title` names the issue `reference`, as `#123` or `ABC-123` does: whole, so `#12` is not in `#123`.
pub(crate) fn mentions(title: &str, reference: &str) -> bool {
    let (title, reference) = (title.to_lowercase(), reference.trim().to_lowercase());
    if reference.is_empty() {
        return false;
    }
    let word = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    title.match_indices(&reference).any(|(at, found)| {
        let before = title[..at].chars().next_back();
        let after = title[at + found.len()..].chars().next();
        before.is_none_or(|c| !word(c)) && after.is_none_or(|c| !word(c))
    })
}

fn live(feature: &&KeptFeature) -> bool {
    feature.state != feature_state::PENDING
}

impl Ground<'_> {
    /// The features an issue matches: by the files of an open pull request that names it, a label and its own title.
    /// `manifest` is the repository it was listed from, and a pull request of another repository does not name it.
    pub(crate) fn issue(&self, manifest: Option<&str>, issue: &ipc::TrackedIssue) -> (Vec<MatchedFeature>, u32) {
        let mut found = Matches::default();
        for pull in self.pulls.iter().filter(|pull| manifest.is_none_or(|one| one == pull.manifest_id) && mentions(&pull.title, &issue.reference)) {
            for file in &pull.files {
                for owner in self.map.owners(Some(&pull.manifest_id), file).into_iter().filter(|owner| live(&owner.feature)) {
                    found.add(owner.feature, MatchedBy::PullRequestFiles);
                }
            }
        }
        let labels: Vec<String> = issue.labels.iter().map(|label| plain(label)).collect();
        let title = plain(&issue.title);
        for feature in self.map.features().iter().filter(live) {
            let (slug, area) = (plain(&feature.slug), plain(&feature.area));
            if labels.iter().any(|label| *label == slug || *label == area) {
                found.add(feature, MatchedBy::Label);
            }
            if names(&title, &slug) || names(&title, &plain(&feature.title)) {
                found.add(feature, MatchedBy::Title);
            }
        }
        found.settle(self.recent)
    }

    /// The features a pull request's own files fall in.
    pub(crate) fn pull(&self, pull: &KeptOpenPull) -> (Vec<MatchedFeature>, u32) {
        let mut found = Matches::default();
        for file in &pull.files {
            for owner in self.map.owners(Some(&pull.manifest_id), file).into_iter().filter(|owner| live(&owner.feature)) {
                found.add(owner.feature, MatchedBy::Files);
            }
        }
        found.settle(self.recent)
    }
}

/// The order of the issues: those that matched a feature before those that matched none, then the most Jobs, then the
/// newest, then the reference so the order never wobbles.
pub(crate) fn order_issues(rows: &mut [RankedIssue]) {
    rows.sort_by(|a, b| {
        a.features
            .is_empty()
            .cmp(&b.features.is_empty())
            .then_with(|| b.jobs.cmp(&a.jobs))
            .then_with(|| b.issue.updated_at.as_str().cmp(a.issue.updated_at.as_str()))
            .then_with(|| a.issue.reference.cmp(&b.issue.reference))
    });
}

/// The order of the pull requests: those that matched a feature first, then the most Jobs, then the newest number.
pub(crate) fn order_pulls(rows: &mut [RankedPull]) {
    rows.sort_by(|a, b| {
        a.features.is_empty().cmp(&b.features.is_empty()).then_with(|| b.jobs.cmp(&a.jobs)).then_with(|| b.number.cmp(&a.number))
    });
}

/// An issue he might pick up: open, assigned to nobody, held by no Job and closed by no pull request.
pub(crate) fn pickable(issue: &ipc::TrackedIssue) -> bool {
    issue.state == ipc::IssueState::Open && issue.assignees.is_empty() && issue.holders.is_empty() && !issue.linked_pull_request
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
    /// `list_ranked_issues`. **A repository whose tracker would not answer is left out** while another did, and the
    /// first refusal is the answer only where none did.
    pub(crate) async fn ranked_issues_read(&self, project: String) -> Result<RankedIssues, Refusal> {
        let held = self.project_held(&project).await?;
        let members: Vec<String> = held.repositories().into_iter().map(|member| member.as_str().to_string()).collect();
        let mut listed: Vec<(Option<ManifestId>, ipc::TrackedIssue)> = Vec::new();
        let mut fetched: Option<ipc::Instant> = None;
        let mut oldest = |at: ipc::Instant| {
            if fetched.as_ref().is_none_or(|one| at.as_str() < one.as_str()) {
                fetched = Some(at);
            }
        };
        if held.values().tracker.is_some() {
            let list = self.project_issues_listed(project.clone(), false).await?;
            oldest(list.fetched_at);
            listed.extend(list.issues.into_iter().map(|issue| (None, issue)));
        } else {
            let mut refused: Option<Refusal> = None;
            let mut answered = false;
            for member in members.iter().filter(|member| self.repositories().serving(member).is_some()) {
                let manifest = ManifestId::carried(member);
                match self.list_issues(manifest.clone(), false).await {
                    Ok(list) => {
                        answered = true;
                        oldest(list.fetched_at);
                        listed.extend(list.issues.into_iter().map(|issue| (Some(manifest.clone()), issue)));
                    }
                    Err(refusal) => refused = refused.or(Some(refusal)),
                }
            }
            if let (false, Some(refusal)) = (answered, refused) {
                return Err(refusal);
            }
        }
        let mine = self.owner_logins().await;
        let now = self.now().epoch_millis().unwrap_or(0);
        let (loaded, pulls, times) = {
            let store = self.store().lock().await;
            let loaded = super::reading::Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
            let mut pulls = store.open_pulls_kept().map_err(|why| self.ledger_fault(why))?;
            pulls.retain(|pull| members.contains(&pull.manifest_id));
            let times = store.feature_job_times(&project).map_err(|why| self.ledger_fault(why))?;
            (loaded, pulls, times)
        };
        let recent = recent_jobs(&times, now);
        let ground = Ground { map: &loaded.map, recent: &recent, pulls: &pulls };
        let mut might_pick_up: Vec<RankedIssue> = listed
            .into_iter()
            .filter(|(_, issue)| pickable(issue))
            .map(|(manifest_id, issue)| {
                let (features, jobs) = ground.issue(manifest_id.as_ref().map(ManifestId::as_str), &issue);
                RankedIssue { manifest_id, issue, features, jobs, why: None }
            })
            .collect();
        order_issues(&mut might_pick_up);
        // Nobody can be called somebody else without knowing who he is.
        let mut might_be_interested: Vec<RankedPull> = match mine.login() {
            None => Vec::new(),
            Some(_) => pulls
                .iter()
                .filter(|pull| !mine.knows(Some(&pull.author)))
                .map(|pull| {
                    let (features, jobs) = ground.pull(pull);
                    RankedPull {
                        manifest_id: ManifestId::carried(&pull.manifest_id),
                        repository: pull.repository.clone(),
                        number: pull.number,
                        title: pull.title.clone(),
                        author: pull.author.clone(),
                        draft: pull.draft,
                        features,
                        jobs,
                        why: None,
                    }
                })
                .collect(),
        };
        order_pulls(&mut might_be_interested);
        Ok(RankedIssues { project, days: DAYS, fetched_at: fetched, might_pick_up, might_be_interested, by_librarian: None })
    }
}
