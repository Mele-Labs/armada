//! The model's check of a feature's docs against its code, and the owner's two answers to a doc that disagrees.
//! `docs/concepts/feature-catalog.md`, *Librarian*.
//!
//! **Only his press of Check runs it**: no librarian run and no tick does, and the free drift part goes on beside it. Each
//! doc of the feature that is a file of a repository at `HEAD` is put to the small model with the lines of the feature's code
//! its names point at, one call a doc, each spent from the night's share (`features.librarianModelShare`). **Nothing is
//! written back**: a disagreement waits on his Start a Job, which asks him which is right, or his Dismiss.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{CheckDocs, CheckedDoc, DocCheck, DocDisagreement, DocFlag, DocJobStarted, DocsChecked, FeatureRead, Instant, ManifestId, SendSessionMessage, SessionId, StartSession, WireError};
use store::{feature_link_kind, KeptDocCheck, KeptFeature, Store, WriteError};

use super::consulting::cut;
use super::doc_drift::{git, inside, names_in};
use super::reading::Loaded;
use crate::daemon::Fleet;

/// The most docs one press checks.
const MOST_DOCS: usize = 8;
/// How much of a doc the prompt carries, in bytes.
const DOC_BYTES: usize = 6000;
/// How much of the code the prompt carries, in bytes.
const CODE_BYTES: usize = 8000;
/// The most lines a name the doc gives brings in, and a line's longest.
const HITS_A_NAME: usize = 3;
const LINE_MOST: usize = 160;
/// The lines an entry point brings in.
const ENTRY_LINES: usize = 40;
/// The longest quote kept, in characters.
const QUOTE_MOST: usize = 200;

/// The night's share has no call left.
const SHARE_SPENT: &str = "fleet.librarian_share_spent";
/// No repository of the project is served, so there is no code.
const NO_REPOSITORY: &str = "fleet.feature_no_repository";
/// No doc of the feature is a file of a repository at `HEAD`.
const NO_DOC: &str = "fleet.no_doc_to_check";
/// Every call was made and no answer could be read.
const CHECK_FAILED: &str = "fleet.doc_check_failed";
/// No disagreement stands on the doc.
const NOT_FLAGGED: &str = "fleet.doc_not_flagged";

/// A doc as the check read it: its text at `HEAD`, the code it is put beside, and git's ids of both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DocRead {
    pub(super) target: String,
    pub(super) text: String,
    pub(super) code: String,
    pub(super) stamp: String,
}

/// What the model said of one doc: `None` where they agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Verdict(pub(super) Option<DocDisagreement>);

/// The docs linked to `feature` by a read, newest first, each once, that are not addresses.
fn docs_of(store: &Store, feature: &KeptFeature) -> Result<Vec<String>, WriteError> {
    let mut targets: Vec<String> = Vec::new();
    for link in store.feature_links(&feature.id)?.into_iter().rev().filter(|link| link.kind == feature_link_kind::DOC) {
        if !link.target.contains("://") && !targets.contains(&link.target) {
            targets.push(link.target);
        }
    }
    Ok(targets)
}

/// `text` cut to at most `most` bytes on a character's edge.
fn bytes_of(text: &str, most: usize) -> &str {
    if text.len() <= most {
        return text;
    }
    let mut at = most;
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    &text[..at]
}

fn plain_path(path: &str) -> &str {
    path.trim_start_matches("./").trim_matches('/')
}

