//! What Fleet knows of one part, in the nine sections Bridge shows. **Read from what Fleet already holds**: the feature
//! catalog, the Jobs' observed writes, git's own log, the issue lists and open pull requests kept. No forge call, no model.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FeatureJob, ManifestId, PartIssue, PartOwner, PartPull, PartWatch, SessionId, SystemPartAnswers};

use super::levels::Ground;
use super::model::{holds, within, Mapped};
use super::repo::{extension_of, text_at};
use super::scanning::literals;
use crate::daemon::Fleet;
use crate::feature_library::mapping::{owns, FeatureMap};
use crate::feature_library::ranking::Ground as RankGround;

/// The window commits are counted over, in days.
const OWNER_DAYS: u32 = 30;
const MOST_OWNERS: usize = 6;
const MOST_DOCS: usize = 20;
/// The most documents read, and the largest one.
const DOCS_READ: usize = 300;
const DOC_LARGEST: u64 = 100_000;
/// One repository's reads that touch the checkout.
#[derive(Default)]
struct Read {
    counts: BTreeMap<String, u32>,
    docs: Vec<String>,
    logs: Vec<String>,
}

/// One repository's share of what a form of the question asks about.
struct Scope {
    repo: usize,
    manifest: String,
    root: std::path::PathBuf,
    paths: Vec<String>,
    at: Option<usize>,
}

/// The most files of a feature that are named as its code.
const MOST_FILED: usize = 40;

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

/// The commits each author made over `paths` in the last `OWNER_DAYS` days.
fn commit_counts(root: &Path, paths: &[String]) -> BTreeMap<String, u32> {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut git = crate::files::git_in(root);
    git.args(["log", &format!("--since={OWNER_DAYS}.days"), "--format=%aN", "--"]);
    let specs: Vec<&str> = paths.iter().map(|path| path.trim_end_matches('/')).filter(|path| !path.is_empty() && *path != ".").collect();
    if specs.is_empty() && !paths.is_empty() {
        git.arg(".");
    }
    git.args(specs);
    let Ok(out) = git.output() else { return counts };
    if !out.status.success() {
        return counts;
    }
    for author in String::from_utf8_lossy(&out.stdout).lines().filter(|line| !line.is_empty()) {
        *counts.entry(author.to_string()).or_default() += 1;
    }
    counts
}

