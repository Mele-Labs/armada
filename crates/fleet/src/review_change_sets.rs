//! Change sets, the overview and a practice check: what each stands on and is told, and the
//! practice catalogue a check reads and keeps. The run itself is `review_runs`'s, like any other,
//! and its answer is read in `review_change_set_answers`. `docs/concepts/pull-review.md`, *Change sets*.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotPool, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ChangeSet, ChangeSetKind, HunkRef, ManifestId, PracticeCheck, PracticeItem, PracticeSource, ReviewResult,
    ReviewRunKind, COMMENT_AT_MOST,
};
use store::KeptPractices;

use crate::daemon::Fleet;
use crate::review_change_set_answers::{checked, found, hunks_of, overview_of, sets_of, ITEMS_AT_MOST, SETS_AT_MOST};
use crate::review_runs::{object_in, run_of, PLACE};

/// An overview or a practice check with no change sets kept at the head it reads. A 409.
const SETS_NEEDED: &str = "fleet.review_change_sets_needed";
/// A practice check of a set the kept change sets do not have. A 422.
const SET_UNKNOWN: &str = "fleet.review_change_set_unknown";
/// The review tools the overview is told, by id, in the order it is told them.
const OVERVIEW_TOOLS: [&str; 3] = ["risks", "bugs", "api-breaking-changes"];

/// What a change-set run stands on, found before the run is queued.
#[derive(Clone)]
pub(crate) enum SetsAsked {
    Sets,
    /// The kept sets, and the review tools' prompts it is told.
    Overview { sets: Vec<ChangeSet>, tools: String },
    /// The set, and what the catalogue keeps for its area and kind.
    Practices { set: ChangeSet, kept: Option<Catalogued> },
}

/// One catalogue entry, read back.
#[derive(Clone)]
pub(crate) struct Catalogued {
    items: Vec<PracticeItem>,
    sources: Vec<PracticeSource>,
    read_at: String,
}

/// What a run is told, and what its answer is held to.
pub(crate) struct Told {
    pub(crate) asking: String,
    pub(crate) contract: String,
    against: Against,
}

enum Against {
    Hunks(Vec<HunkRef>),
    Sets(Vec<String>),
    Read(ChangeSet),
    Kept(ChangeSet, Catalogued),
}

