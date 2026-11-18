//! The two pages of the Features screen that belong to a project: its overview and what moved since he last looked.
//! Plain reads of the repository's files and the library's rows, no model. `docs/concepts/feature-catalog.md`,
//! *Project overview* and *Since you looked*.

use std::cmp::Reverse;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Timestamp;
use ipc::{BusyFeature, GlossaryTerm, LookedFeature, LookedTag, MapRoom, ProjectOverview, ReadmeSentence, SinceYouLooked};
use store::{feature_link_kind, feature_state, Store, WriteError};

use super::mapping::FeatureMap;
use super::modules::{self, Layout, DEEPEST, MOST_FOLDERS, SKIPPED};
use super::reading::Loaded;
use crate::daemon::Fleet;

/// How far back a merge counts toward a feature being busy. Chosen, not measured.
const BUSIEST_DAYS: i64 = 14;
/// How many features the overview lists as busiest. Chosen, not measured.
const BUSIEST_FEATURES: usize = 5;
/// How many pull request titles make a feature's line in Since you looked.
const WHAT_TITLES: usize = 3;
/// The longest sentence kept from a README, in characters.
const SENTENCE_CHARS: usize = 300;
/// The largest README read, in bytes.
const LARGEST_README: u64 = 256 * 1024;
/// The most files looked at in a repository, so a very large tree costs a bounded read.
const MOST_FILES: usize = 20_000;
const DAY_MS: i64 = 86_400_000;
const SINCE_UNREADABLE: &str = "fleet.feature_since_unreadable";
/// README names in the order they are preferred, compared in lower case.
const README_NAMES: &[&str] = &["readme.md", "readme.markdown", "readme.rst", "readme.txt", "readme"];

/// A merge of a pull request on a feature.
struct Merge {
    at: i64,
    number: u64,
    title: String,
}

/// A repository of the project as read for the map: its modules and the files under it.
struct Read {
    manifest: String,
    layout: Layout,
    files: Vec<String>,
}

/// The instant in milliseconds, `None` where it is no RFC 3339 time.
fn millis(at: &str) -> Option<i64> {
    Timestamp::from_rfc3339(at.trim().to_string()).epoch_millis()
}

/// The merges of every feature of `project`, each feature's newest first.
fn merges_of(store: &Store, project: &str) -> Result<BTreeMap<String, Vec<Merge>>, WriteError> {
    let mut by_feature: BTreeMap<String, Vec<Merge>> = BTreeMap::new();
    for link in store.project_links(project, feature_link_kind::MERGE)? {
        if let (Some(at), Ok(number)) = (millis(&link.at), link.target.parse::<u64>()) {
            by_feature.entry(link.feature_id).or_default().push(Merge { at, number, title: link.title });
        }
    }
    for merges in by_feature.values_mut() {
        merges.sort_by_key(|merge| Reverse((merge.at, merge.number)));
        merges.dedup_by_key(|merge| merge.number);
    }
    Ok(by_feature)
}

/// The files of the repository at `root`, relative and `/`-separated. **Blocking**, and bounded by folders, depth and files.
fn files_of(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::from([(String::new(), 0)]);
    let mut looked = 0;
    while let Some((rel, depth)) = queue.pop_front() {
        looked += 1;
        if looked > MOST_FOLDERS || files.len() >= MOST_FILES {
            break;
        }
        let Ok(listing) = std::fs::read_dir(if rel.is_empty() { root.to_path_buf() } else { root.join(&rel) }) else { continue };
        let mut entries: Vec<_> = listing.flatten().collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let below = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if depth < DEEPEST && !name.starts_with('.') && !SKIPPED.contains(&name.as_str()) {
                        queue.push_back((below, depth + 1));
                    }
                }
                Ok(kind) if kind.is_file() => files.push(below),
                _ => {}
            }
        }
    }
    files
}

/// The README of the repository at `root` and the first sentence of what it says.
fn readme_of(root: &Path) -> Option<ReadmeSentence> {
    let names: Vec<String> = std::fs::read_dir(root).ok()?.flatten().filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file())).map(|entry| entry.file_name().to_string_lossy().into_owned()).collect();
    let file = README_NAMES.iter().find_map(|want| names.iter().find(|name| name.to_ascii_lowercase() == *want))?;
    if std::fs::metadata(root.join(file)).ok()?.len() > LARGEST_README {
        return None;
    }
    let text = std::fs::read_to_string(root.join(file)).ok()?;
    Some(ReadmeSentence { sentence: first_sentence(&text)?, file: file.clone() })
}

/// Whether a line is a rule or a heading's underline: one of `-`, `=`, `*` or `_` repeated.
fn is_rule(line: &str) -> bool {
    line.len() >= 2 && line.chars().all(|c| matches!(c, '-' | '=' | '*' | '_'))
}

/// Whether a line is no prose: a heading, an image or badge, a tag, a table row, a quote or a list item.
fn is_furniture(line: &str) -> bool {
    let listed = ["- ", "* ", "+ "].iter().any(|mark| line.starts_with(mark)) || line.split_once(". ").is_some_and(|(number, _)| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()));
    ["#", "![", "[![", "<", "|", ">"].iter().any(|start| line.starts_with(start)) || is_rule(line) || listed
}

