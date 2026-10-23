//! Saved issue queries and the query writer. `docs/concepts/tracker.md`, *Saved queries*.
//!
//! **A query is the tracker's own language and Fleet never reads one.** The writer is one model
//! call with no tools, given the tracker's guide and the person's words and never a ticket's title
//! or body. Whatever it writes is run with the tracker before it is answered, so a query the tracker
//! refuses fails here with the tracker's words and is never offered.
//!
//! **Nothing is kept until he saves**, and saving makes no model call: running a saved query later
//! is one tracker call.

use adapter_traits::{AgentHarness, Ask, Delivery, NotTracked, OpenIssue, QueryGuide, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    Instant, IssueList, IssueQueries, IssueQueryGuide, ManifestId, SaveIssueQuery, SavedIssueQuery,
    SearchIssues, WireError, WriteIssueQuery, WrittenIssueQuery,
};
use serde::Deserialize;
use store::KeptIssueQuery;

use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::review_runs::{asked_and_costed, object_in, said, QUESTION_AT_MOST};
use crate::tracking::UNREADABLE;

/// The words, a name or a query are blank, or the words are too long. A 422.
const BLANK: &str = "fleet.issue_query_blank";
/// The tracker would not run the query, in its own words. A 422.
const REFUSED: &str = "fleet.issue_query_refused";
/// The model's call failed or its answer could not be used. A 500: asking again is reasonable.
const NOT_WRITTEN: &str = "fleet.issue_query_not_written";
/// The repository has no saved query by that id. A 404.
const NO_SUCH_QUERY: &str = "fleet.no_such_issue_query";

const NAME_AT_MOST: usize = 120;

#[derive(Deserialize)]
struct QueryOut {
    query: String,
    #[serde(default)]
    row_fields: Vec<String>,
}

/// The row fields a person or a model named, each held to the names the tracker's guide lists.
/// In the order written and each once.
pub(crate) fn row_fields_held(guide: &QueryGuide, named: &[String]) -> Result<Vec<String>, String> {
    let mut kept: Vec<String> = Vec::new();
    for field in named.iter().map(|one| one.trim()) {
        if !guide.fields.iter().any(|known| known == field) {
            return Err(match guide.fields.is_empty() {
                true => format!("`{field}` is not a field this tracker fills; it fills none"),
                false => format!("`{field}` is not a field this tracker fills: {}", guide.fields.join(", ")),
            });
        }
        if !kept.iter().any(|had| had == field) {
            kept.push(field.to_string());
        }
    }
    Ok(kept)
}

/// The model's answer, held to the guide: a non-blank query and the fields a row may show.
pub(crate) fn query_read(answer: &str, guide: &QueryGuide) -> Result<(String, Vec<String>), String> {
    let out: QueryOut = said(object_in(answer)?)?;
    let query = out.query.trim();
    if query.is_empty() {
        return Err("it wrote a blank query".to_string());
    }
    Ok((query.to_string(), row_fields_held(guide, &out.row_fields)?))
}

/// What the model is told. **Everything but the person's words is the tracker's own teaching**, and
/// no issue's text is in it.
fn brief(guide: &QueryGuide, words: &str) -> String {
    let fields = match guide.fields.is_empty() {
        true => "(none)".to_string(),
        false => guide.fields.iter().map(|one| format!("- {one}")).collect::<Vec<_>>().join("\n"),
    };
    format!(
        "You write one query for an issue tracker, from what a person asked to see. You have no tools: \
         everything you may use is below.\n\n\
         ## The query language: {language}\n\n{teaching}\n\n\
         Two queries the tracker starts from:\n- Open: {open}\n- Assigned to me: {assigned}\n\n\
         ## Fields a row can show\n\n{fields}\n\n\
         ## What the person asked to see\n\n{words}\n\n\
         ## How to answer\n\n\
         Answer with one JSON object and nothing else: no prose before or after, no code fence. Its shape: \
         {{\"query\": \"one query in {language}\", \"row_fields\": [\"field names from the list above\"]}}. \
         Write the query as the language above takes it and use only what it says it can ask. \
         `row_fields` are the fields the person's words call for showing beside each issue, spelt exactly as \
         listed; an empty list is an answer.",
        language = guide.language,
        teaching = guide.guide.trim(),
        open = guide.open,
        assigned = guide.assigned_to_me,
    )
}

