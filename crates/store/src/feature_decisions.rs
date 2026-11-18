//! What the owner settled in a Job, kept for the feature pages. `docs/concepts/feature-catalog.md`, *Decisions*.
//!
//! **A decision is kept once, with the Job and the artifact it answered, and read under a feature.** Which features it
//! belongs to is worked out when a page is read, from `feature_links`, so a feature linked to the Job after the decision
//! was kept still shows it, and a merged feature takes its decisions with its links.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

/// One thing the owner settled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFeatureDecision {
    pub job: String,
    /// Names the act, so hearing it twice keeps one row.
    pub key: String,
    /// The target of the artifact he answered, empty where it was a question or a card.
    pub artifact: String,
    /// The question, or the artifact's title.
    pub question: String,
    /// The option's label, or `Approved`, `Asked for changes`, `Looks good`.
    pub choice: String,
    /// His words, empty where he gave none.
    pub note: String,
    pub at: String,
}

/// The kinds of link that name the features an artifact is about.
const ARTIFACT_KINDS: &str = "('sketch', 'page', 'walk', 'draft')";

impl Store {
    /// Keep a decision. **One already kept is left as it was**, so a repeat keeps its first date. Whether a row was added.
    pub fn keep_feature_decision(&mut self, decision: &KeptFeatureDecision) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO feature_decisions (job, key, artifact, question, choice, note, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![decision.job, decision.key, decision.artifact, decision.question, decision.choice, decision.note, decision.at],
            )
            .map(|added| added > 0)
            .map_err(failed("keeping a decision"))
    }

    /// The decisions that belong to the feature, newest first. **An artifact that names features files its decisions under
    /// those and no other**; any other decision belongs to every feature its Job is linked to.
    pub fn feature_decisions(&self, feature_id: &str) -> Result<Vec<KeptFeatureDecision>, WriteError> {
        let doing = "reading a feature's decisions";
        let named = format!("SELECT 1 FROM feature_links n WHERE n.kind IN {ARTIFACT_KINDS} AND n.target = d.artifact AND n.job = d.job");
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT d.job, d.key, d.artifact, d.question, d.choice, d.note, d.at FROM feature_decisions d
                 WHERE EXISTS ({named} AND n.feature_id = ?1)
                    OR (NOT EXISTS ({named}) AND EXISTS (SELECT 1 FROM feature_links j WHERE j.feature_id = ?1 AND j.kind = 'job' AND j.target = d.job))
                 ORDER BY d.at DESC, d.key"
            ))
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([feature_id], |row| {
                Ok(KeptFeatureDecision { job: row.get(0)?, key: row.get(1)?, artifact: row.get(2)?, question: row.get(3)?, choice: row.get(4)?, note: row.get(5)?, at: row.get(6)? })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