/// The share of the commits each author made, most first.
fn owners_from(counts: BTreeMap<String, u32>) -> Vec<PartOwner> {
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

/// The documents at the root of the repository, README first and then `docs/`, as repository-relative paths.
fn root_docs(map: &Mapped) -> Vec<String> {
    let mut found: Vec<String> = map.files.iter().map(|(file, _)| file.clone()).filter(|file| is_doc(file)).collect();
    found.sort_by(|a, b| a.contains('/').cmp(&b.contains('/')).then_with(|| a.cmp(b)));
    found.truncate(MOST_DOCS);
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
    /// `get_system_part`: one part, one feature, or neither, which is the whole project.
    pub(crate) async fn system_part_read(&self, project: String, part: Option<String>, feature: Option<String>) -> Result<SystemPartAnswers, Refusal> {
        let maps = self.system_mapped(&project).await?;
        let (features, filed, additions, librarian_at) = self.system_ground(&project, &maps).await?;
        let map = FeatureMap::new(features.clone(), filed);
        let in_map = |feature: &str, file: &str| map.filed_of(feature).any(|one| one == file);
        let ground = Ground { project: &project, maps: &maps, features: &features, filed: &in_map, additions: &additions, librarian_at };
        let members = self.system_members(&project).await?;
        let root_of = |manifest: &str| members.iter().find(|member| member.manifest == manifest).map(|member| member.root.clone()).unwrap_or_default();

        // What the question is about: the repositories it reaches, and the paths in each.
        let mut scopes: Vec<Scope> = Vec::new();
        let mut code: Vec<String> = Vec::new();
        let (mut screen, mut name) = (false, String::new());
        let shown: Vec<ipc::SystemFeature>;
        let slugs: BTreeSet<String>;
        let mut kept: Option<&store::KeptFeature> = None;
        if let Some(slug) = feature.as_deref().filter(|one| !one.is_empty()) {
            let one = features.iter().find(|one| one.slug == slug).ok_or_else(|| Refusal::NotFound(ipc::WireError::raised("fleet.system_feature_unknown", format!("the project holds no feature `{slug}`"), self.run_id())))?;
            for (repo, mapped) in maps.iter().enumerate().filter(|(_, mapped)| one.manifest_id.is_empty() || one.manifest_id == mapped.manifest) {
                let mut paths: Vec<String> = within(&mapped.folders, &one.paths);
                paths.extend(map.filed_of(&one.id).filter(|file| mapped.files.iter().any(|(held, _)| held == file) && !one.paths.iter().any(|prefix| owns(prefix, file))).map(str::to_string).take(MOST_FILED));
                if paths.is_empty() {
                    continue;
                }
                screen |= paths.iter().any(|path| ground.draws_screen_at(repo, path.trim_end_matches('/')));
                code.extend(paths.iter().cloned());
                scopes.push(Scope { repo, manifest: mapped.manifest.clone(), root: root_of(&mapped.manifest), paths, at: None });
            }
            shown = ground.all_features().into_iter().filter(|each| each.slug == slug).collect();
            slugs = BTreeSet::from([slug.to_string()]);
            kept = Some(one);
        } else if let Some(id) = part.as_deref().filter(|one| !one.is_empty()) {
            let there = ground
                .describe(id)
                .ok_or_else(|| Refusal::NotFound(ipc::WireError::raised("fleet.system_part_unknown", format!("the map holds no part `{id}`"), self.run_id())))?;
            let held = [(there.repo, id.to_string(), there.paths.clone())];
            shown = ground.features_over(&held);
            slugs = shown.iter().map(|one| one.slug.clone()).collect();
            screen = there.screen;
            name = there.name;
            code = there.paths.clone();
            scopes.push(Scope { repo: there.repo, manifest: there.manifest.clone(), root: root_of(&there.manifest), paths: there.paths, at: there.part });
        } else {
            for (repo, mapped) in maps.iter().enumerate() {
                let folder = members.iter().find(|member| member.manifest == mapped.manifest).and_then(|member| member.root.file_name().map(|name| name.to_string_lossy().into_owned()));
                let folder = folder.unwrap_or_else(|| mapped.manifest.clone());
                // A repository read by folders is those folders, everywhere the whole one is its root.
                let paths: Vec<String> = if mapped.folders.is_empty() { vec!["./".to_string()] } else { mapped.folders.iter().map(|one| format!("{one}/")).collect() };
                if mapped.folders.is_empty() {
                    code.push(if maps.len() > 1 { format!("{folder}/") } else { "./".to_string() });
                } else {
                    code.extend(paths.iter().map(|path| if maps.len() > 1 { format!("{folder}/{path}") } else { path.clone() }));
                }
                screen |= mapped.parts.iter().any(|one| one.screen);
                scopes.push(Scope { repo, manifest: mapped.manifest.clone(), root: root_of(&mapped.manifest), paths, at: None });
            }
            shown = ground.all_features();
            slugs = features.iter().map(|one| one.slug.clone()).collect();
        }
        let whole = feature.as_deref().is_none_or(str::is_empty) && part.as_deref().is_none_or(str::is_empty);

        let mut working: Vec<FeatureJob> = Vec::new();
        for scope in &scopes {
            for job in self.system_working(&scope.manifest, &scope.paths).await {
                if !working.iter().any(|one| one.session_id == job.session_id) {
                    working.push(job);
                }
            }
        }
        let mut counts: BTreeMap<String, u32> = BTreeMap::new();
        let mut docs: Vec<String> = Vec::new();
        let mut monitor: Vec<PartWatch> = Vec::new();
        let mut issues: Vec<PartIssue> = Vec::new();
        let mut pulls: Vec<PartPull> = Vec::new();
        let mut tracker = false;
        let slug_refs: BTreeSet<&str> = slugs.iter().map(String::as_str).collect();
        for scope in &scopes {
            let mapped = &maps[scope.repo];
            let (root, held, paths, doc_name, one_part) = (scope.root.clone(), mapped.clone(), scope.paths.clone(), name.clone(), !whole && kept.is_none());
            let read = tokio::task::spawn_blocking(move || Read {
                counts: commit_counts(&root, &paths),
                docs: if one_part {
                    docs_naming(&root, &held, &doc_name, &paths)
                } else if whole {
                    root_docs(&held)
                } else {
                    Vec::new()
                },
                logs: if one_part { logs_named(&root, &held, &paths) } else { Vec::new() },
            })
            .await
            .unwrap_or_default();
            for (author, count) in read.counts {
                *counts.entry(author).or_default() += count;
            }
            docs.extend(read.docs);
            let logs = read.logs;
            if whole && mapped.folders.is_empty() {
                monitor.extend(self.system_kit(&scope.manifest));
            } else if whole {
                // Read by folders: the servers that run in them.
                for (_, server) in &mapped.kit {
                    if !monitor.iter().any(|one| one.kind == "server" && one.label == server.name) {
                        monitor.push(PartWatch { label: server.name.clone(), kind: "server".to_string() });
                    }
                }
            } else {
                monitor.extend(self.system_monitor(mapped, scope.at, &scope.manifest, &scope.paths, logs));
            }
            let (found, has) = self.system_issues(&project, &scope.manifest, &features, &slug_refs, &map).await;
            tracker |= has;
            issues.extend(found.into_iter().filter(|issue| !issues.iter().any(|one| one.id == issue.id)).collect::<Vec<_>>());
            let open = if whole && mapped.folders.is_empty() { self.system_pulls_open(&scope.manifest).await } else { self.system_pulls(&scope.manifest, &scope.paths).await };
            pulls.extend(open.into_iter().filter(|pull| !pulls.iter().any(|one| one.number == pull.number)).collect::<Vec<_>>());
        }
        if let Some(one) = kept {
            docs = self.system_feature_docs(&one.id).await;
        }
        docs.dedup();
        docs.truncate(MOST_DOCS);
        Ok(SystemPartAnswers {
            project,
            part: part.filter(|_| !whole && kept.is_none()).unwrap_or_default(),
            feature: kept.map(|one| one.slug.clone()),
            features: shown,
            code,
            working,
            owners: owners_from(counts),
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

    /// Every server and run the repository's Kit declares.
    fn system_kit(&self, manifest: &str) -> Vec<PartWatch> {
        let Some(served) = self.repositories().serving(manifest) else { return Vec::new() };
        let manifest = served.manifest();
        let mut watch: Vec<PartWatch> = manifest.server_names().into_iter().map(|label| PartWatch { label, kind: "server".to_string() }).collect();
        watch.extend(manifest.app_run_names().iter().map(|label| PartWatch { label: label.clone(), kind: "run".to_string() }));
        watch
    }

    /// Every open pull request of the repository `manifest`, from those the review lists kept.
    async fn system_pulls_open(&self, manifest: &str) -> Vec<PartPull> {
        let pulls = self.store().lock().await.open_pulls_kept().unwrap_or_default();
        pulls.into_iter().filter(|pull| pull.manifest_id == manifest).map(|pull| PartPull { number: pull.number, title: pull.title, author: pull.author }).collect()
    }

    /// The documents the Jobs that changed the feature's files read first, newest first, each once.
    async fn system_feature_docs(&self, feature_id: &str) -> Vec<String> {
        let links = self.store().lock().await.feature_links(feature_id).unwrap_or_default();
        let mut docs: Vec<String> = Vec::new();
        for link in links.iter().rev().filter(|link| link.kind == store::feature_link_kind::DOC) {
            if !docs.contains(&link.target) {
                docs.push(link.target.clone());
            }
        }
        docs
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
