//! What Fleet knows of one part, in the nine sections Bridge shows. **Read from what Fleet already holds**: the feature
//! catalog, the Jobs' observed writes, git's own log, the issue lists and open pull requests kept. No forge call, no model.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FeatureJob, ManifestId, PartIssue, PartOwner, PartPull, PartWatch, SessionId, SystemPartAnswers};

use super::levels::Ground;
use super::model::{holds, Mapped};
use super::repo::{extension_of, text_at};
use super::scanning::literals;
use crate::daemon::Fleet;
use crate::feature_library::mapping::FeatureMap;
use crate::feature_library::ranking::Ground as RankGround;

/// The window commits are counted over, in days.
const OWNER_DAYS: u32 = 30;
const MOST_OWNERS: usize = 6;
const MOST_DOCS: usize = 20;
/// The most documents read, and the largest one.
const DOCS_READ: usize = 300;
const DOC_LARGEST: u64 = 100_000;
/// The most files read for a log folder, and the most such folders named.
const LOGS_READ: usize = 120;
const MOST_LOGS: usize = 3;

fn is_doc(path: &str) -> bool {
    matches!(extension_of(path), "md" | "mdx" | "rst" | "txt") && (path.starts_with("docs/") || !path.contains('/') || path.to_ascii_lowercase().ends_with("readme.md"))
}

/// Whether `text` names the part: one of its folders, or its name as a whole word.
fn names(text: &str, name: &str, paths: &[String]) -> bool {
    let folder = |path: &String| {
        let path = path.trim_matches('/');
        path.len() >= 4 && path != "." && text.contains(path)
    };
    if paths.iter().any(folder) {
        return true;
    }
    let lower = text.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    name.len() >= 4 && lower.match_indices(&name).any(|(at, _)| {
        let before = lower[..at].chars().next_back();
        let after = lower[at + name.len()..].chars().next();
        before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric())
    })
}

/// The share of the commits over `paths` each author made in the last `OWNER_DAYS` days, most first.
fn owners_of(root: &Path, paths: &[String]) -> Vec<PartOwner> {
    let mut git = crate::files::git_in(root);
    git.args(["log", &format!("--since={OWNER_DAYS}.days"), "--format=%aN", "--"]);
    let specs: Vec<&str> = paths.iter().map(|path| path.trim_end_matches('/')).filter(|path| !path.is_empty() && *path != ".").collect();
    if specs.is_empty() && !paths.is_empty() {
        git.arg(".");
    }
    git.args(specs);
    let Ok(out) = git.output() else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for author in String::from_utf8_lossy(&out.stdout).lines().filter(|line| !line.is_empty()) {
        *counts.entry(author.to_string()).or_default() += 1;
    }
    let total: u32 = counts.values().sum();
    let mut owners: Vec<(String, u32)> = counts.into_iter().collect();
    owners.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    owners.into_iter().take(MOST_OWNERS).map(|(name, count)| PartOwner { name, share: u8::try_from((count * 100 + total / 2) / total.max(1)).unwrap_or(100) }).collect()
}

/// The documents of the repository that name the part, as repository-relative paths.
fn docs_naming(root: &Path, map: &Mapped, name: &str, paths: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for (file, _) in map.files.iter().filter(|(file, _)| is_doc(file)).take(DOCS_READ) {
        if paths.iter().any(|path| holds(path, file)) && !paths.iter().any(|path| path.trim_matches('/') == ".") {
            // A document inside the part's own folder names it by being there.
            if !file.to_ascii_lowercase().ends_with("readme.md") {
                continue;
            }
        }
        if let Some(text) = text_at(root, file, DOC_LARGEST) {
            if names(&text, name, paths) {
                found.push(file.clone());
                if found.len() >= MOST_DOCS {
                    break;
                }
            }
        }
    }
    found
}

