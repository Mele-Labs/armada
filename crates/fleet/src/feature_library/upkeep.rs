//! The librarian's free upkeep part: each feature read against the code at `HEAD`, and each linked doc that is a file of the
//! repository. **No model is asked.** `docs/concepts/feature-catalog.md`, *Librarian*.
//!
//! A feature whose paths match no file and never did is **planned**. One whose paths matched files once and match none now,
//! or whose files and Jobs have been quiet for [`UNUSED_DAYS`], is **proposed for Retire**, with the commit and date it rests on.
//! Fleet only proposes: `answer_retire` is the owner's. A linked doc that is a file of the repository and is not at `HEAD` is
//! **gone**. Where git cannot say, nothing is marked and what was marked stays.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use store::{feature_state, upkeep_kind, Store, UpkeepFound, WriteError};

use super::doc_drift::{git, inside, MOST_DOCS};

/// How long a feature's files and its Jobs may be quiet before Retire is proposed. **Chosen, not measured**: about two
/// quarters, long enough that a feature between releases is not called unused.
pub(crate) const UNUSED_DAYS: i64 = 180;
/// The most features checked in one run.
const MOST_FEATURES: usize = 400;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// One feature the check reads.
#[derive(Clone, Debug)]
struct Asking {
    slug: String,
    paths: Vec<String>,
    /// When the newest link of any kind was kept for it, in milliseconds.
    last_job: Option<i64>,
}

/// What a check reads of the store, taken before git is asked.
#[derive(Clone, Debug)]
pub(crate) struct Asked {
    features: Vec<Asking>,
    docs: Vec<String>,
    now: i64,
}

/// The features the check reads in `project`: those named, with paths. `Pending` ones are the owner's to confirm first.
fn features_checked(store: &Store, project: &str) -> Result<Vec<store::KeptFeature>, WriteError> {
    let mut features: Vec<store::KeptFeature> = store.features_in(project)?.into_iter().filter(|one| one.state != feature_state::PENDING && !one.paths.is_empty()).collect();
    features.truncate(MOST_FEATURES);
    Ok(features)
}

/// How many features the check reads in `project`.
pub(crate) fn checked_count(store: &Store, project: &str) -> Result<usize, WriteError> {
    Ok(features_checked(store, project)?.len())
}

/// What the check of `project` reads of the store, at `now` milliseconds, over the linked `docs`.
pub(crate) fn asked(store: &Store, project: &str, docs: &[String], now: i64) -> Result<Asked, WriteError> {
    let mut features = Vec::new();
    for feature in features_checked(store, project)? {
        let newest = store
            .feature_links(&feature.id)?
            .iter()
            // A time Fleet cannot read counts as now, so it proposes nothing.
            .map(|link| core_model::Timestamp::from_rfc3339(link.at.clone()).epoch_millis().unwrap_or(now))
            .max();
        features.push(Asking { slug: feature.slug, paths: feature.paths, last_job: newest });
    }
    Ok(Asked { features, docs: docs.to_vec(), now })
}

/// What git said of one feature.
#[derive(Clone, Debug)]
struct Seen {
    /// Some file at `HEAD` is one of its paths or under one.
    files_now: bool,
    /// The newest commit that touched its paths, and when, in seconds. Absent where none ever did.
    last: Option<(String, i64)>,
}

/// What git said of the features and the docs.
#[derive(Clone, Debug)]
pub(crate) struct Inspected {
    /// One for each of [`Asked`]'s features, in its order.
    features: Vec<Seen>,
    /// The linked docs that are files of the repository and are not at `HEAD`.
    gone: Vec<String>,
}

/// Every file at `HEAD` in the checkout at `root`. `None` where git could not say, as in a folder that is no repository.
fn head_files(root: &Path) -> Option<BTreeSet<String>> {
    let listing = git(root, &["ls-tree".to_string(), "-r".to_string(), "--name-only".to_string(), "-z".to_string(), "HEAD".to_string()], &[0])?;
    Some(listing.split('\0').filter(|path| !path.is_empty()).map(str::to_string).collect())
}

/// Whether some file in `files` is `prefix` or under it, on whole segments.
fn holds(files: &BTreeSet<String>, prefix: &str) -> bool {
    let prefix = prefix.trim_start_matches("./").trim_matches('/');
    if prefix.is_empty() {
        return false;
    }
    let under = format!("{prefix}/");
    files.contains(prefix) || files.range::<str, _>((std::ops::Bound::Included(under.as_str()), std::ops::Bound::Unbounded)).next().is_some_and(|file| file.starts_with(&under))
}

/// The newest commit in `root` that touched `paths`: the outer `None` where git could not say, the inner where none did.
fn last_commit(root: &Path, paths: &[String]) -> Option<Option<(String, i64)>> {
    let mut args: Vec<String> = ["log", "-1", "--format=%H%x09%ct", "HEAD", "--"].map(String::from).to_vec();
    args.extend(paths.iter().map(|path| format!(":(literal){}", path.trim_start_matches("./").trim_matches('/'))));
    let out = git(root, &args, &[0])?;
    Some(out.trim().split_once('\t').and_then(|(hash, at)| Some((hash.to_string(), at.parse::<i64>().ok()?))))
}

