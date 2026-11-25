//! A project's library read into the shapes the wire carries: the list with who is on each feature, one page whole, and
//! the page condensed.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    FeatureAreaRow, FeatureBrief, FeatureDecision, FeatureFile, FeatureJob, FeatureLibrary, FeatureLink, FeatureLinkKind, FeaturePage, FeatureRow, FeatureSection, FeatureRead,
    FeatureSectionKind, FeatureState, Instant, SessionId,
};
use store::{feature_link_kind, upkeep_kind, KeptFeatureDecision, KeptFeature, KeptFeatureLink, KeptSection, KeptUpkeep, Store, WriteError};

use super::health::Health;
use super::brief::{condensed, first_line, tokens_of, BriefFacts};
use super::mapping::FeatureMap;
use super::staleness::{owned, stale_since, OwnedMerge};
use super::whos_on_what::elsewhere_of;
use crate::daemon::Fleet;
use crate::my_pulls::Mine;

pub(super) const UNKNOWN: &str = "fleet.feature_unknown";
const SECTION_UNKNOWN: &str = "fleet.feature_section_unknown";

/// A project's features, areas and the map that says which own a file.
pub(super) struct Loaded {
    pub(super) project: String,
    pub(super) areas: Vec<store::KeptArea>,
    pub(super) map: FeatureMap,
    /// The slugs features were known by, each with the slug it is now.
    pub(super) aliases: BTreeMap<String, String>,
}

impl Loaded {
    pub(super) fn read(store: &Store, project: &str) -> Result<Loaded, WriteError> {
        let features = store.features_in(project)?;
        let filed = store.feature_filed_paths(project)?;
        let aliases = store.feature_aliases(project)?.into_iter().collect();
        Ok(Loaded { project: project.to_string(), areas: store.feature_areas(project)?, map: FeatureMap::new(features, filed), aliases })
    }

    pub(super) fn feature(&self, slug: &str) -> Option<&KeptFeature> {
        self.map.features().iter().find(|feature| feature.slug == slug)
    }

    /// The feature `slug` names, or the one it was known as before a rename or merge. **What a Job or the Operator says goes
    /// through here**, and a keeper of the library's own rows through [`Loaded::feature`].
    pub(super) fn feature_named(&self, slug: &str) -> Option<&KeptFeature> {
        self.feature(slug).or_else(|| self.aliases.get(slug).and_then(|now| self.feature(now)))
    }
}

/// The slug `slug` names in `project`, as the library keeps it: `None` for one the project does not hold.
pub(crate) fn slug_held(store: &Store, project: &str, slug: &str) -> Result<Option<String>, WriteError> {
    Ok(Loaded::read(store, project)?.feature_named(slug.trim()).map(|feature| feature.slug.clone()))
}