/// The name a change-set run is kept under, and its label. A practice check is named by its set.
pub(crate) fn named(kind: ReviewRunKind, name: &str) -> (String, String) {
    match kind {
        ReviewRunKind::Overview => ("overview".to_string(), "Overview".to_string()),
        ReviewRunKind::Practices => (name.trim().to_string(), format!("Practices of {}", name.trim())),
        _ => ("change-sets".to_string(), "Change sets".to_string()),
    }
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
    /// What a change-set run stands on: for an overview or a practice check, the sets kept at
    /// `head_column`, and for a practice check its set and the catalogue's entry. Refused before
    /// anything is spent.
    pub(crate) async fn sets_asked(
        &self,
        root: &str,
        manifest_id: &ManifestId,
        number: u64,
        head_column: &str,
        kind: ReviewRunKind,
        name: &str,
    ) -> Result<SetsAsked, Refusal> {
        if kind == ReviewRunKind::ChangeSets {
            return Ok(SetsAsked::Sets);
        }
        let store = self.store().lock().await;
        let rows = store
            .review_runs(manifest_id.as_str(), number)
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
        let kept = rows
            .iter()
            .filter(|row| row.kind == "change_sets" && row.state == "done" && row.head == head_column)
            .find_map(|row| match run_of(row).result {
                Some(ReviewResult::ChangeSets { sets }) => Some(sets),
                _ => None,
            });
        let Some(sets) = kept else {
            return Err(Refusal::IllegalMove(
                self.pr_refusal(SETS_NEEDED, "the change sets are run first, at the same head"),
            ));
        };
        if kind == ReviewRunKind::Overview {
            let resolved = self.review_tools_resolved(root);
            let tools = OVERVIEW_TOOLS
                .iter()
                .filter_map(|id| resolved.get(id))
                .map(|def| format!("### {}\n\n{}", def.name, def.prompt.trim()))
                .collect::<Vec<_>>()
                .join("\n\n");
            return Ok(SetsAsked::Overview { sets, tools });
        }
        let Some(set) = sets.into_iter().find(|set| set.id == name) else {
            return Err(Refusal::Unacceptable(
                self.pr_refusal(SET_UNKNOWN, format!("`{name}` is not one of the change sets kept at this head")),
            ));
        };
        let entry = store
            .practices(manifest_id.as_str(), &set.area, kind_word(set.kind))
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
        // An entry that will not read back is read again, as if none were kept.
        let kept = entry.and_then(|entry| {
            Some(Catalogued {
                items: ipc::decode("practice items", entry.items.as_bytes()).ok()?,
                sources: ipc::decode("practice sources", entry.sources.as_bytes()).ok()?,
                read_at: entry.read_at,
            })
        });
        Ok(SetsAsked::Practices { set, kept })
    }

    /// What the run is told, in its bay. A kept entry whose docs moved at the base is read again.
    pub(crate) async fn sets_told(&self, asked: &SetsAsked, pool: &SlotPool, slot: u32, patch: &str) -> Told {
        match asked {
            SetsAsked::Sets => sets_told(hunks_of(patch)),
            SetsAsked::Overview { sets, tools } => overview_told(sets, tools),
            SetsAsked::Practices { set, kept: Some(kept) } => {
                let paths: Vec<String> = kept.sources.iter().map(|source| source.path.clone()).collect();
                let then: Vec<Option<String>> = kept.sources.iter().map(|source| source.blob.clone()).collect();
                match self.base_blobs(pool, slot, paths).await == Some(then) {
                    true => kept_told(set, kept),
                    false => read_told(set),
                }
            }
            SetsAsked::Practices { set, kept: None } => read_told(set),
        }
    }

    /// The answer, held to what the run was told. A read that found practices is kept in the
    /// catalogue, with each doc's blob at the base, while the bay still holds the checkout.
    pub(crate) async fn sets_answered(
        &self,
        told: &Told,
        answer: &str,
        changed: &[String],
        checkout: &Path,
        bay: (&SlotPool, u32, &ManifestId),
    ) -> Result<ReviewResult, String> {
        let json = object_in(answer)?;
        let (set, kept) = match &told.against {
            Against::Hunks(hunks) => return Ok(ReviewResult::ChangeSets { sets: sets_of(json, hunks)? }),
            Against::Sets(sets) => return Ok(ReviewResult::Overview { findings: overview_of(json, sets, changed)? }),
            Against::Kept(set, kept) => {
                let checks = checked(json, &kept.items)?;
                return Ok(practices_result(set, true, &kept.read_at, kept.sources.clone(), checks));
            }
            Against::Read(set) => (set, found(json, checkout)?),
        };
        let (pool, slot, manifest_id) = bay;
        let paths: Vec<String> = kept.iter().map(|check| check.source.clone()).collect::<BTreeSet<_>>().into_iter().collect();
        let blobs = self.base_blobs(pool, slot, paths.clone()).await;
        let sources: Vec<PracticeSource> = paths
            .into_iter()
            .enumerate()
            .map(|(at, path)| PracticeSource { path, blob: blobs.as_ref().and_then(|blobs| blobs.get(at).cloned().flatten()) })
            .collect();
        let read_at = self.clock().now().as_str().to_string();
        // Nothing found is looked for again next time, and blobs that would not read could never
        // say the entry is still fresh.
        if !kept.is_empty() && blobs.is_some() {
            let items: Vec<PracticeItem> =
                kept.iter().map(|check| PracticeItem { item: check.item.clone(), source: check.source.clone() }).collect();
            let entry = KeptPractices {
                manifest_id: manifest_id.as_str().to_string(),
                area: set.area.clone(),
                kind: kind_word(set.kind).to_string(),
                items: ipc::encode(&items).map_err(|why| why.why)?,
                sources: ipc::encode(&sources).map_err(|why| why.why)?,
                read_at: read_at.clone(),
            };
            let _ = self.store().lock().await.keep_practices(&entry);
        }
        Ok(practices_result(set, false, &read_at, sources, kept))
    }

    /// Each path's blob at the bay's base, or none where the bay would not say.
    async fn base_blobs(&self, pool: &SlotPool, slot: u32, paths: Vec<String>) -> Option<Vec<Option<String>>> {
        let (vcs, pool) = (Arc::clone(self.vcs()), pool.clone());
        tokio::task::spawn_blocking(move || vcs.review_bay_base_blobs(&pool, slot, &paths)).await.ok()?.ok()
    }
}

