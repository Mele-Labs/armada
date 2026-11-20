//! A project's glossary, kept by the librarian and filled by the Jobs that introduce a word. **No model is read here.**
//! `docs/concepts/feature-catalog.md`, *Glossary*.
//!
//! A Job's terms arrive with `write_feature_sections` and Armada's own with the seed (`seeding`). A word a Job uses against
//! a term is **flagged, never refused**: a commit message it is about to make is read for the `not` words of its project's
//! glossary, and each found is filed as a link on the features the Job is on.

use std::collections::BTreeSet;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{CheckCommitFeature, Glossary, GlossaryEntry, GlossaryTermWrite};
use store::{feature_link_kind, KeptFeatureLink, KeptTerm, SessionState, Store, WriteError};

use super::committing::gives_message;
use super::reading::Loaded;
use crate::daemon::Fleet;

pub(super) const BLANK: &str = "fleet.glossary_blank";

/// What can sit inside a word: letters, digits, and the marks that join an identifier or a path.
fn part_of_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '/')
}

/// Whether `word` stands whole in `text`, in any case: `dashboard` is in "the dashboard shows" and not in `dashboard_view`.
pub(super) fn uses(text: &str, word: &str) -> bool {
    let (text, word) = (text.to_lowercase(), word.trim().to_lowercase());
    if word.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(found) = text[from..].find(&word) {
        let (start, end) = (from + found, from + found + word.len());
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if !before.is_some_and(part_of_word) && !after.is_some_and(part_of_word) {
            return true;
        }
        from = start + text[start..].chars().next().map_or(1, char::len_utf8);
    }
    false
}

/// The part of a `git commit` line that is the commit's own words: from the `commit` word on, so a path given to `git add`
/// before it is not read.
fn commit_words(command: &str) -> &str {
    command
        .match_indices("commit")
        .find(|(at, _)| command[..*at].ends_with(char::is_whitespace) && command[at + "commit".len()..].starts_with(char::is_whitespace))
        .map_or(command, |(at, _)| &command[at..])
}

/// Keep each term a Job wrote in `project`, replacing the one held under the same word in any case. The words as kept.
pub(super) fn terms_kept(store: &mut Store, project: &str, job: &str, now: &str, terms: &[GlossaryTermWrite]) -> Result<Vec<String>, WriteError> {
    let mut kept = Vec::new();
    for one in terms {
        let mut not: Vec<String> = Vec::new();
        for word in one.not.iter().map(|word| word.trim()).filter(|word| !word.is_empty()) {
            if !not.iter().any(|held| held.eq_ignore_ascii_case(word)) {
                not.push(word.to_string());
            }
        }
        store.put_glossary_term(&KeptTerm {
            project: project.to_string(),
            term: one.term.trim().to_string(),
            meaning: one.meaning.trim().to_string(),
            not,
            job: job.to_string(),
            at: now.to_string(),
        })?;
        kept.push(one.term.trim().to_string());
    }
    Ok(kept)
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
    /// `get_glossary`.
    pub(super) async fn glossary_read(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<Glossary, Refusal> {
        let store = self.store().lock().await;
        let projects = self.projects_asked(&store, caller, project.as_deref())?;
        // A Job in several projects reads the first that has a glossary.
        let mut chosen = projects[0].clone();
        let mut terms = store.glossary(&chosen).map_err(|why| self.ledger_fault(why))?;
        for one in &projects[1..] {
            if !terms.is_empty() {
                break;
            }
            terms = store.glossary(one).map_err(|why| self.ledger_fault(why))?;
            chosen = one.clone();
        }
        let terms = terms
            .into_iter()
            .map(|term| GlossaryEntry { term: term.term, meaning: term.meaning, not: term.not, job: term.job, at: ipc::Instant::carried(term.at) })
            .collect();
        Ok(Glossary { project: chosen, terms })
    }

    /// A commit a Job is about to make, read for the words its project's glossary says not to use. **A flag, never a
    /// refusal**: what is found is filed as a `glossary_flag` link on each feature the Job is on, naming the word, the
    /// term to use and the Job, and nothing here can fail the commit.
    pub(super) async fn commit_words_flagged(&self, check: &CheckCommitFeature) {
        if !gives_message(&check.command) {
            return;
        }
        let job = check.session_id.as_str().trim().to_string();
        let words = commit_words(&check.command).to_string();
        let found: Vec<(String, Vec<(String, String)>)> = {
            let store = self.store().lock().await;
            match store.session(&job) {
                Ok(Some(session)) if session.state != SessionState::Ended => {}
                _ => return,
            }
            let mut found = Vec::new();
            for project in self.projects_of_job_in(&store, &job) {
                let hits: Vec<(String, String)> = store
                    .glossary(&project)
                    .unwrap_or_default()
                    .into_iter()
                    .flat_map(|term| term.not.into_iter().filter(|word| uses(&words, word)).map(move |word| (word, term.term.clone())).collect::<Vec<_>>())
                    .collect();
                if !hits.is_empty() {
                    found.push((project, hits));
                }
            }
            found
        };
        if found.is_empty() {
            return;
        }
        self.feature_writes_filed().await;
        let now = self.now().as_str().to_string();
        let named = adapters::feature_commits::feature_lines_in(&check.command);
        let mut store = self.store().lock().await;
        let linked = store.feature_links_of_job(&job, feature_link_kind::JOB).unwrap_or_default();
        for (project, hits) in found {
            let Ok(loaded) = Loaded::read(&store, &project) else { continue };
            let mut on: BTreeSet<String> = linked.iter().map(|link| link.feature_id.clone()).filter(|id| loaded.map.features().iter().any(|feature| feature.id == *id)).collect();
            on.extend(named.iter().filter_map(|slug| loaded.feature_named(slug)).map(|feature| feature.id.clone()));
            for feature_id in on {
                for (word, term) in &hits {
                    let _ = store.put_feature_link(&KeptFeatureLink {
                        feature_id: feature_id.clone(),
                        kind: feature_link_kind::GLOSSARY_FLAG.to_string(),
                        target: word.clone(),
                        job: job.clone(),
                        title: term.clone(),
                        gist: String::new(),
                        gist_by: String::new(),
                        state: String::new(),
                        via: "commit".to_string(),
                        at: now.clone(),
                    });
                }
            }
        }
    }
}