/// A name with its case and separators taken off, so `JobCard` and `job_card` read alike.
fn plain(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// The stem of the last segment of a path or entry point, plain.
fn stem(path: &str) -> String {
    let path = path.split(':').next().unwrap_or(path).trim_end_matches('/');
    let last = path.rsplit('/').next().unwrap_or(path);
    plain(last.split('.').next().unwrap_or(last))
}

/// The slugs of the features a note on a screen points to in `project`: those that own a file the note's element was drawn from,
/// and where it names none, those with an entry point or a prefix whose file is named for the component. **Read off the paths the
/// library keeps, never guessed.** Empty where the note names neither.
pub(crate) fn features_of_note(store: &Store, project: &str, manifest: &str, files: &[String], component: Option<&str>) -> Result<Vec<String>, WriteError> {
    let loaded = Loaded::read(store, project)?;
    let mut found: Vec<String> = Vec::new();
    for file in files {
        for owner in loaded.map.owners(Some(manifest), file) {
            if !found.contains(&owner.feature.slug) {
                found.push(owner.feature.slug.clone());
            }
        }
    }
    let named = component.map(plain).filter(|name| !name.is_empty());
    if let (true, Some(name)) = (found.is_empty(), named) {
        for feature in loaded.map.features() {
            let reads = feature.entry_points.iter().chain(feature.paths.iter()).any(|path| stem(path) == name);
            if reads && !found.contains(&feature.slug) {
                found.push(feature.slug.clone());
            }
        }
    }
    Ok(found)
}

fn state_of(kept: &str) -> FeatureState {
    match kept {
        store::feature_state::CONFIRMED => FeatureState::Confirmed,
        store::feature_state::PENDING => FeatureState::Pending,
        _ => FeatureState::Seeded,
    }
}

fn instant(at: &str) -> Instant {
    Instant::carried(at.to_string())
}

/// The date of an RFC3339 instant.
fn date(at: &str) -> String {
    at.chars().take(10).collect()
}

/// Each written section of the feature with the merge it is stale since, if any. One read of the merges serves them all.
pub(super) fn sections_marked(store: &Store, loaded: &Loaded, feature: &KeptFeature) -> Result<Vec<(KeptSection, Option<u64>)>, WriteError> {
    let sections = store.feature_sections(&feature.id)?;
    let Some(oldest) = sections.iter().map(|one| one.written_at.as_str()).min() else {
        return Ok(Vec::new());
    };
    let merges = owned(&loaded.map, store.feature_merges_after(&feature.manifest_id, oldest)?);
    Ok(marked(feature, sections, &merges))
}

/// The sections of `feature`, each with the merge it is stale since. `merges` may hold every repository's: a feature that
/// names its repository is read against that one's alone.
pub(super) fn marked(feature: &KeptFeature, sections: Vec<KeptSection>, merges: &[OwnedMerge]) -> Vec<(KeptSection, Option<u64>)> {
    let here = |one: &&OwnedMerge| feature.manifest_id.is_empty() || one.merge.manifest_id == feature.manifest_id;
    sections
        .into_iter()
        .map(|section| {
            let since = stale_since(&feature.id, &section, merges.iter().filter(here));
            (section, since)
        })
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
    /// The project ids a call is about: the one named, else the project the calling Job was started for, else those that
    /// hold its repository.
    pub(super) fn projects_asked(&self, store: &Store, caller: Option<api::Caller>, named: Option<&str>) -> Result<Vec<String>, Refusal> {
        if let Some(named) = named.map(str::trim).filter(|named| !named.is_empty()) {
            return Ok(vec![named.to_string()]);
        }
        let job = caller
            .and_then(|caller| self.session_holding(&caller))
            .ok_or_else(|| self.hosted_refusal("fleet.feature_project", "name a project, or call from a job that works in one"))?;
        let projects = self.projects_of_job_in(store, &job);
        match projects.is_empty() {
            true => Err(self.hosted_refusal("fleet.feature_project", "this job works in no project")),
            false => Ok(projects),
        }
    }

    /// The projects a Job's work is filed in: the one it was started for, else those that hold its repository.
    pub(crate) fn projects_of_job_in(&self, store: &Store, job: &str) -> Vec<String> {
        let hosting = store.hosting(job).ok().flatten();
        if let Some(project) = hosting.as_ref().and_then(|hosting| hosting.project.clone()) {
            return vec![project];
        }
        let manifest = hosting
            .map(|hosting| hosting.manifest_id)
            .or_else(|| store.session(job).ok().flatten().and_then(|session| session.manifest_id))
            .filter(|manifest| !manifest.is_empty());
        let Some(manifest) = manifest else {
            return Vec::new();
        };
        let held = self.projects_held();
        held.holding(&ipc::ManifestId::carried(&manifest).to_domain()).into_iter().map(|one| one.id().as_str().to_string()).collect()
    }

    /// The repositories the project's members name, which a feature that names none is read in. **A copy of another
    /// Mac's names its own**, read in this Mac's catalog.
    fn repositories_of(&self, store: &Store, project: &str) -> Vec<String> {
        let held = self.projects_held();
        let own = core_model::ScopeId::new(project).ok().and_then(|scope| held.get(&scope).cloned());
        let Some(project) = own.or_else(|| self.copy_read_in(store, project)) else {
            return Vec::new();
        };
        project.repositories().into_iter().map(|one| one.as_str().to_string()).collect()
    }

    fn feature_fault(&self, why: WriteError) -> Refusal {
        self.ledger_fault(why)
    }

    pub(crate) fn feature_unknown(&self, slug: &str) -> Refusal {
        Refusal::NotFound(ipc::WireError::raised(UNKNOWN, format!("no feature `{slug}` in this project"), self.run_id()))
    }

    /// `list_features`.
    pub(super) async fn features_listed(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<FeatureLibrary, Refusal> {
        self.feature_writes_filed().await;
        let tonight = self.librarian_night().await;
        let store = self.store().lock().await;
        let projects = self.projects_asked(&store, caller, project.as_deref())?;
        // A Job in several projects reads the first that has a library.
        let mut chosen = projects[0].clone();
        for one in &projects {
            if store.project_has_features(one).map_err(|why| self.feature_fault(why))? {
                chosen = one.clone();
                break;
            }
        }
        let loaded = Loaded::read(&store, &chosen).map_err(|why| self.feature_fault(why))?;
        let on = self.jobs_on_features(&store, &chosen)?;
        let mut areas: Vec<FeatureAreaRow> = loaded.areas.iter().map(|area| FeatureAreaRow { slug: area.slug.clone(), title: area.title.clone(), features: Vec::new() }).collect();
        let mut health = Health::new(&store, &loaded, self.repositories_of(&store, &loaded.project));
        let upkeep = store.upkeep_in(&chosen).map_err(|why| self.feature_fault(why))?;
        // Every feature's sections are read against one read of the merges after the oldest of them.
        let sections = loaded.map.features().iter().map(|feature| store.feature_sections(&feature.id)).collect::<Result<Vec<_>, _>>().map_err(|why| self.feature_fault(why))?;
        let merges = match sections.iter().flatten().map(|one| one.written_at.as_str()).min() {
            Some(oldest) => owned(&loaded.map, store.feature_merges_after("", oldest).map_err(|why| self.feature_fault(why))?),
            None => Vec::new(),
        };
        for (feature, sections) in loaded.map.features().iter().zip(sections) {
            let marked = marked(feature, sections, &merges);
            let stale = marked.iter().filter_map(|(_, since)| *since).min();
            let jobs = on.get(&feature.id).cloned().unwrap_or_default();
            let row = row_of(feature, health.of(feature, &marked, &jobs), jobs, stale, upkeep.iter().find(|mark| mark.slug == feature.slug));
            match areas.iter_mut().find(|area| area.slug == feature.area) {
                Some(area) => area.features.push(row),
                // A feature whose area was never kept is listed under its own slug rather than dropped.
                None => areas.push(FeatureAreaRow { slug: feature.area.clone(), title: feature.area.clone(), features: vec![row] }),
            }
        }
        let mut librarian = super::librarian::state_of(&store, &chosen).map_err(|why| self.feature_fault(why))?;
        librarian.steps = self.librarian_steps(&store, &chosen, &librarian, &tonight)?;
        librarian.running = self.librarian_running().now(&chosen);
        Ok(FeatureLibrary { project: chosen, areas, librarian })
    }

    /// The Jobs on each feature of the project: those that filed a file under it and are live and
    /// hold a worktree slot now, the fact `list_jobs` shows as `slot`. A live Job holding none has
    /// stopped without ending, and is not at work on anything.
    pub(super) fn jobs_on_features(&self, store: &Store, project: &str) -> Result<BTreeMap<String, Vec<FeatureJob>>, Refusal> {
        let pairs = store.feature_job_pairs(project).map_err(|why| self.feature_fault(why))?;
        let jobs: BTreeSet<&str> = pairs.iter().map(|(_, job)| job.as_str()).collect();
        let mut live: BTreeMap<String, String> = BTreeMap::new();
        for job in jobs {
            if let Ok(Some(session)) = store.session(job) {
                if session.state == store::SessionState::Live && holds_a_slot(store, job) {
                    live.insert(job.to_string(), session.title.unwrap_or_else(|| job.to_string()));
                }
            }
        }
        let mut on: BTreeMap<String, Vec<FeatureJob>> = BTreeMap::new();
        for (feature, job) in pairs {
            if let Some(title) = live.get(&job) {
                on.entry(feature).or_default().push(FeatureJob { session_id: SessionId::carried(&job), title: title.clone() });
            }
        }
        Ok(on)
    }

    /// Find the feature `slug` in the first of `projects` that has it.
    fn feature_in(&self, store: &Store, projects: &[String], slug: &str) -> Result<(Loaded, KeptFeature), Refusal> {
        for project in projects {
            let loaded = Loaded::read(store, project).map_err(|why| self.feature_fault(why))?;
            if let Some(feature) = loaded.feature_named(slug.trim()).cloned() {
                return Ok((loaded, feature));
            }
        }
        Err(self.feature_unknown(slug))
    }

    /// `get_feature_page`. With `section` the page holds that section alone, and no links or files. Without `with_files` the
    /// floor plan's rooms are empty of files and git is not asked.
    pub(super) async fn feature_paged(&self, caller: Option<api::Caller>, project: Option<String>, slug: String, section: Option<String>, with_files: bool) -> Result<FeaturePage, Refusal> {
        let only = match section.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
            None => None,
            Some(name) => Some(FeatureSectionKind::from_name(&name.to_ascii_lowercase()).ok_or_else(|| {
                Refusal::Unacceptable(ipc::WireError::raised(SECTION_UNKNOWN, format!("no section `{name}`: it is one of test, runs, observe or debt"), self.run_id()))
            })?),
        };
        self.feature_writes_filed().await;
        let store = self.store().lock().await;
        let projects = self.projects_asked(&store, caller, project.as_deref())?;
        let (loaded, feature) = self.feature_in(&store, &projects, &slug)?;
        let on = self.jobs_on_features(&store, &loaded.project)?;
        let marked = sections_marked(&store, &loaded, &feature).map_err(|why| self.feature_fault(why))?;
        let stale = marked.iter().filter_map(|(_, since)| *since).min();
        let jobs = on.get(&feature.id).cloned().unwrap_or_default();
        let health = Health::new(&store, &loaded, self.repositories_of(&store, &loaded.project)).of(&feature, &marked, &jobs);
        let (links, files, decisions) = if only.is_some() {
            (Vec::new(), Vec::new(), Vec::new())
        } else {
            (
                store.feature_links(&feature.id).map_err(|why| self.feature_fault(why))?,
                store.feature_top_files(&feature.id, 20).map_err(|why| self.feature_fault(why))?,
                store.feature_decisions(&feature.id).map_err(|why| self.feature_fault(why))?,
            )
        };
        let mut docs = reads_of(&links, feature_link_kind::DOC);
        if !docs.is_empty() {
            for kept in store.doc_drift_in(&loaded.project).map_err(|why| self.feature_fault(why))? {
                if let Some(doc) = docs.iter_mut().find(|doc| doc.target == kept.target) {
                    doc.drifted = kept.names;
                }
            }
            // A doc the upkeep check found no longer a file of the repository keeps its gist and says `gone`.
            let gone = store.docs_gone_in(&loaded.project).map_err(|why| self.feature_fault(why))?;
            for doc in docs.iter_mut().filter(|doc| gone.contains(&doc.target)) {
                doc.state = "gone".to_string();
            }
            super::doc_checking::checks_onto(&store, &feature.id, &mut docs).map_err(|why| self.feature_fault(why))?;
        }
        let upkeep = store.upkeep_in(&loaded.project).map_err(|why| self.feature_fault(why))?;
        let tickets = reads_of(&links, feature_link_kind::TICKET);
        let mut page = FeaturePage {
            project: loaded.project.clone(),
            feature: row_of(&feature, health, jobs, stale, upkeep.iter().find(|mark| mark.slug == feature.slug)),
            manifest_id: Some(feature.manifest_id.clone()).filter(|one| !one.is_empty()),
            paths: feature.paths.clone(),
            entry_points: feature.entry_points.clone(),
            sections: marked
                .into_iter()
                .filter_map(|(section, since)| section_on_the_wire(section, since))
                .filter(|section| only.is_none_or(|kind| section.section == kind))
                .collect(),
            links: links.into_iter().filter_map(link_on_the_wire).collect(),
            decisions: decisions.into_iter().map(decision_on_the_wire).collect(),
            files: files.into_iter().map(|(path, jobs)| FeatureFile { path, jobs }).collect(),
            docs,
            tickets,
            floor_plan: None,
        };
        drop(store);
        if only.is_none() {
            page.floor_plan = self.floor_plan_of(&loaded, &feature, with_files).await;
        }
        Ok(page)
    }

    /// `get_feature_brief`.
    pub(super) async fn feature_briefed(&self, caller: Option<api::Caller>, project: Option<String>, slug: String) -> Result<FeatureBrief, Refusal> {
        self.feature_writes_filed().await;
        let mine = self.owner_logins().await;
        let store = self.store().lock().await;
        let projects = self.projects_asked(&store, caller, project.as_deref())?;
        let (loaded, feature) = self.feature_in(&store, &projects, &slug)?;
        self.feature_brief_of(&store, &loaded, &feature, &mine)
    }

    /// One feature's page condensed, read from `store` with the library `loaded` holds it in.
    pub(super) fn feature_brief_of(&self, store: &Store, loaded: &Loaded, feature: &KeptFeature, mine: &Mine) -> Result<FeatureBrief, Refusal> {
        let on = self.jobs_on_features(store, &loaded.project)?;
        let marked = sections_marked(store, loaded, feature).map_err(|why| self.feature_fault(why))?;
        let links = store.feature_links(&feature.id).map_err(|why| self.feature_fault(why))?;
        let files = store.feature_top_files(&feature.id, 5).map_err(|why| self.feature_fault(why))?;
        let newest = store.feature_merge_newest(&feature.manifest_id).map_err(|why| self.feature_fault(why))?;
        let area = loaded.areas.iter().find(|area| area.slug == feature.area).map(|area| area.title.clone()).unwrap_or_else(|| feature.area.clone());
        let text_of = |name: &str| marked.iter().find(|(section, _)| section.section == name).and_then(|(section, _)| first_line(&section.text));
        let mut entry_points = feature.entry_points.clone();
        for (path, _) in files {
            if !entry_points.iter().any(|point| point.split(':').next() == Some(path.as_str())) {
                entry_points.push(path);
            }
        }
        let facts = BriefFacts {
            slug: feature.slug.clone(),
            title: feature.title.clone(),
            area,
            current_as_of: newest.map(|(number, at)| (number, date(&at))),
            stale: marked
                .iter()
                .filter_map(|(section, since)| Some((FeatureSectionKind::from_name(&section.section)?.name().to_string(), (*since)?)))
                .collect(),
            jobs_now: on.get(&feature.id).map(|jobs| jobs.iter().map(|job| job.title.clone()).collect()).unwrap_or_default(),
            elsewhere: elsewhere_of(store, loaded, feature, mine),
            entry_points,
            test: text_of(store::feature_section::TEST),
            bugs: open_bugs(&links),
            runs: text_of(store::feature_section::RUNS),
            designs: designs(&links),
            debt: text_of(store::feature_section::DEBT),
        };
        let text = condensed(&facts);
        Ok(FeatureBrief { feature: feature.slug.clone(), tokens: tokens_of(&text), text })
    }
}

/// Whether `job` has a standing `slot` row in the ledger, which is what `list_jobs` reads as its `slot`.
fn holds_a_slot(store: &Store, job: &str) -> bool {
    store
        .attachments_of(&store::Holder::session(job))
        .unwrap_or_default()
        .iter()
        .any(|one| one.kind == "slot" && one.state == store::AttachmentState::Standing)
}

/// The proposal to retire a feature, where one stands: the owner has not kept it on this evidence.
fn retire_of(mark: Option<&KeptUpkeep>) -> Option<ipc::RetireProposal> {
    let mark = mark.filter(|mark| mark.kept_at.is_none())?;
    let why = match mark.kind.as_str() {
        upkeep_kind::RETIRE_GONE => ipc::RetireWhy::CodeGone,
        upkeep_kind::RETIRE_UNUSED => ipc::RetireWhy::Unused,
        _ => return None,
    };
    Some(ipc::RetireProposal { why, evidence: mark.evidence.clone() })
}

fn row_of(feature: &KeptFeature, health: ipc::FeatureHealth, jobs_now: Vec<FeatureJob>, stale_since: Option<u64>, upkeep: Option<&KeptUpkeep>) -> FeatureRow {
    FeatureRow {
        id: feature.id.clone(),
        slug: feature.slug.clone(),
        area: feature.area.clone(),
        title: feature.title.clone(),
        summary: feature.summary.clone(),
        state: state_of(&feature.state),
        jobs_now,
        stale_since,
        health,
        retire: retire_of(upkeep),
        planned: upkeep.is_some_and(|mark| mark.kind == upkeep_kind::PLANNED),
    }
}

fn section_on_the_wire(section: KeptSection, stale_since: Option<u64>) -> Option<FeatureSection> {
    Some(FeatureSection {
        section: FeatureSectionKind::from_name(&section.section)?,
        text: section.text,
        job: section.job,
        written_at: instant(&section.written_at),
        commit: Some(section.at_commit).filter(|one| !one.is_empty()),
        stale_since,
    })
}

fn link_on_the_wire(link: KeptFeatureLink) -> Option<FeatureLink> {
    Some(FeatureLink {
        kind: FeatureLinkKind::from_name(&link.kind)?,
        target: link.target,
        title: link.title,
        state: link.state,
        job: Some(link.job).filter(|one| !one.is_empty()),
        via: link.via,
        at: instant(&link.at),
    })
}

fn decision_on_the_wire(kept: KeptFeatureDecision) -> FeatureDecision {
    FeatureDecision { question: kept.question, choice: kept.choice, note: Some(kept.note).filter(|note| !note.is_empty()), job: kept.job, at: instant(&kept.at) }
}

/// Whose words a gist is, in the order they win: a Job's, then the librarian's, then a first sentence.
fn gist_rank(by: &str) -> u8 {
    match by {
        store::feature_gist_by::JOB => 2,
        store::feature_gist_by::LIBRARIAN => 1,
        _ => 0,
    }
}

/// The docs or tickets of `kind` the links hold, newest first, **each once by what it was read by**: the newest read of it
/// names the Job, and the gist is a Job's where any of them wrote one, and else the librarian's. A read in two Jobs is one entry.
fn reads_of(links: &[KeptFeatureLink], kind: &str) -> Vec<FeatureRead> {
    let mut reads: Vec<FeatureRead> = Vec::new();
    let mut ranks: Vec<u8> = Vec::new();
    for link in links.iter().rev().filter(|link| link.kind == kind) {
        if let Some(at) = reads.iter().position(|read| read.target == link.target) {
            // A gist a Job wrote wins over the librarian's, and the librarian's over a first sentence heard later.
            if gist_rank(&link.gist_by) > ranks[at] {
                reads[at].gist = link.gist.clone();
                reads[at].gist_by_job = link.gist_by == store::feature_gist_by::JOB;
                ranks[at] = gist_rank(&link.gist_by);
            }
            continue;
        }
        ranks.push(gist_rank(&link.gist_by));
        reads.push(FeatureRead {
            target: link.target.clone(),
            title: Some(link.title.clone()).filter(|title| !title.is_empty()).unwrap_or_else(|| link.target.clone()),
            gist: link.gist.clone(),
            gist_by_job: link.gist_by == store::feature_gist_by::JOB,
            job: link.job.clone(),
            at: instant(&link.at),
            state: link.state.clone(),
            drifted: Vec::new(),
            check: None,
        });
    }
    reads
}

/// The open bugs, newest first.
fn open_bugs(links: &[KeptFeatureLink]) -> Vec<String> {
    links
        .iter()
        .rev()
        .filter(|link| link.kind == feature_link_kind::BUG && link.state != "closed")
        .map(|link| if link.title.is_empty() { link.target.clone() } else { link.title.clone() })
        .collect()
}

/// The titles of the sketches, pages, walks and drafts, newest first, each once.
fn designs(links: &[KeptFeatureLink]) -> Vec<String> {
    let mut titles: Vec<String> = Vec::new();
    for link in links.iter().rev() {
        if matches!(link.kind.as_str(), feature_link_kind::SKETCH | feature_link_kind::PAGE | feature_link_kind::WALK | feature_link_kind::DRAFT) {
            let title = if link.title.is_empty() { link.target.rsplit('/').next().unwrap_or(&link.target).to_string() } else { link.title.clone() };
            if !titles.contains(&title) {
                titles.push(title);
            }
        }
    }
    titles
}