/// The first sentence of the first paragraph of prose in a README, with its markup taken off. `None` where there is none.
pub(crate) fn first_sentence(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let (mut fenced, mut front) = (false, false);
    let mut paragraph: Vec<&str> = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        if at == 0 && *line == "---" {
            front = true;
            continue;
        }
        if front {
            front = *line != "---";
            continue;
        }
        if line.starts_with("```") || line.starts_with("~~~") {
            fenced = !fenced;
        }
        let heading = paragraph.is_empty() && lines.get(at + 1).is_some_and(|next| is_rule(next));
        let prose = !fenced && !line.is_empty() && !line.starts_with("```") && !line.starts_with("~~~") && !is_furniture(line) && !heading;
        match (prose, paragraph.is_empty()) {
            (true, _) => paragraph.push(line),
            (false, true) => {}
            (false, false) => break,
        }
    }
    let text = unmarked(&paragraph.join(" "));
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then(|| ended(&text))
}

/// `text` with links as their labels, images gone, and code and emphasis marks taken off.
fn unmarked(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    while at < chars.len() {
        let image = chars[at] == '!' && chars.get(at + 1) == Some(&'[');
        let start = at + usize::from(image);
        if chars[start.min(chars.len() - 1)] == '[' {
            if let Some((label, after)) = link_at(&chars, start) {
                if !image {
                    out.push_str(&unmarked(&label));
                }
                at = after;
                continue;
            }
        }
        if !matches!(chars[at], '`' | '*') {
            out.push(chars[at]);
        }
        at += 1;
    }
    out
}

/// The label and the index after a `[label](target)` that starts at `chars[start]`.
fn link_at(chars: &[char], start: usize) -> Option<(String, usize)> {
    let (mut depth, mut close) = (0, None);
    for (at, c) in chars.iter().enumerate().skip(start) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(at);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = chars.iter().enumerate().skip(close + 2).find(|(_, c)| **c == ')')?.0;
    Some((chars[start + 1..close].iter().collect(), end + 1))
}

/// `text` cut after its first full stop, question mark or exclamation mark that ends a word, and kept to a bounded length.
fn ended(text: &str) -> String {
    let marks: Vec<(usize, char)> = text.char_indices().collect();
    let mut end = text.len();
    for (n, (at, c)) in marks.iter().enumerate() {
        let closes = matches!(c, '.' | '!' | '?') && marks.get(n + 1).is_none_or(|(_, next)| next.is_whitespace());
        let word = text[..*at].rsplit(' ').next().unwrap_or("").to_ascii_lowercase();
        if closes && !["e.g", "i.e", "vs", "cf"].contains(&word.as_str()) {
            end = at + 1;
            break;
        }
    }
    let sentence = &text[..end];
    if sentence.chars().count() <= SENTENCE_CHARS {
        return sentence.to_string();
    }
    let cut: String = sentence.chars().take(SENTENCE_CHARS).collect();
    format!("{}…", cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head))
}

/// The slug of the area owning the most files of each room of `read`, or none where no feature owns one.
fn room_areas(read: &Read, map: &FeatureMap) -> Vec<Option<String>> {
    let mut counts: Vec<BTreeMap<&str, u32>> = vec![BTreeMap::new(); read.layout.modules.len()];
    for file in &read.files {
        let Some(room) = read.layout.holding(file) else { continue };
        let owner = map.owners(Some(&read.manifest), file).into_iter().find(|owner| owner.feature.state != feature_state::PENDING);
        if let Some(owner) = owner {
            *counts[room].entry(owner.feature.area.as_str()).or_default() += 1;
        }
    }
    counts.into_iter().map(|count| count.into_iter().max_by(|(a, most_a), (b, most_b)| most_a.cmp(most_b).then(b.cmp(a))).map(|(area, _)| area.to_string())).collect()
}