/// The lines of the feature's `paths` at `HEAD` in `root` that hold a name the doc gives, a few a name, then its entry
/// points' opening lines, as `path:line: text`, at most [`CODE_BYTES`].
fn code_of(root: &Path, doc: &str, names: &[String], paths: &[String], entry_points: &[String]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut size = 0;
    let mut push = |line: String, lines: &mut Vec<String>| {
        if size + line.len() + 1 > CODE_BYTES {
            return false;
        }
        size += line.len() + 1;
        lines.push(line);
        true
    };
    if !names.is_empty() && !paths.is_empty() {
        let mut args: Vec<String> = ["grep", "-n", "-I", "-F", "-w"].map(String::from).to_vec();
        for name in names {
            args.extend(["-e".to_string(), name.clone()]);
        }
        args.extend(["HEAD".to_string(), "--".to_string()]);
        args.extend(paths.iter().map(|path| format!(":(literal){}", plain_path(path))));
        args.push(format!(":(exclude,literal){doc}"));
        let found = git(root, &args, &[0, 1]).unwrap_or_default();
        let mut each = vec![0usize; names.len()];
        for line in found.lines() {
            let Some((path, rest)) = line.strip_prefix("HEAD:").and_then(|rest| rest.split_once(':')) else { continue };
            let Some((number, text)) = rest.split_once(':') else { continue };
            let Some(at) = names.iter().position(|name| text.contains(name.as_str())) else { continue };
            if each[at] >= HITS_A_NAME {
                continue;
            }
            each[at] += 1;
            if !push(format!("{path}:{number}: {}", cut(text.trim(), LINE_MOST)), &mut lines) {
                break;
            }
        }
    }
    for point in entry_points {
        let (path, from) = match point.rsplit_once(':') {
            Some((path, line)) if line.parse::<usize>().is_ok() => (path, line.parse::<usize>().unwrap_or(1).saturating_sub(5).max(1)),
            _ => (point.as_str(), 1),
        };
        let Some(text) = git(root, &["show".to_string(), format!("HEAD:{}", plain_path(path))], &[0]) else { continue };
        for (at, line) in text.lines().enumerate().skip(from - 1).take(ENTRY_LINES) {
            if !push(format!("{path}:{}: {}", at + 1, cut(line.trim_end(), LINE_MOST)), &mut lines) {
                break;
            }
        }
    }
    lines.join("\n")
}

/// Git's ids of `doc` and of each of `paths` at `HEAD` in `root`: what the doc says and what the code is, so a dismissal
/// holds until either changes. A path git cannot name stands as `-`.
fn stamp_of(root: &Path, doc: &str, paths: &[String]) -> Option<String> {
    let id = |path: &str| git(root, &["rev-parse".to_string(), format!("HEAD:{path}")], &[0]).map(|out| out.trim().to_string());
    let doc = id(doc)?;
    let code: Vec<String> = paths.iter().map(|path| id(plain_path(path)).unwrap_or_else(|| "-".to_string())).collect();
    Some(format!("{doc} {}", code.join(",")))
}

/// Each of `targets`, at most [`MOST_DOCS`], that is a file of a repository in `roots` at `HEAD`, read with the code it is
/// put beside. **Blocking**: the caller runs it off the main thread.
pub(super) fn read_docs(roots: &[PathBuf], targets: &[String], paths: &[String], entry_points: &[String]) -> Vec<DocRead> {
    let mut found = Vec::new();
    for target in targets {
        if found.len() >= MOST_DOCS {
            break;
        }
        let one = roots.iter().find_map(|root| {
            let path = inside(root, target)?;
            let text = git(root, &["show".to_string(), format!("HEAD:{path}")], &[0])?;
            let text = bytes_of(text.trim(), DOC_BYTES).to_string();
            if text.is_empty() {
                return None;
            }
            let stamp = stamp_of(root, &path, paths)?;
            let code = code_of(root, &path, &names_in(&text), paths, entry_points);
            Some(DocRead { target: target.clone(), text, code, stamp })
        });
        found.extend(one);
    }
    found
}

/// What the model is asked of one doc: the rules, the feature, the doc, then the code.
pub(super) fn question(feature: &str, doc: &DocRead) -> String {
    let code = if doc.code.is_empty() { "(no lines of the code were found for the names this document gives)" } else { doc.code.as_str() };
    format!(
        "Below is a document from a software project and code from the feature it describes. Say whether the document says something the code shown contradicts.\n\n\
         Rules:\n\
         - Where nothing the document says is contradicted by the code shown, answer AGREES and nothing else.\n\
         - Otherwise answer these three lines and nothing else, about the one disagreement that matters most:\n\
         DOC: a quote of at most 25 words from the document\n\
         CODE: the path:line of the code that contradicts it, as written below\n\
         WHAT: at most 25 words quoting or saying what the code does there\n\
         - Something the document names that the code shown does not include is not a disagreement.\n\
         - No advice, no opinion, no dashes.\n\n\
         Feature: {feature}\n\n\
         Document {}:\n{}\n\n\
         Code:\n{code}",
        doc.target, doc.text
    )
}