/// The word a kind is kept under in the catalogue, as the wire spells it.
pub(crate) fn kind_word(kind: ChangeSetKind) -> &'static str {
    match kind {
        ChangeSetKind::Migration => "migration",
        ChangeSetKind::Data => "data",
        ChangeSetKind::Logic => "logic",
        ChangeSetKind::Api => "api",
        ChangeSetKind::Ui => "ui",
        ChangeSetKind::Tests => "tests",
        ChangeSetKind::Config => "config",
        ChangeSetKind::Other => "other",
    }
}

fn hunk_line(id: &str, hunk: &HunkRef) -> String {
    format!(
        "{id} `{}` @@ -{},{} +{},{} @@",
        hunk.path, hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
    )
}

fn set_lines(set: &ChangeSet) -> String {
    let hunks: Vec<String> = set.hunks.iter().map(|hunk| hunk_line("-", hunk)).collect();
    format!("{} ({}) {}: {}\n{}", set.id, kind_word(set.kind), set.title, set.purpose, hunks.join("\n"))
}

const KINDS: &str = "\"migration\" | \"data\" | \"logic\" | \"api\" | \"ui\" | \"tests\" | \"config\" | \"other\"";
const VERDICTS: &str = "\"met\" | \"missed\" | \"not_applicable\"";

fn sets_told(hunks: Vec<HunkRef>) -> Told {
    let listed: Vec<String> = hunks.iter().enumerate().map(|(at, hunk)| hunk_line(&format!("h{}", at + 1), hunk)).collect();
    Told {
        asking: format!(
            "Read the change as a few sets of change, each with one purpose, such as a database migration, the \
             data layer, the logic, an API, the interface, its tests or its configuration. Put every hunk below in \
             the set it serves, and order the sets, and the hunks in each, in the order they run: a migration \
             before the code that reads it, a function before its callers.\n\n## Hunks\n\n{}",
            listed.join("\n")
        ),
        contract: format!(
            "{{\"sets\": [{{\"title\": \"a few plain words\", \"kind\": {KINDS}, \"purpose\": \"one sentence on what \
             the set is for\", \"hunks\": [\"h1\", ...]}}]}}. At most {SETS_AT_MOST} sets. Name hunks by their ids \
             above, each in one set only."
        ),
        against: Against::Hunks(hunks),
    }
}

fn overview_told(sets: &[ChangeSet], tools: &str) -> Told {
    let listed: Vec<String> = sets.iter().map(set_lines).collect();
    Told {
        asking: format!(
            "Find what would block this change from merging, what puts it at risk, and what could be a bug, \
             reading it as the change sets below. A blocker must be fixed before it merges; a risk could go wrong \
             in a case the change does not handle; a bug is code that likely does the wrong thing. Report only \
             what you can place on lines of a changed file.\n\n## Change sets\n\n{}\n\n## What to look for\n\n{tools}",
            listed.join("\n\n")
        ),
        contract: format!(
            "{{\"findings\": [{{\"severity\": \"blocker\" | \"risk\" | \"bug\", \"set\": \"<a set's id above>\", \
             \"title\": \"a few plain words\", \"reason\": \"...\", \"at\": <place>}}]}}. An empty list is an answer. \
             {PLACE}. The reason is what the person posts to the author as it stands: one plain sentence, at most \
             {COMMENT_AT_MOST} characters, saying what goes wrong and what to do about it, with no jargon, no \
             review labels, nothing about yourself, and no code, list or line numbers."
        ),
        against: Against::Sets(sets.iter().map(|set| set.id.clone()).collect()),
    }
}