fn saved_of(kept: KeptIssueQuery) -> SavedIssueQuery {
    SavedIssueQuery { id: kept.id, name: kept.name, words: kept.words, query: kept.query, row_fields: kept.row_fields }
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
    fn query_refusal(&self, code: &'static str, said: impl Into<String>) -> WireError {
        WireError::raised(code, said, self.run_id())
    }

    /// `query` run with the tracker `served`'s Manifest chose. **The tracker's refusal is the answer**,
    /// in its own words.
    async fn query_run(&self, served: &Served, query: &str) -> Result<Vec<OpenIssue>, Refusal> {
        let (tracker, root, query) = (self.tracker_of(served), served.root().to_string(), query.to_string());
        let found = tokio::task::spawn_blocking(move || tracker.search_issues(&root, &query))
            .await
            .unwrap_or_else(|panicked| Err(NotTracked::of("searching the issues", panicked.to_string())));
        found.map_err(|not| {
            Refusal::Unacceptable(self.query_refusal(REFUSED, format!("{} did not happen: {}", not.doing, not.said)))
        })
    }

    pub(crate) async fn issues_searched(
        &self,
        manifest_id: ManifestId,
        search: SearchIssues,
    ) -> Result<IssueList, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let found = self.query_run(&served, &search.query).await?;
        let holders = self.issue_holders(&manifest_id).await?;
        Ok(IssueList {
            manifest_id,
            fetched_at: Instant::carried(self.now().as_str()),
            issues: found.iter().map(|one| crate::tracking::tracked(one, &holders)).collect(),
            stale: None,
        })
    }

    pub(crate) async fn issue_queries_read(&self, manifest_id: ManifestId) -> Result<IssueQueries, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let guide = self.tracker_of(&served).query_guide();
        let saved = self
            .store()
            .lock()
            .await
            .issue_queries(manifest_id.as_str())
            .map_err(|why| self.ledger_fault(why))?;
        let refused = guide.refusal().map(str::to_string);
        Ok(IssueQueries {
            manifest_id,
            guide: match refused {
                Some(_) => None,
                None => Some(IssueQueryGuide {
                    language: guide.language,
                    open: guide.open,
                    assigned_to_me: guide.assigned_to_me,
                    fields: guide.fields,
                }),
            },
            refused,
            saved: saved.into_iter().map(saved_of).collect(),
        })
    }

    /// **One call to the model and one to the tracker.** A tracker that cannot be read is refused
    /// before the model is asked, since there is nothing to run what it writes against.
    pub(crate) async fn issue_query_written(
        &self,
        manifest_id: ManifestId,
        write: WriteIssueQuery,
    ) -> Result<WrittenIssueQuery, Refusal> {
        let blank = |said: String| Refusal::Unacceptable(self.query_refusal(BLANK, said));
        let words = write.words.split_whitespace().collect::<Vec<_>>().join(" ");
        if words.is_empty() {
            return Err(blank("an issue query needs words".to_string()));
        }
        if words.chars().count() > QUESTION_AT_MOST {
            return Err(blank(format!("words for an issue query are at most {QUESTION_AT_MOST} characters")));
        }
        let served = self.served_named(Some(&manifest_id))?;
        let guide = self.tracker_of(&served).query_guide();
        if let Some(why) = guide.refusal() {
            return Err(Refusal::Fault(self.query_refusal(UNREADABLE, format!("the tracker cannot be queried: {why}"))));
        }
        let not_written = |said: String| Refusal::Fault(self.query_refusal(NOT_WRITTEN, said));
        let reviewing = self.reviewing().map_err(|why| not_written(format!("no agent could be asked: {why:?}")))?;
        let prompt = brief(&guide, &words);
        // No `.reading(...)`: a call that reaches nothing.
        let ask = Ask::put(reviewing.model.clone(), &prompt, reviewing.environment.clone())
            .map_err(|why| not_written(format!("the question could not be put: {why:?}")))?;
        let (answered, cost_micros) = asked_and_costed(&reviewing, &ask).await;
        let answer = answered.map_err(|why| not_written(format!("the agent did not answer: {why}")))?;
        let (query, row_fields) =
            query_read(&answer, &guide).map_err(|why| not_written(format!("the answer was refused: {why}")))?;
        let found = self.query_run(&served, &query).await?;
        let holders = self.issue_holders(&manifest_id).await?;
        Ok(WrittenIssueQuery {
            query,
            row_fields,
            issues: found.iter().map(|one| crate::tracking::tracked(one, &holders)).collect(),
            cost_micros,
        })
    }

    pub(crate) async fn issue_query_saved(
        &self,
        manifest_id: ManifestId,
        save: SaveIssueQuery,
    ) -> Result<SavedIssueQuery, Refusal> {
        let blank = |said: &str| Refusal::Unacceptable(self.query_refusal(BLANK, said));
        let (name, query) = (save.name.trim(), save.query.trim());
        if name.is_empty() || name.chars().count() > NAME_AT_MOST {
            return Err(blank(&format!("a saved query needs a name of at most {NAME_AT_MOST} characters")));
        }
        if query.is_empty() {
            return Err(blank("a saved query needs a query"));
        }
        let served = self.served_named(Some(&manifest_id))?;
        let guide = self.tracker_of(&served).query_guide();
        let row_fields = row_fields_held(&guide, &save.row_fields)
            .map_err(|why| Refusal::Unacceptable(self.query_refusal(REFUSED, why)))?;
        self.query_run(&served, query).await?;
        let mut store = self.store().lock().await;
        let id = match save.id {
            Some(id) => {
                let had = store.issue_query(manifest_id.as_str(), &id).map_err(|why| self.ledger_fault(why))?;
                if had.is_none() {
                    return Err(Refusal::NotFound(
                        self.query_refusal(NO_SUCH_QUERY, format!("the repository has no saved query {id}")),
                    ));
                }
                id
            }
            None => self.mint().ulid().as_str().to_lowercase(),
        };
        let kept = KeptIssueQuery {
            id,
            manifest_id: manifest_id.as_str().to_string(),
            name: name.to_string(),
            words: save.words.trim().to_string(),
            query: query.to_string(),
            row_fields,
        };
        store.keep_issue_query(&kept, self.now().as_str()).map_err(|why| self.ledger_fault(why))?;
        Ok(saved_of(kept))
    }

    pub(crate) async fn issue_query_forgotten(
        &self,
        manifest_id: ManifestId,
        id: String,
    ) -> Result<IssueQueries, Refusal> {
        self.served_named(Some(&manifest_id))?;
        let gone = self
            .store()
            .lock()
            .await
            .forget_issue_query(manifest_id.as_str(), &id)
            .map_err(|why| self.ledger_fault(why))?;
        if !gone {
            return Err(Refusal::NotFound(
                self.query_refusal(NO_SUCH_QUERY, format!("the repository has no saved query {id}")),
            ));
        }
        self.issue_queries_read(manifest_id).await
    }
}