/// One line kept as Fleet keeps a model's words: on one line, no quote marks round it, no dashes, cut.
fn said(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let flat = flat.trim_matches(|c: char| c == '"' || c == '\'' || c == '`').replace(" - ", ", ").replace(['\u{2014}', '\u{2013}'], ",").replace(" ,", ",");
    cut(&flat, QUOTE_MOST)
}

/// The model's reply read: agreement, a disagreement whose three lines are all there with a `path:line`, or `None` for a
/// reply Fleet cannot read.
pub(super) fn verdict_of(reply: &str) -> Option<Verdict> {
    let mut lines = reply.lines().map(|line| line.trim().trim_start_matches(['*', '-', '#']).trim()).filter(|line| !line.is_empty());
    let field = |name: &str, line: &str| line.strip_prefix(name).map(|rest| rest.trim_start_matches(['*', ':']).trim().to_string());
    let first = lines.clone().next()?;
    if first.trim_end_matches('.').eq_ignore_ascii_case("AGREES") {
        return Some(Verdict(None));
    }
    let (mut doc, mut at, mut what) = (None, None, None);
    for line in lines.by_ref() {
        doc = doc.or_else(|| field("DOC", line));
        at = at.or_else(|| field("CODE", line));
        what = what.or_else(|| field("WHAT", line));
    }
    let at = at?.trim_matches('`').to_string();
    let (path, number) = at.rsplit_once(':')?;
    if path.is_empty() || number.parse::<u32>().is_err() {
        return None;
    }
    let (doc, what) = (said(&doc?), said(&what?));
    (!doc.is_empty() && !what.is_empty()).then(|| Verdict(Some(DocDisagreement { doc_quote: doc, code_at: at, code_quote: what })))
}

/// A kept check as the wire carries it.
pub(super) fn check_of(kept: &KeptDocCheck) -> DocCheck {
    let disagrees = kept.disagrees().then(|| DocDisagreement { doc_quote: kept.doc_quote.clone(), code_at: kept.code_at.clone(), code_quote: kept.code_quote.clone() });
    DocCheck { at: Instant::carried(kept.checked_at.clone()), disagrees, dismissed: kept.disagrees() && !kept.flagged() }
}

/// What the checks of `feature_id` found, onto the docs a page serves.
pub(super) fn checks_onto(store: &Store, feature_id: &str, docs: &mut [FeatureRead]) -> Result<(), WriteError> {
    for kept in store.doc_checks_of(feature_id)? {
        if let Some(doc) = docs.iter_mut().find(|doc| doc.target == kept.target) {
            doc.check = Some(check_of(&kept));
        }
    }
    Ok(())
}

/// The model calls `project`'s librarian spent on `night`: its runs' and its presses of Check.
pub(crate) fn night_spent(store: &Store, project: &str, night: &str) -> Result<u32, WriteError> {
    Ok(store.librarian_model_calls(project, night)?.saturating_add(store.librarian_check_calls(project, night)?))
}