fn read_told(set: &ChangeSet) -> Told {
    let area = match set.area.as_str() {
        "" => "anywhere in the repository".to_string(),
        area => format!("under `{area}`"),
    };
    Told {
        asking: format!(
            "Find the repository's own docs that state practices for {kind} changes {area}: practice and \
             contributing docs, agent files, and docs next to the code. Read them into a checklist, one practice \
             a line, and check the change set below against each.\n\n## The change set\n\n{set}",
            kind = kind_word(set.kind),
            set = set_lines(set)
        ),
        contract: format!(
            "{{\"items\": [{{\"item\": \"one practice, in a short line\", \"source\": \"<the doc's path in the \
             checkout>\", \"verdict\": {VERDICTS}, \"note\": \"one short line on why\"}}]}}. At most {ITEMS_AT_MOST} \
             items, and only practices a doc states. An empty list is an answer: give it only if no doc states a \
             practice for this kind of change here."
        ),
        against: Against::Read(set.clone()),
    }
}

fn kept_told(set: &ChangeSet, kept: &Catalogued) -> Told {
    let items: Vec<String> = kept
        .items
        .iter()
        .enumerate()
        .map(|(at, item)| format!("{}. {} (from `{}`)", at + 1, item.item, item.source))
        .collect();
    Told {
        asking: format!(
            "Check the change set below against each of the repository's practices listed, which were read from \
             its docs before.\n\n## Practices\n\n{}\n\n## The change set\n\n{}",
            items.join("\n"),
            set_lines(set)
        ),
        contract: format!(
            "{{\"checks\": [{{\"item\": <the practice's number>, \"verdict\": {VERDICTS}, \"note\": \"one short line \
             on why\"}}]}}. Check every practice once."
        ),
        against: Against::Kept(set.clone(), kept.clone()),
    }
}

fn practices_result(
    set: &ChangeSet,
    reused: bool,
    read_at: &str,
    sources: Vec<PracticeSource>,
    checks: Vec<PracticeCheck>,
) -> ReviewResult {
    ReviewResult::Practices {
        set: set.id.clone(),
        kind: set.kind,
        area: set.area.clone(),
        read_at: read_at.to_string(),
        reused,
        sources,
        checks,
    }
}

#[cfg(test)]
pub(crate) fn change_sets_read(answer: &str, hunks: &[HunkRef]) -> Result<ReviewResult, String> {
    Ok(ReviewResult::ChangeSets { sets: sets_of(object_in(answer)?, hunks)? })
}

#[cfg(test)]
pub(crate) fn overview_read(answer: &str, sets: &[String], changed: &[String]) -> Result<ReviewResult, String> {
    Ok(ReviewResult::Overview { findings: overview_of(object_in(answer)?, sets, changed)? })
}

/// A first read when `items` is 0, and otherwise a check against that many kept practices.
#[cfg(test)]
pub(crate) fn practices_read(answer: &str, items: usize, checkout: &Path) -> Result<ReviewResult, String> {
    let set = ChangeSet {
        id: "set-1".into(),
        title: "T".into(),
        kind: ChangeSetKind::Other,
        purpose: "P.".into(),
        area: String::new(),
        hunks: Vec::new(),
    };
    let json = object_in(answer)?;
    let checks = match items {
        0 => found(json, checkout)?,
        n => {
            let kept: Vec<PracticeItem> =
                (1..=n).map(|at| PracticeItem { item: format!("practice {at}"), source: "docs/x.md".into() }).collect();
            checked(json, &kept)?
        }
    };
    Ok(practices_result(&set, items > 0, "", Vec::new(), checks))
}
