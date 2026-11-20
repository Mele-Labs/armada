//! A project's feature library, kept on this Mac. `docs/concepts/feature-catalog.md`.
//!
//! **Rows only.** Which files a feature owns, what is stale and what a page says are Fleet's to work out from
//! these; the store keeps what was written and what was seen. Ids are minted by the caller and never a rowid, so
//! the rows can move to a shared store unchanged.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What a feature is known by.
pub mod feature_state {
    pub const SEEDED: &str = "seeded";
    pub const CONFIRMED: &str = "confirmed";
    /// Named by a Job's trailer or act and not in the catalog yet: the Operator brings it to the owner on a card.
    pub const PENDING: &str = "pending";
}

/// The sections a Job writes, as the table names them.
pub mod section {
    pub const TEST: &str = "test";
    pub const RUNS: &str = "runs";
    pub const OBSERVE: &str = "observe";
    pub const DEBT: &str = "debt";
    /// Every section, in the order a page shows them.
    pub const ALL: [&str; 4] = [TEST, RUNS, OBSERVE, DEBT];
}

/// What a link ties a feature to, as the table names it.
pub mod link_kind {
    pub const JOB: &str = "job";
    pub const PULL_REQUEST: &str = "pull_request";
    pub const MERGE: &str = "merge";
    pub const SKETCH: &str = "sketch";
    pub const PAGE: &str = "page";
    pub const WALK: &str = "walk";
    pub const DRAFT: &str = "draft";
    pub const COMMIT: &str = "commit";
    pub const BUG: &str = "bug";
    /// A document or ticket a Job read before it changed the feature's files.
    pub const DOC: &str = "doc";
    pub const TICKET: &str = "ticket";
    /// A Job used a word the project's glossary says not to: the word is the target, the term to use is the title.
    pub const GLOSSARY_FLAG: &str = "glossary_flag";
    pub const ALL: [&str; 12] = [JOB, PULL_REQUEST, MERGE, SKETCH, PAGE, WALK, DRAFT, COMMIT, BUG, DOC, TICKET, GLOSSARY_FLAG];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptArea {
    pub project: String,
    pub slug: String,
    pub title: String,
    pub position: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFeature {
    pub id: String,
    pub project: String,
    /// The area's slug.
    pub area: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub state: String,
    /// The repository the prefixes are in, empty for any member of the project.
    pub manifest_id: String,
    /// `path` or `path:line`.
    pub entry_points: Vec<String>,
    /// The path prefixes it owns, as written.
    pub paths: Vec<String>,
    pub position: u32,
    /// When the feature first entered the catalog. **Set at the first insert and kept by every upsert after it.**
    pub added_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSection {
    pub feature_id: String,
    pub section: String,
    pub text: String,
    /// The Job that wrote it.
    pub job: String,
    pub written_at: String,
    /// The commit the Job's checkout stood at, empty where it could not be read.
    pub at_commit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFeatureLink {
    pub feature_id: String,
    pub kind: String,
    pub target: String,
    /// The Job it came through, empty where none.
    pub job: String,
    pub title: String,
    pub state: String,
    pub via: String,
    pub at: String,
    /// What a doc or ticket says, as its first sentence or as the Job that read it wrote it. Empty for every other kind.
    pub gist: String,
    /// [`gist_by`] says who wrote `gist`: empty for the result's first sentence.
    pub gist_by: String,
}

/// Who wrote a link's gist, as `feature_links.gist_by` holds it.
pub mod gist_by {
    /// The Job that read the doc wrote it at its end.
    pub const JOB: &str = "job";
}

/// A file a Job wrote while its work was filed under a feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFeatureFile {
    pub feature_id: String,
    pub job: String,
    pub path: String,
    pub first_at: String,
    pub last_at: String,
}

/// What one merged pull request changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFeatureMerge {
    pub manifest_id: String,
    pub number: u64,
    pub merged_at: String,
    pub at_commit: String,
    /// The Job that held the pull request, empty where none did.
    pub job: String,
    pub files: Vec<String>,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn strings(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn json(lines: &[String]) -> String {
    serde_json::to_string(lines).unwrap_or_else(|_| String::from("[]"))
}

const FEATURE: &str = "id, project, area, slug, title, summary, state, manifest_id, entry_points, position, added_at";
const SECTION: &str = "feature_id, section, text, job, written_at, at_commit";
const LINK: &str = "feature_id, kind, target, job, title, state, via, at, gist, gist_by";

fn feature_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptFeature> {
    Ok(KeptFeature {
        id: row.get(0)?,
        project: row.get(1)?,
        area: row.get(2)?,
        slug: row.get(3)?,
        title: row.get(4)?,
        summary: row.get(5)?,
        state: row.get(6)?,
        manifest_id: row.get(7)?,
        entry_points: strings(row.get(8)?),
        paths: Vec::new(),
        position: row.get(9)?,
        added_at: row.get(10)?,
    })
}

fn link_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptFeatureLink> {
    Ok(KeptFeatureLink {
        feature_id: row.get(0)?,
        kind: row.get(1)?,
        target: row.get(2)?,
        job: row.get(3)?,
        title: row.get(4)?,
        state: row.get(5)?,
        via: row.get(6)?,
        at: row.get(7)?,
        gist: row.get(8)?,
        gist_by: row.get(9)?,
    })
}

impl Store {
    /// Keep an area, replacing its title and place where the project already has one by that slug.
    pub fn put_feature_area(&mut self, area: &KeptArea) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO feature_areas (project, slug, title, position) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (project, slug) DO UPDATE SET title = excluded.title, position = excluded.position",
                params![area.project, area.slug, area.title, area.position],
            )
            .map(drop)
            .map_err(failed("keeping a feature area"))
    }

    /// The project's areas, in their order.
    pub fn feature_areas(&self, project: &str) -> Result<Vec<KeptArea>, WriteError> {
        let doing = "reading a project's feature areas";
        let mut statement = self
            .conn
            .prepare("SELECT project, slug, title, position FROM feature_areas WHERE project = ?1 ORDER BY position, slug")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([project], |row| Ok(KeptArea { project: row.get(0)?, slug: row.get(1)?, title: row.get(2)?, position: row.get(3)? }))
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep a feature by its project and slug. **A feature the project already has by that slug keeps its id**
    /// and takes the rest, its prefixes replaced whole. Answers the id the row has.
    pub fn put_feature(&mut self, feature: &KeptFeature) -> Result<String, WriteError> {
        let doing = "keeping a feature";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        let held: Option<String> = tx
            .query_row("SELECT id FROM features WHERE project = ?1 AND slug = ?2", params![feature.project, feature.slug], |row| row.get(0))
            .optional()
            .map_err(failed(doing))?;
        let id = held.unwrap_or_else(|| feature.id.clone());
        tx.execute(
            &format!(
                "INSERT INTO features ({FEATURE}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT (id) DO UPDATE SET area = excluded.area, title = excluded.title, summary = excluded.summary,
                 state = excluded.state, manifest_id = excluded.manifest_id, entry_points = excluded.entry_points,
                 position = excluded.position"
            ),
            params![
                id,
                feature.project,
                feature.area,
                feature.slug,
                feature.title,
                feature.summary,
                feature.state,
                feature.manifest_id,
                json(&feature.entry_points),
                feature.position,
                feature.added_at,
            ],
        )
        .map_err(failed(doing))?;
        tx.execute("DELETE FROM feature_aliases WHERE project = ?1 AND alias = ?2", params![feature.project, feature.slug]).map_err(failed(doing))?;
        tx.execute("DELETE FROM feature_paths WHERE feature_id = ?1", [&id]).map_err(failed(doing))?;
        for prefix in &feature.paths {
            tx.execute("INSERT OR IGNORE INTO feature_paths (feature_id, prefix) VALUES (?1, ?2)", params![id, prefix]).map_err(failed(doing))?;
        }
        tx.commit().map_err(failed(doing))?;
        Ok(id)
    }

    /// Every feature of the project, area by area in their order, each with its prefixes.
    pub fn features_in(&self, project: &str) -> Result<Vec<KeptFeature>, WriteError> {
        let doing = "reading a project's features";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {FEATURE} FROM features f WHERE project = ?1
                 ORDER BY COALESCE((SELECT position FROM feature_areas a WHERE a.project = f.project AND a.slug = f.area), 0), area, position, slug"
            ))
            .map_err(failed(doing))?;
        let mut features: Vec<KeptFeature> = statement
            .query_map([project], feature_row)
            .map_err(failed(doing))?
            .collect::<Result<_, _>>()
            .map_err(failed(doing))?;
        let mut paths = self
            .conn
            .prepare("SELECT p.feature_id, p.prefix FROM feature_paths p JOIN features f ON f.id = p.feature_id WHERE f.project = ?1 ORDER BY p.prefix")
            .map_err(failed(doing))?;
        let owned = paths
            .query_map([project], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(failed(doing))?;
        for one in owned {
            let (id, prefix) = one.map_err(failed(doing))?;
            if let Some(feature) = features.iter_mut().find(|feature| feature.id == id) {
                feature.paths.push(prefix);
            }
        }
        Ok(features)
    }

    /// Every feature of every project that is pending, oldest first, without its prefixes.
    pub fn pending_features(&self) -> Result<Vec<KeptFeature>, WriteError> {
        let doing = "reading the pending features";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {FEATURE} FROM features WHERE state = ?1 ORDER BY added_at, slug"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([feature_state::PENDING], feature_row).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// How many features each area of each project holds, pending ones left out, the biggest first.
    pub fn feature_counts(&self) -> Result<Vec<(String, String, u32)>, WriteError> {
        let doing = "counting the features of each area";
        let mut statement = self
            .conn
            .prepare("SELECT project, area, COUNT(*) AS n FROM features WHERE state != ?1 GROUP BY project, area ORDER BY n DESC, project, area")
            .map_err(failed(doing))?;
        let rows = statement.query_map([feature_state::PENDING], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Put the feature `slug` of `project` in the area `area`. **The caller checks the area is kept.**
    pub fn set_feature_area(&mut self, project: &str, slug: &str, area: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE features SET area = ?3 WHERE project = ?1 AND slug = ?2", params![project, slug, area])
            .map(drop)
            .map_err(failed("moving a feature to an area"))
    }

    /// The slugs `project`'s features were known by, each with the slug it is now.
    pub fn feature_aliases(&self, project: &str) -> Result<Vec<(String, String)>, WriteError> {
        let doing = "reading a project's feature aliases";
        let mut statement = self.conn.prepare("SELECT alias, slug FROM feature_aliases WHERE project = ?1 ORDER BY alias").map_err(failed(doing))?;
        let rows = statement.query_map([project], |row| Ok((row.get(0)?, row.get(1)?))).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The feature `slug` names in `project`.
    pub fn feature_named(&self, project: &str, slug: &str) -> Result<Option<KeptFeature>, WriteError> {
        Ok(self.features_in(project)?.into_iter().find(|feature| feature.slug == slug))
    }

    /// The feature whose id is `id`, in whichever project.
    pub fn feature_by_id(&self, id: &str) -> Result<Option<KeptFeature>, WriteError> {
        let project: Option<String> = self
            .conn
            .query_row("SELECT project FROM features WHERE id = ?1", [id], |row| row.get(0))
            .optional()
            .map_err(failed("reading a feature"))?;
        match project {
            Some(project) => Ok(self.features_in(&project)?.into_iter().find(|feature| feature.id == id)),
            None => Ok(None),
        }
    }

    /// Keep a section, replacing the one the feature has of that name.
    pub fn put_feature_section(&mut self, kept: &KeptSection) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO feature_sections ({SECTION}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT (feature_id, section) DO UPDATE SET text = excluded.text, job = excluded.job,
                     written_at = excluded.written_at, at_commit = excluded.at_commit"
                ),
                params![kept.feature_id, kept.section, kept.text, kept.job, kept.written_at, kept.at_commit],
            )
            .map(drop)
            .map_err(failed("keeping a feature's section"))
    }

    /// The sections a feature has, in the order a page shows them.
    pub fn feature_sections(&self, feature_id: &str) -> Result<Vec<KeptSection>, WriteError> {
        let doing = "reading a feature's sections";
        let mut statement = self.conn.prepare(&format!("SELECT {SECTION} FROM feature_sections WHERE feature_id = ?1")).map_err(failed(doing))?;
        let rows = statement
            .query_map([feature_id], |row| {
                Ok(KeptSection {
                    feature_id: row.get(0)?,
                    section: row.get(1)?,
                    text: row.get(2)?,
                    job: row.get(3)?,
                    written_at: row.get(4)?,
                    at_commit: row.get(5)?,
                })
            })
            .map_err(failed(doing))?;
        let mut kept: Vec<KeptSection> = rows.collect::<Result<_, _>>().map_err(failed(doing))?;
        kept.sort_by_key(|one| section::ALL.iter().position(|name| *name == one.section).unwrap_or(usize::MAX));
        Ok(kept)
    }

    /// How many sections of `project`'s features were written after `since`, an RFC3339 instant. Every one where it is empty.
    pub fn sections_written_since(&self, project: &str, since: &str) -> Result<u64, WriteError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM feature_sections s JOIN features f ON f.id = s.feature_id WHERE f.project = ?1 AND s.written_at > ?2",
                [project, since],
                |row| row.get(0),
            )
            .map_err(failed("counting the sections written in a project"))
    }

    /// Keep a link. **A link already held is left as it was**, so its first sighting stays its date; a state
    /// that moved (a bug closed) is [`Store::move_feature_link`]'s. Whether a row was added.
    pub fn put_feature_link(&mut self, link: &KeptFeatureLink) -> Result<bool, WriteError> {
        self.conn
            .execute(
                &format!("INSERT OR IGNORE INTO feature_links ({LINK}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"),
                params![link.feature_id, link.kind, link.target, link.job, link.title, link.state, link.via, link.at, link.gist, link.gist_by],
            )
            .map(|added| added > 0)
            .map_err(failed("keeping a feature's link"))
    }

    /// Every link of the feature, oldest first.
    pub fn feature_links(&self, feature_id: &str) -> Result<Vec<KeptFeatureLink>, WriteError> {
        let doing = "reading a feature's links";
        let mut statement = self.conn.prepare(&format!("SELECT {LINK} FROM feature_links WHERE feature_id = ?1 ORDER BY at, kind, target")).map_err(failed(doing))?;
        let rows = statement.query_map([feature_id], link_row).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The links of one kind that came through `job`, across features.
    pub fn feature_links_of_job(&self, job: &str, kind: &str) -> Result<Vec<KeptFeatureLink>, WriteError> {
        let doing = "reading what a Job is linked to";
        let mut statement = self.conn.prepare(&format!("SELECT {LINK} FROM feature_links WHERE job = ?1 AND kind = ?2 ORDER BY at")).map_err(failed(doing))?;
        let rows = statement.query_map(params![job, kind], link_row).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The `kind` links of every feature in `project`, newest first.
    pub fn project_links(&self, project: &str, kind: &str) -> Result<Vec<KeptFeatureLink>, WriteError> {
        let doing = "reading a project's links";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {} FROM feature_links l JOIN features f ON f.id = l.feature_id WHERE f.project = ?1 AND l.kind = ?2 ORDER BY l.at DESC",
                LINK.split(", ").map(|name| format!("l.{name}")).collect::<Vec<_>>().join(", ")
            ))
            .map_err(failed(doing))?;
        let rows = statement.query_map(params![project, kind], link_row).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep that `job` wrote `path` under the feature, the first time and the latest.
    pub fn keep_feature_write(&mut self, feature_id: &str, job: &str, path: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO feature_job_files (feature_id, job, path, first_at, last_at) VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT (feature_id, job, path) DO UPDATE SET last_at = excluded.last_at",
                params![feature_id, job, path, at],
            )
            .map(drop)
            .map_err(failed("keeping a file a Job wrote"))
    }

    /// The files `job` wrote under any feature.
    pub fn feature_files_of_job(&self, job: &str) -> Result<Vec<KeptFeatureFile>, WriteError> {
        let doing = "reading the files a Job wrote";
        let mut statement = self
            .conn
            .prepare("SELECT feature_id, job, path, first_at, last_at FROM feature_job_files WHERE job = ?1 ORDER BY first_at, path")
            .map_err(failed(doing))?;
        let rows = statement.query_map([job], file_row).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Each feature of the project with each Job that filed a file under it, once each.
    pub fn feature_job_pairs(&self, project: &str) -> Result<Vec<(String, String)>, WriteError> {
        let doing = "reading which Jobs worked on a project's features";
        let mut statement = self
            .conn
            .prepare(
                "SELECT DISTINCT j.feature_id, j.job FROM feature_job_files j JOIN features f ON f.id = j.feature_id
                 WHERE f.project = ?1 ORDER BY j.feature_id, j.job",
            )
            .map_err(failed(doing))?;
        let rows = statement.query_map([project], |row| Ok((row.get(0)?, row.get(1)?))).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Whether a merged pull request's files are kept already.
    pub fn feature_merge_kept(&self, manifest: &str, number: u64) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT 1 FROM feature_merges WHERE manifest_id = ?1 AND number = ?2", params![manifest, number], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(failed("reading whether a merge is kept"))
    }

    /// Keep that `feature_id`'s page was handed to `holder` of `session`: the Job's own thread where `holder` is empty,
    /// else a subagent's id. **Whether this is the first time**, so two calls at once hand it over once.
    pub fn keep_feature_handoff(&mut self, session: &str, holder: &str, feature_id: &str, via: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO feature_handoffs (session_id, holder, feature_id, via, handed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session, holder, feature_id, via, at],
            )
            .map(|added| added > 0)
            .map_err(failed("keeping that a feature's page was handed over"))
    }

    /// The ids of the features whose page was handed to `holder` of `session`.
    pub fn feature_handoffs(&self, session: &str, holder: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading which feature pages a Job was handed";
        let mut statement = self
            .conn
            .prepare("SELECT feature_id FROM feature_handoffs WHERE session_id = ?1 AND holder = ?2 ORDER BY handed_at, feature_id")
            .map_err(failed(doing))?;
        let rows = statement.query_map(params![session, holder], |row| row.get(0)).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Whether any feature is kept for the project.
    pub fn project_has_features(&self, project: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT 1 FROM features WHERE project = ?1 LIMIT 1", [project], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(failed("reading whether a project has features"))
    }

    /// Every file a Job filed under a feature of the project, once each.
    pub fn feature_filed_paths(&self, project: &str) -> Result<Vec<(String, String)>, WriteError> {
        let doing = "reading the files filed under a project's features";
        let mut statement = self
            .conn
            .prepare(
                "SELECT DISTINCT j.feature_id, j.path FROM feature_job_files j JOIN features f ON f.id = j.feature_id
                 WHERE f.project = ?1 ORDER BY j.feature_id, j.path",
            )
            .map_err(failed(doing))?;
        let rows = statement.query_map([project], |row| Ok((row.get(0)?, row.get(1)?))).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The files the most Jobs wrote under the feature, most first, with how many Jobs.
    pub fn feature_top_files(&self, feature_id: &str, limit: u32) -> Result<Vec<(String, u32)>, WriteError> {
        let doing = "reading a feature's most written files";
        let mut statement = self
            .conn
            .prepare(
                "SELECT path, COUNT(DISTINCT job) AS n FROM feature_job_files WHERE feature_id = ?1 GROUP BY path ORDER BY n DESC, path LIMIT ?2",
            )
            .map_err(failed(doing))?;
        let rows = statement.query_map(params![feature_id, limit], |row| Ok((row.get(0)?, row.get(1)?))).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep what a merged pull request changed. **The first sighting stands.** Whether a row was added.
    pub fn keep_feature_merge(&mut self, merge: &KeptFeatureMerge) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO feature_merges (manifest_id, number, merged_at, at_commit, job, files) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![merge.manifest_id, merge.number, merge.merged_at, merge.at_commit, merge.job, json(&merge.files)],
            )
            .map(|added| added > 0)
            .map_err(failed("keeping what a merged pull request changed"))
    }

    /// The merges of a repository after `after` (RFC3339, exclusive), oldest first. An empty `manifest` is every repository.
    pub fn feature_merges_after(&self, manifest: &str, after: &str) -> Result<Vec<KeptFeatureMerge>, WriteError> {
        let doing = "reading the merges after a section";
        let mut statement = self
            .conn
            .prepare(
                "SELECT manifest_id, number, merged_at, at_commit, job, files FROM feature_merges
                 WHERE (?1 = '' OR manifest_id = ?1) AND merged_at > ?2 ORDER BY merged_at, number",
            )
            .map_err(failed(doing))?;
        let rows = statement
            .query_map(params![manifest, after], |row| {
                Ok(KeptFeatureMerge {
                    manifest_id: row.get(0)?,
                    number: row.get(1)?,
                    merged_at: row.get(2)?,
                    at_commit: row.get(3)?,
                    job: row.get(4)?,
                    files: strings(row.get(5)?),
                })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The newest merge Fleet kept for a repository, which is what a page is current as of. An empty `manifest` is every
    /// repository.
    pub fn feature_merge_newest(&self, manifest: &str) -> Result<Option<(u64, String)>, WriteError> {
        self.conn
            .query_row(
                "SELECT number, merged_at FROM feature_merges WHERE (?1 = '' OR manifest_id = ?1) ORDER BY merged_at DESC, number DESC LIMIT 1",
                [manifest],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(failed("reading the newest merge kept"))
    }
}

fn file_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptFeatureFile> {
    Ok(KeptFeatureFile {
        feature_id: row.get(0)?,
        job: row.get(1)?,
        path: row.get(2)?,
        first_at: row.get(3)?,
        last_at: row.get(4)?,
    })
}