/// The first message of a Job started on a doc that disagrees with the code. **Sent as it stands**, so it is short and
/// plain, and it asks before anything changes.
pub(super) fn brief_of(feature: &KeptFeature, target: &str, kept: &KeptDocCheck) -> String {
    let mut brief = format!(
        "Doc or code: {}\n\nWhen the person pressed Check on the feature {} (`{}`), the model found one of its docs disagreeing with its code.\n\n\
         The doc, {target}:\n> {}\n\nThe code, {}:\n> {}",
        feature.title, feature.title, feature.slug, kept.doc_quote, kept.code_at, kept.code_quote
    );
    if !feature.paths.is_empty() {
        brief.push_str(&format!("\n\nThe feature's code is under {}.", feature.paths.iter().map(|path| format!("`{path}`")).collect::<Vec<_>>().join(", ")));
    }
    brief.push_str(
        "\n\nRead both, then ask the person which is right before you change anything: the doc, the code, or neither. \
         Change only what he picks.",
    );
    brief
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
    /// The served repositories of `project`, by Manifest id and root, in served order. `None` for a project Fleet does not hold.
    pub(super) fn served_members(&self, project: &str) -> Option<Vec<(String, PathBuf)>> {
        let held = self.projects_held();
        let one = held.running().find(|one| one.id().as_str() == project)?;
        let served = self.repositories().served();
        Some(served.iter().filter(|repository| one.holds(repository.manifest().id())).map(|repository| (repository.manifest().id().as_str().to_string(), PathBuf::from(repository.root()))).collect())
    }

    /// The repositories of `project`, refused where Fleet holds no such project or serves none of its repositories.
    fn checked_repositories(&self, project: &str) -> Result<Vec<(String, PathBuf)>, Refusal> {
        let Some(repositories) = self.served_members(project) else {
            return Err(self.hosted_refusal(super::seeding::PROJECT, &format!("no project is named `{project}`")));
        };
        if repositories.is_empty() {
            return Err(self.hosted_refusal(NO_REPOSITORY, &format!("No repository of project `{project}` is served here, so there is no code to check against.")));
        }
        Ok(repositories)
    }

    /// `check_docs`. **Nothing is written back**, and nothing is asked where the share is spent.
    pub(super) async fn docs_checked(&self, caller: Option<api::Caller>, check: CheckDocs) -> Result<DocsChecked, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = check.project.trim().to_string();
        let repositories = self.checked_repositories(&project)?;
        let night = self.librarian_night().await;
        let (feature, targets, kept, spent) = {
            let store = self.store().lock().await;
            let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
            let feature = loaded.feature_named(check.slug.trim()).cloned().ok_or_else(|| self.feature_unknown(&check.slug))?;
            let targets = docs_of(&store, &feature).map_err(|why| self.ledger_fault(why))?;
            let kept = store.doc_checks_of(&feature.id).map_err(|why| self.ledger_fault(why))?;
            let spent = night_spent(&store, &project, &night).map_err(|why| self.ledger_fault(why))?;
            (feature, targets, kept, spent)
        };
        // The feature's own repository first, where it names one.
        let mut roots: Vec<PathBuf> = repositories.iter().filter(|(id, _)| *id == feature.manifest_id).map(|(_, root)| root.clone()).collect();
        roots.extend(repositories.iter().filter(|(id, _)| *id != feature.manifest_id).map(|(_, root)| root.clone()));
        let (paths, points, reading) = (feature.paths.clone(), feature.entry_points.clone(), targets);
        let read = tokio::task::spawn_blocking(move || read_docs(&roots, &reading, &paths, &points)).await.unwrap_or_default();
        if read.is_empty() {
            return Err(self.hosted_refusal(NO_DOC, "No doc of this feature is a file of the repository, so there is nothing to check."));
        }
        // A dismissed doc whose text and code read as they did is quiet, and not asked again.
        let asking: Vec<DocRead> = read.into_iter().filter(|one| !kept.iter().any(|held| held.target == one.target && held.disagrees() && held.dismissed.as_deref() == Some(one.stamp.as_str()))).collect();
        let share = self.librarian_set().model_share;
        let left = share.saturating_sub(spent);
        if left == 0 && !asking.is_empty() {
            return Err(Refusal::IllegalMove(WireError::raised(SHARE_SPENT, format!("The librarian's share is spent tonight: {spent} of {share} calls."), self.run_id())));
        }
        let at = self.now().as_str().to_string();
        let mut calls: u32 = 0;
        let mut checked = Vec::new();
        let mut unchecked = Vec::new();
        for one in &asking {
            if calls >= left {
                unchecked.push(one.target.clone());
                continue;
            }
            calls += 1;
            let reply = match self.story_asked(&question(&feature.title, one)).await {
                Ok(reply) => reply,
                Err(why) => {
                    eprintln!("fleet: the check of {} for feature {} was not made: {why}", one.target, feature.slug);
                    continue;
                }
            };
            let Some(Verdict(found)) = verdict_of(&reply) else { continue };
            let (doc_quote, code_at, code_quote) = found.map(|one| (one.doc_quote, one.code_at, one.code_quote)).unwrap_or_default();
            let row = KeptDocCheck { feature_id: feature.id.clone(), target: one.target.clone(), checked_at: at.clone(), doc_quote, code_at, code_quote, stamp: one.stamp.clone(), dismissed: None };
            self.store().lock().await.keep_doc_check(&row).map_err(|why| self.ledger_fault(why))?;
            checked.push(one.target.clone());
        }
        let mut store = self.store().lock().await;
        if calls > 0 {
            store.keep_librarian_check_calls(&project, &night, &at, calls).map_err(|why| self.ledger_fault(why))?;
        }
        if calls > 0 && checked.is_empty() {
            let used = spent + calls;
            return Err(Refusal::Fault(WireError::raised(CHECK_FAILED, format!("The model gave no answer Fleet could read, {used} of {share} calls tonight."), self.run_id())));
        }
        let kept = store.doc_checks_of(&feature.id).map_err(|why| self.ledger_fault(why))?;
        let checked = kept.iter().filter(|held| checked.contains(&held.target)).map(|held| CheckedDoc { target: held.target.clone(), check: check_of(held) }).collect();
        Ok(DocsChecked { project, slug: feature.slug, checked, unchecked, calls })
    }

    /// The feature `slug` of `project` and the check of `target` that found it disagreeing, where that stands.
    async fn flag_held(&self, flag: &DocFlag) -> Result<(String, KeptFeature, KeptDocCheck), Refusal> {
        let project = flag.project.trim().to_string();
        let store = self.store().lock().await;
        let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
        let feature = loaded.feature_named(flag.slug.trim()).cloned().ok_or_else(|| self.feature_unknown(&flag.slug))?;
        let kept = store.doc_checks_of(&feature.id).map_err(|why| self.ledger_fault(why))?.into_iter().find(|held| held.target == flag.target && held.flagged());
        let Some(kept) = kept else {
            return Err(Refusal::IllegalMove(WireError::raised(NOT_FLAGGED, format!("No check found `{}` disagreeing with the code of `{}`.", flag.target, feature.slug), self.run_id())));
        };
        Ok((project, feature, kept))
    }

    /// `dismiss_doc_flag`. **Nothing is kept on a refusal.**
    pub(super) async fn doc_flag_dismissed(&self, caller: Option<api::Caller>, flag: DocFlag) -> Result<DocFlag, Refusal> {
        self.owner_or_operator_called(caller)?;
        let (project, feature, kept) = self.flag_held(&flag).await?;
        self.store().lock().await.dismiss_doc_check(&feature.id, &kept.target).map_err(|why| self.ledger_fault(why))?;
        Ok(DocFlag { project, slug: feature.slug, target: kept.target })
    }

    /// `start_doc_job`: a Job in the project, sent the brief Fleet writes from what it kept.
    pub(super) async fn doc_job_started(self: &Arc<Self>, caller: Option<api::Caller>, flag: DocFlag) -> Result<DocJobStarted, Refusal> {
        self.owner_or_operator_called(caller)?;
        let (project, feature, kept) = self.flag_held(&flag).await?;
        let repositories = self.checked_repositories(&project)?;
        let manifest = repositories.iter().find(|(id, _)| *id == feature.manifest_id).or_else(|| repositories.first()).map(|(id, _)| id.clone()).unwrap_or_default();
        let title = format!("Doc or code: {}", feature.title);
        let start = StartSession {
            project: Some(project.clone()),
            manifest_id: ManifestId::carried(manifest),
            title: Some(title.clone()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
            workflow: None,
        };
        let started = HostedSessions::start_session(self.as_ref(), start).await?.id.as_str().to_string();
        let brief = brief_of(&feature, &kept.target, &kept);
        let sent = Arc::clone(self)
            .send_session_message(SendSessionMessage { session_id: SessionId::carried(started.clone()), text: brief.clone(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })
            .await;
        // **Not an error**: the Job exists, so the answer is 200 and `send_failed` says why nothing was sent.
        let send_failed = sent.err().map(|refused| refused.error().message.clone());
        Ok(DocJobStarted { project, slug: feature.slug, target: kept.target, session_id: SessionId::carried(started), title, brief, send_failed })
    }
}