/// Ask git about `asked` in each of `roots`. `None` where it could not say of any one of them, so nothing is marked on a
/// doubt. **Blocking**: the caller runs it off the main thread.
pub(crate) fn inspect(roots: &[PathBuf], asked: &Asked) -> Option<Inspected> {
    let mut trees: Vec<(&Path, BTreeSet<String>)> = Vec::new();
    for root in roots {
        trees.push((root.as_path(), head_files(root)?));
    }
    if trees.is_empty() {
        return None;
    }
    let mut features = Vec::new();
    for feature in &asked.features {
        let files_now = feature.paths.iter().any(|path| trees.iter().any(|(_, files)| holds(files, path)));
        let mut last: Option<(String, i64)> = None;
        for (root, _) in &trees {
            if let Some(found) = last_commit(root, &feature.paths)? {
                if last.as_ref().is_none_or(|(_, at)| found.1 > *at) {
                    last = Some(found);
                }
            }
        }
        features.push(Seen { files_now, last });
    }
    let mut gone = Vec::new();
    let mut checked = 0;
    for target in &asked.docs {
        if checked >= MOST_DOCS {
            break;
        }
        // A doc is a file of the repository where some root holds its path. It is gone where none has it at `HEAD`, and none
        // has it on disk either, so an untracked doc is not called gone.
        let places: Vec<(&Path, &BTreeSet<String>, String)> = trees.iter().filter_map(|(root, files)| Some((*root, files, inside(root, target)?))).collect();
        if places.is_empty() {
            continue;
        }
        checked += 1;
        if places.iter().all(|(root, files, path)| !files.contains(path) && !root.join(path).exists()) {
            gone.push(target.clone());
        }
    }
    Some(Inspected { features, gone })
}

fn short(hash: &str) -> &str {
    hash.get(..7).unwrap_or(hash)
}

/// `14 May`, and the year where it is not this one's.
fn day(millis: i64, now: i64) -> String {
    let (stamp, this) = (crate::clock::rfc3339_utc(millis), crate::clock::rfc3339_utc(now));
    let month = stamp.get(5..7).and_then(|month| month.parse::<usize>().ok()).and_then(|month| MONTHS.get(month.checked_sub(1)?)).copied().unwrap_or("");
    let date = stamp.get(8..10).unwrap_or("").trim_start_matches('0');
    match stamp.get(..4) == this.get(..4) {
        true => format!("{date} {month}"),
        false => format!("{date} {month} {}", stamp.get(..4).unwrap_or("")),
    }
}

/// What the check finds of one feature: a kind and its evidence, or nothing. **Doubt is nothing.**
fn judged(feature: &Asking, seen: &Seen, now: i64) -> Option<(String, String)> {
    match (&seen.last, seen.files_now) {
        (None, false) => Some((upkeep_kind::PLANNED.to_string(), String::new())),
        // Files at `HEAD` and no commit that touched them: git cannot say, so nothing is proposed.
        (None, true) => None,
        (Some((hash, at)), false) => Some((upkeep_kind::RETIRE_GONE.to_string(), format!("Its paths match no file since {}, {}", short(hash), day(at * 1000, now)))),
        (Some((hash, at)), true) => {
            let quiet = |then: i64| now - then > UNUSED_DAYS * DAY_MS;
            if !quiet(at * 1000) || !feature.last_job.is_none_or(quiet) {
                return None;
            }
            let job = feature.last_job.map_or_else(|| "no Job has been on it".to_string(), |then| format!("no Job on it since {}", day(then, now)));
            Some((upkeep_kind::RETIRE_UNUSED.to_string(), format!("No change to its files since {}, {}; {job}", short(hash), day(at * 1000, now))))
        }
    }
}

/// How many of each the last check left standing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counted {
    pub(crate) retire: usize,
    pub(crate) planned: usize,
    pub(crate) dead_links: usize,
}

/// Keep what `seen` says for `project` at `at`, and count what stands: a proposal the owner kept on this evidence is not one.
pub(crate) fn keep(store: &mut Store, project: &str, at: &str, asked: &Asked, seen: &Inspected) -> Result<Counted, WriteError> {
    let found: Vec<UpkeepFound> = asked.features.iter().zip(&seen.features).map(|(feature, seen)| UpkeepFound { slug: feature.slug.clone(), found: judged(feature, seen, asked.now) }).collect();
    store.keep_upkeep(project, at, &found)?;
    store.keep_docs_gone(project, at, &seen.gone)?;
    let marks = store.upkeep_in(project)?;
    Ok(Counted {
        retire: marks.iter().filter(|mark| mark.kind != upkeep_kind::PLANNED && mark.kept_at.is_none()).count(),
        planned: marks.iter().filter(|mark| mark.kind == upkeep_kind::PLANNED).count(),
        dead_links: seen.gone.len(),
    })
}