/// The folders of logs the part's code names: a quoted path that holds `log`.
fn logs_named(root: &Path, map: &Mapped, paths: &[String]) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    let files = map.files.iter().map(|(file, _)| file).filter(|file| paths.iter().any(|path| holds(path, file)) && matches!(extension_of(file), "rs" | "py" | "js" | "ts" | "go" | "yml" | "yaml" | "toml")).take(LOGS_READ);
    for file in files {
        let Some(text) = text_at(root, file, 200_000) else { continue };
        for literal in literals(&text) {
            let lower = literal.to_ascii_lowercase();
            if (literal.starts_with("~/") || literal.starts_with('/')) && lower.contains("log") && !literal.contains(' ') && !literal.contains('{') && literal.len() < 120 && literal.matches('/').count() >= 2 {
                found.insert(literal.to_string());
            }
        }
        if found.len() >= MOST_LOGS {
            break;
        }
    }
    found.into_iter().take(MOST_LOGS).collect()
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
    /// `get_system_part`.
    pub(crate) async fn system_part_read(&self, project: String, part: String) -> Result<SystemPartAnswers, Refusal> {
        let maps = self.system_mapped(&project).await?;
        let (features, filed, additions, librarian_at) = self.system_ground(&project, &maps).await?;
        let map = FeatureMap::new(features.clone(), filed);
        let in_map = |feature: &str, file: &str| map.filed_of(feature).any(|one| one == file);
        let ground = Ground { project: &project, maps: &maps, features: &features, filed: &in_map, additions: &additions, librarian_at };
        let there = ground
            .describe(&part)
            .ok_or_else(|| Refusal::NotFound(ipc::WireError::raised("fleet.system_part_unknown", format!("the map holds no part `{part}`"), self.run_id())))?;
        let (repo, manifest, name, screen, at, paths) = (there.repo, there.manifest, there.name, there.screen, there.part, there.paths);
        let mapped = &maps[repo];
        let root = self.system_members(&project).await?.into_iter().find(|member| member.manifest == manifest).map(|member| member.root).unwrap_or_default();

        let held = [(repo, part.clone(), paths.clone())];
        let features_of = ground.features_over(&held);
        let slugs: BTreeSet<&str> = features_of.iter().map(|feature| feature.slug.as_str()).collect();

        let working = self.system_working(&manifest, &paths).await;
        let (owners, docs, logs) = {
            let (root, mapped, name, paths) = (root.clone(), maps[repo].clone(), name.clone(), paths.clone());
            tokio::task::spawn_blocking(move || (owners_of(&root, &paths), docs_naming(&root, &mapped, &name, &paths), logs_named(&root, &mapped, &paths))).await.unwrap_or_default()
        };
        let monitor = self.system_monitor(mapped, at, &manifest, &paths, logs);
        let (issues, tracker) = self.system_issues(&project, &manifest, &features, &slugs, &map).await;
        let pulls = self.system_pulls(&manifest, &paths).await;
        Ok(SystemPartAnswers {
            project,
            part,
            features: features_of,
            code: paths,
            working,
            owners,
            owner_days: OWNER_DAYS,
            looks: None,
            has_screen: screen,
            docs,
            monitor,
            issues,
            tracker,
            pulls,
        })
    }

    /// The live Jobs whose observed writes fall in `paths` of the repository `manifest`.
    async fn system_working(&self, manifest: &str, paths: &[String]) -> Vec<FeatureJob> {
        let writers: Vec<String> = self
            .writes()
            .holders()
            .into_iter()
            .filter(|(_, held, written)| held == manifest && written.iter().any(|file| paths.iter().any(|path| holds(path, file))))
            .map(|(who, _, _)| match who {
                crate::clashing::Who::Session(id) => id,
            })
            .collect();
        let store = self.store().lock().await;
        let mut jobs = Vec::new();
        for id in writers {
            if let Ok(Some(session)) = store.session(&id) {
                if session.state == store::SessionState::Live {
                    jobs.push(FeatureJob { title: session.title.unwrap_or_else(|| id.clone()), session_id: SessionId::carried(&id) });
                }
            }
        }
        jobs
    }

    /// What to watch the part by: Kit's servers, the health routes it serves, the log folders its code names and the
    /// Checks that cover its files.
    fn system_monitor(&self, mapped: &Mapped, at: Option<usize>, manifest: &str, paths: &[String], logs: Vec<String>) -> Vec<PartWatch> {
        let mut watch: Vec<PartWatch> = Vec::new();
        if let Some(at) = at {
            for (_, server) in mapped.kit.iter().filter(|(part, _)| *part == at) {
                watch.push(PartWatch { label: server.name.clone(), kind: "server".to_string() });
            }
            let port = mapped.parts[at].port.map(|port| format!(":{port}")).unwrap_or_default();
            for (_, route) in mapped.routes.iter().filter(|(part, route)| *part == at && ["health", "ping", "ready", "status", "alive"].iter().any(|word| route.to_ascii_lowercase().contains(word))) {
                watch.push(PartWatch { label: format!("GET {port}{route}"), kind: "health".to_string() });
            }
        }
        watch.extend(logs.into_iter().map(|label| PartWatch { label, kind: "log".to_string() }));
        if let Some(served) = self.repositories().serving(manifest) {
            let files: Vec<String> = mapped.files.iter().map(|(file, _)| file).filter(|file| paths.iter().any(|path| holds(path, file))).take(200).cloned().collect();
            let manifest = served.manifest();
            for name in manifest.check_names() {
                if manifest.check(&name).is_some_and(|check| check.when().is_some() && check.covers(&files)) {
                    watch.push(PartWatch { label: name, kind: "check".to_string() });
                }
            }
        }
        watch
    }

    /// The open issues the part's features match, from the lists Fleet holds, and whether a tracker is read at all.
    async fn system_issues(&self, project: &str, manifest: &str, features: &[store::KeptFeature], slugs: &BTreeSet<&str>, map: &FeatureMap) -> (Vec<PartIssue>, bool) {
        let tracker = self.project_held(project).await.is_ok_and(|held| held.values().tracker.is_some()) || self.repositories().serving(manifest).is_some_and(|served| served.manifest().tracker().is_some());
        let Some(held) = self.tracking().held(manifest) else { return (Vec::new(), tracker) };
        let Ok(list) = self.issue_list(ManifestId::carried(manifest), held, None).await else { return (Vec::new(), tracker) };
        let pulls = self.store().lock().await.open_pulls_kept().unwrap_or_default();
        let recent = BTreeMap::new();
        let ground = RankGround { map, recent: &recent, pulls: &pulls };
        let issues = list
            .issues
            .into_iter()
            .filter(|issue| issue.state == ipc::IssueState::Open)
            .filter(|issue| ground.issue(Some(manifest), issue).0.iter().any(|matched| slugs.contains(matched.slug.as_str()) && features.iter().any(|feature| feature.slug == matched.slug)))
            .map(|issue| PartIssue { id: issue.reference, title: issue.title })
            .collect();
        (issues, tracker)
    }

    /// The open pull requests whose files fall in `paths` of the repository `manifest`, from those the review lists kept.
    async fn system_pulls(&self, manifest: &str, paths: &[String]) -> Vec<PartPull> {
                let pulls = self.store().lock().await.open_pulls_kept().unwrap_or_default();
        pulls
            .into_iter()
            .filter(|pull| pull.manifest_id == manifest && pull.files.iter().any(|file| paths.iter().any(|path| holds(path, file))))
            .map(|pull| PartPull { number: pull.number, title: pull.title, author: pull.author })
            .collect()
    }
}