/// A feature's line in Since you looked: the titles of what merged since, or its summary where none carries one.
fn what_of(summary: &str, merged: &[&Merge]) -> String {
    let mut titles: Vec<&str> = Vec::new();
    for title in merged.iter().map(|merge| merge.title.trim()).filter(|title| !title.is_empty()) {
        if !titles.contains(&title) && titles.len() < WHAT_TITLES {
            titles.push(title);
        }
    }
    if titles.is_empty() {
        summary.to_string()
    } else {
        titles.join("; ")
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
    /// The library of the project a call is about, and its merges. A Job in several projects reads the first that has a library.
    async fn project_library(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<(Loaded, BTreeMap<String, Vec<Merge>>), Refusal> {
        self.feature_writes_filed().await;
        let store = self.store().lock().await;
        let projects = self.projects_asked(&store, caller, project.as_deref())?;
        let mut chosen = projects[0].clone();
        for one in &projects {
            if store.project_has_features(one).map_err(|why| self.ledger_fault(why))? {
                chosen = one.clone();
                break;
            }
        }
        let loaded = Loaded::read(&store, &chosen).map_err(|why| self.ledger_fault(why))?;
        let merges = merges_of(&store, &chosen).map_err(|why| self.ledger_fault(why))?;
        Ok((loaded, merges))
    }

    /// The repositories of `project` in the order it lists them, read off the main thread, with the first one's README.
    async fn repositories_read(&self, project: &str) -> (Vec<Read>, Option<ReadmeSentence>) {
        let repositories: Vec<(String, PathBuf)> = {
            let held = self.projects_held();
            let Some(one) = held.running().find(|one| one.id().as_str() == project) else {
                return (Vec::new(), None);
            };
            let served = self.repositories().served();
            one.repositories()
                .into_iter()
                .filter_map(|id| served.iter().find(|repository| repository.manifest().id().as_str() == id.as_str()))
                .map(|repository| (repository.manifest().id().as_str().to_string(), PathBuf::from(repository.root())))
                .collect()
        };
        let read = tokio::task::spawn_blocking(move || {
            let readme = repositories.first().and_then(|(_, root)| readme_of(root));
            let reads = repositories.into_iter().map(|(manifest, root)| Read { manifest, layout: modules::read(&root), files: files_of(&root) }).collect();
            (reads, readme)
        })
        .await;
        read.unwrap_or_default()
    }

    /// `get_project_overview`.
    pub(super) async fn project_overview_read(&self, caller: Option<api::Caller>, project: Option<String>) -> Result<ProjectOverview, Refusal> {
        let (loaded, merges) = self.project_library(caller, project).await?;
        let (reads, what_it_is) = self.repositories_read(&loaded.project).await;
        let several = reads.len() > 1;
        let mut rooms = Vec::new();
        for read in &reads {
            let areas = room_areas(read, &loaded.map);
            for (module, area) in read.layout.modules.iter().zip(areas) {
                rooms.push(MapRoom {
                    module: module.name.clone(),
                    path: if module.root.is_empty() { ".".to_string() } else { module.root.clone() },
                    kind: module.kind.label().to_string(),
                    manifest_id: several.then(|| read.manifest.clone()),
                    area,
                });
            }
        }
        let cutoff = self.now().epoch_millis().unwrap_or(0) - BUSIEST_DAYS * DAY_MS;
        let mut busy: Vec<(&store::KeptFeature, Vec<&Merge>)> = loaded
            .map
            .features()
            .iter()
            .filter_map(|feature| {
                let within: Vec<&Merge> = merges.get(&feature.id)?.iter().filter(|merge| merge.at >= cutoff).collect();
                (!within.is_empty()).then_some((feature, within))
            })
            .collect();
        busy.sort_by(|(a, with_a), (b, with_b)| with_b.len().cmp(&with_a.len()).then(with_b[0].at.cmp(&with_a[0].at)).then(a.slug.cmp(&b.slug)));
        busy.truncate(BUSIEST_FEATURES);
        let busiest = busy
            .into_iter()
            .map(|(feature, within)| BusyFeature { slug: feature.slug.clone(), title: feature.title.clone(), pulls: within.iter().map(|merge| merge.number).collect() })
            .collect();
        let glossary = self
            .store()
            .lock()
            .await
            .glossary(&loaded.project)
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .map(|term| GlossaryTerm { term: term.term, meaning: term.meaning })
            .collect();
        Ok(ProjectOverview { project: loaded.project, what_it_is, rooms, busiest, glossary })
    }

    /// `get_since_you_looked`.
    pub(super) async fn since_looked_read(&self, caller: Option<api::Caller>, project: Option<String>, since: String) -> Result<SinceYouLooked, Refusal> {
        let since_ms = millis(&since).ok_or_else(|| {
            Refusal::Unacceptable(ipc::WireError::raised(SINCE_UNREADABLE, format!("`{since}` is not a time: give it as 2026-11-18T09:30:00Z"), self.run_id()))
        })?;
        let (loaded, merges) = self.project_library(caller, project).await?;
        let mut listed: Vec<((u8, Reverse<i64>, String), LookedFeature)> = Vec::new();
        for feature in loaded.map.features() {
            let merged: Vec<&Merge> = merges.get(&feature.id).map(|all| all.iter().filter(|merge| merge.at > since_ms).collect()).unwrap_or_default();
            let added = millis(&feature.added_at).unwrap_or(0);
            let (tag, rank, moved) = if feature.state == feature_state::PENDING {
                (LookedTag::NeedsYou, 0, added)
            } else if added > since_ms {
                (LookedTag::New, 2, added)
            } else if let Some(newest) = merged.first() {
                (LookedTag::Changed, 1, newest.at)
            } else {
                continue;
            };
            listed.push((
                (rank, Reverse(moved), feature.slug.clone()),
                LookedFeature {
                    tag,
                    slug: feature.slug.clone(),
                    title: feature.title.clone(),
                    what: what_of(&feature.summary, &merged),
                    pulls: merged.iter().map(|merge| merge.number).collect(),
                },
            ));
        }
        listed.sort_by(|(a, _), (b, _)| a.cmp(b));
        Ok(SinceYouLooked { project: loaded.project, paragraph: String::new(), features: listed.into_iter().map(|(_, one)| one).collect() })
    }
}
