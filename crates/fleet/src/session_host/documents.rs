//! A Job's document: an HTML page it designed and wrote to `.armada/documents/<name>.html` in its
//! worktree, where git ignores it. **Each write that changes the page is kept as a revision**, in Fleet's
//! own attachments folder and not in the worktree, so a released slot keeps them. The page is one
//! artifact on the Job's ledger, upserted on its path, whose `revisions` detail names the kept files
//! oldest first. The sketches it embeds are checked as `show_sketch` checks one, and a Job whose page
//! holds one that will not draw is told, as is one whose page declares an action that will not run or a vote that
//! will not count. The
//! write is never refused, but a page that asks and buries its ask is kept as one for reading (`page_measure`). **A revision is put to the person
//! only where the page asks**: it marks itself `<meta name="armada-review" content="approve">`, or the
//! Job waits on a `document_approved` gate or step. Every other page is for reading, and the Job's own revision of
//! one waits on him as `read:<path>` until he opens it (`reads`). Read off the page
//! kept here and never off the tool call: `document_answers` answers it.
//! **A page is asked only of the Job that changed it.** The file stays in a slot that changes hands, so a
//! Job that took another's place is given the page's history, and any other Job passes over a page
//! another Job left as it kept it. `docs/concepts/document.md`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Sessions;
use ipc::{AttachmentReport, PlanGate, SessionFact, SessionId, SessionReport, DOCUMENT_ASKED, DOCUMENT_ITEM_PREFIX, DOCUMENT_KEPT_AT, DOCUMENT_REVISION};
use store::{Holder, KeptAttachment};

use super::document_answers::{is_document, DOCUMENT_READS, READ_ITEM_PREFIX};
use super::artifact_for::with_for;
use super::page_actions::actions_that_will_not_run;
use super::page_measure::buried_ask;
use super::page_votes::votes_that_will_not_count;
use super::document_page::{asks_approval, for_of, sketches_in, title_of};
use crate::daemon::Fleet;
use crate::rail_ends;

/// What a page's path is cut at to name its folder of revisions, so the same page is the same folder
/// whichever slot the Job wrote it from.
const DOCUMENTS: &str = "/.armada/documents/";

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
    /// Where a Job's pages are kept, a folder to each page.
    pub(crate) fn documents_of(&self, id: &str) -> PathBuf {
        Path::new(&self.host().attachments_dir).join("documents").join(id)
    }

    /// Fleet's own copy of the page a Job wrote at `path` in any checkout: **the Document, and its id**, one a Job
    /// and file name, so a slot given back, reset or moved from neither loses nor doubles it.
    pub(crate) fn page_home(&self, id: &str, path: &str) -> String {
        let home = self.documents_of(id);
        if Path::new(path).starts_with(&home) {
            return path.to_string();
        }
        home.join(DOCUMENTS.trim_matches('/')).join(page_name(path)).to_string_lossy().into_owned()
    }

    /// The page at `path` was written and the tool has answered, so the file on disk is the Job's.
    pub(super) async fn page_written(self: &Arc<Self>, id: &str, path: String) {
        let folder = self.documents_of(id).join(folder_of(&path));
        let read = path.clone();
        let kept = tokio::task::spawn_blocking(move || {
            let html = std::fs::read_to_string(&read)?;
            Ok::<_, io::Error>(keep(&folder, html.as_bytes(), "html")?.map(|revisions| (html, revisions)))
        })
        .await;
        // Unreadable, not text, or the bytes the newest revision holds: nothing new to keep.
        let Ok(Ok(Some((html, revisions)))) = kept else { return };
        let home = self.page_home(id, &path);
        let Some(told) = self.page_kept(id, &home, Some(&path), &html, &revisions, true).await else { return };
        let (fleet, id) = (Arc::clone(self), id.to_string());
        // Off the hearing of the stream: a word to the Job is a send, and the stream is what it waits on.
        tokio::spawn(async move {
            let _ = fleet.told_by_fleet(&id, &told, ipc::Notice::DocumentSketch).await;
        });
    }

    /// The page at `path`, Fleet's own copy ([`Self::page_home`]), was kept as its newest revision, `html`: the copy
    /// holds it and its row says so, asking again where it asks. `written` is the file in the Job's checkout it came
    /// from, where it came from one. What the Job is to be told about the page, where anything. His Edit keeps a
    /// revision this way too, and `measured` is false for it: only the Job's own revision is held back for an ask it buried.
    pub(super) async fn page_kept(&self, id: &str, path: &str, written: Option<&str>, html: &str, revisions: &[PathBuf], measured: bool) -> Option<String> {
        let names: Vec<String> = revisions.iter().map(|one| one.to_string_lossy().into_owned()).collect();
        let encoded = ipc::encode(&names).ok()?;
        let (home, bytes) = (PathBuf::from(path), html.as_bytes().to_vec());
        let copied = tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(home.parent().unwrap_or(&home))?;
            std::fs::write(&home, bytes)
        })
        .await;
        if !matches!(copied, Ok(Ok(()))) {
            return None;
        }
        let moved = self.page_moved_home(id, path).await;
        let written_at = written.map(str::to_string).or(moved);
        // The Job reads its page where it writes it, never at Fleet's copy.
        let shown = shown_page(path);
        let styles = self.page_styles(id, html).await;
        let asks = asks_approval(html) || self.approval_gate_waits(id, PlanGate::DocumentApproved, rail_ends::End::DocumentApproved).await;
        // A page that buries its ask is kept and does not reach him: it reads, and the Job is told what to move.
        // A vote or buttons ask him too, on a page with no Approve mark.
        let presses = { let lower = html.to_ascii_lowercase(); lower.contains("<armada-vote") || lower.contains("<armada-action") };
        let buried = if (asks || presses) && measured { buried_ask(html, &shown) } else { None };
        // The Job's own revision of a page that asks nothing waits on him to read it. One that buried its ask goes
        // back to the Job, and his Edit he has read.
        let reads = measured && !asks && !presses;
        let asks = asks && buried.is_none();
        let (text, links) = for_of(html);
        let (said, dropped) = ipc::ArtifactFor::kept(text.as_deref(), &links);
        // The store replaces a row's detail whole, so leaving `settled`, `settled_at` and `words` out of
        // this one is what asks the person again about the new revision, where it asks.
        let mut detail: std::collections::BTreeMap<String, String> = [
            ("form".to_string(), "document".to_string()),
            ("title".to_string(), title_of(html, path)),
            ("revisions".to_string(), encoded),
            (DOCUMENT_ASKED.to_string(), asks.to_string()),
            (DOCUMENT_REVISION.to_string(), names.len().to_string()),
            (DOCUMENT_KEPT_AT.to_string(), self.instant().as_str().to_string()),
        ]
        .into();
        if reads {
            detail.insert(DOCUMENT_READS.to_string(), String::from("true"));
        }
        detail.extend(written_at.map(|at| (DOCUMENT_WRITTEN_AT.to_string(), at)));
        // The page is not rewritten, because `keep` compares its bytes: the style sets its mocks were drawn with ride beside it.
        detail.extend(styles.kept.clone().map(|kept| ("kept_styles".to_string(), kept)));
        with_for(&mut detail, self.for_line(id, said).await.as_ref());
        detail.extend(super::document_page::features_of(html).map(|slugs| (ipc::ARTIFACT_FEATURE.to_string(), slugs)));
        // **One open artifact per Job**: a page that asks, written while one still asks him, is a part of it. A page
        // that joined one keeps it across its revisions.
        let kept_in = {
            let store = self.store().lock().await;
            store
                .attachments_of(&store::Holder::session(id))
                .ok()
                .and_then(|held| held.into_iter().find(|one| one.kind == "artifact" && one.target == path))
                .and_then(|one| one.detail.get(ipc::ARTIFACT_IN).cloned())
        };
        let joins = match (kept_in, asks) {
            (Some(root), _) => Some(root),
            (None, true) => self.artifact_joined(id, adapters::HOSTED_HARNESS, path, &title_of(html, path), false).await.ok().flatten(),
            (None, false) => None,
        };
        detail.extend(joins.map(|root| (ipc::ARTIFACT_IN.to_string(), root)));
        // A page he dismissed waits again once it is revised, as a sketch drawn again does.
        let item = format!("{DOCUMENT_ITEM_PREFIX}{path}");
        let _ = self.store().lock().await.restore_waiting(id, &item);
        let _ = self.store().lock().await.restore_waiting(id, &format!("{READ_ITEM_PREFIX}{path}"));
        let _ = self
            .report_session(SessionReport {
                harness: String::from(adapters::HOSTED_HARNESS),
                session_id: SessionId::carried(id),
                fact: SessionFact::Attached {
                    attachment: AttachmentReport { kind: String::from("artifact"), target: path.to_string(), detail },
                },
            })
            .await;
        let closes = self.closes_on(id, html).await;
        let told = [sketches_that_will_not_draw(html, &shown), actions_that_will_not_run(html, &shown, &closes), votes_that_will_not_count(html, &shown), buried]
            .into_iter()
            .flatten()
            .reduce(|told, more| format!("{told} {more}"));
        match (styles.told(told, &shown), links_dropped(&dropped)) {
            (Some(told), Some(links)) => Some(format!("{told} {links}")),
            (None, Some(links)) => Some(format!("In the page {shown}, {}", links.replacen("The", "the", 1))),
            (told, None) => told,
        }
    }

    /// Every other page of the Job's with the file name of `home`, Fleet's copy, moved to it: one an earlier build
    /// kept at the path it was written at. Where the checkout file it was last written at, if any.
    async fn page_moved_home(&self, id: &str, home: &str) -> Option<String> {
        let mut store = self.store().lock().await;
        let held = store.attachments_of(&Holder::session(id)).unwrap_or_default();
        let name = page_name(home);
        let mut written = held.iter().find(|one| one.kind == "artifact" && one.target == home).and_then(|one| one.detail.get(DOCUMENT_WRITTEN_AT).cloned());
        for old in held.iter().filter(|one| one.kind == "artifact" && one.target != home && is_document(&one.detail) && page_name(&one.target) == name) {
            if store.move_page(id, &old.target, home).is_ok() && written.is_none() {
                written = Some(old.target.clone());
            }
        }
        written
    }

    /// Whether the Job waits on his approval of a page or a sketch, `gate` and `end` naming which: its plan
    /// holds that gate open, or the workflow step it is on ends on it. So a page that does not mark itself asks
    /// too, and `all_done` leaves the sketch it waits on. The step is the one the rail kept when it last drew
    /// the Job, so the row is not built again here.
    pub(super) async fn approval_gate_waits(&self, id: &str, gate: PlanGate, end: rail_ends::End) -> bool {
        let hosting = {
            let store = self.store().lock().await;
            let gated = store.session_plan(id).ok().flatten().is_some_and(|plan| {
                plan.groups.iter().any(|group| group.gate.as_deref() == Some(gate.word()) && group.closed.is_none())
            });
            if gated {
                return true;
            }
            store.hosting(id).ok().flatten()
        };
        let Some(hosting) = hosting else { return false };
        let Some(workflow) = hosting.workflow.as_deref().filter(|workflow| self.workflow_on(&hosting.manifest_id, hosting.project.as_deref(), workflow)) else {
            return false;
        };
        let Some(steps) = self.job_rail_steps(id, &hosting.manifest_id, workflow).await else { return false };
        let Some(at) = self.store().lock().await.session_rail(id, &crate::rail::step_ids(&steps)).ok().and_then(|rail| rail.step) else {
            return false;
        };
        rail_ends::ends_of(&steps, at as usize).contains(&end)
    }

    /// The turn ended: every page under `<cwd>/.armada/documents/` is read again, so one a script or a
    /// shell changed is kept too. A page whose bytes the newest revision holds keeps nothing, and neither
    /// does one another Job left here as it kept it.
    pub(super) async fn pages_reread(self: &Arc<Self>, id: &str, cwd: &str) {
        let folder = Path::new(cwd).join(DOCUMENTS.trim_matches('/'));
        let Ok(pages) = tokio::task::spawn_blocking(move || pages_under(&folder)).await else { return };
        for page in pages {
            if !self.left_by_another(id, &page).await {
                self.page_written(id, page).await;
            }
        }
    }

    /// Whether the page at `path` is another Job's as that Job kept it: this Job holds no row for it and
    /// the file holds the bytes of another's newest revision. A slot handed over is not cleaned, and Jobs
    /// can share a checkout.
    async fn left_by_another(&self, id: &str, path: &str) -> bool {
        let name = page_name(path);
        let pages: Vec<KeptAttachment> = {
            let store = self.store().lock().await;
            if store.attachments_of(&Holder::session(id)).unwrap_or_default().iter().any(|one| one.kind == "artifact" && is_document(&one.detail) && page_name(&one.target) == name) {
                return false;
            }
            let Some(manifest) = store.hosting(id).ok().flatten().map(|hosting| hosting.manifest_id) else { return false };
            // A page is kept at Fleet's copy and names the file it was written at; an earlier build's, at that file.
            let targets = store.attachment_targets("artifact", &manifest).unwrap_or_default();
            targets
                .iter()
                .filter(|target| page_name(target) == name)
                .flat_map(|target| store.attachments_at("artifact", target, None).unwrap_or_default())
                .filter(|one| is_document(&one.detail) && (one.target == path || one.detail.get(DOCUMENT_WRITTEN_AT).is_some_and(|at| at == path)))
                .collect()
        };
        let newest: Vec<String> = pages.iter().filter_map(|one| revisions_named(&one.detail).pop()).collect();
        let path = path.to_string();
        tokio::task::spawn_blocking(move || {
            let Ok(now) = std::fs::read(&path) else { return false };
            newest.iter().any(|kept| std::fs::read(kept).is_ok_and(|bytes| bytes == now))
        })
        .await
        .unwrap_or(false)
    }

    /// Job `new` took the place of `old`: it holds each page from here, with the revisions copied to its
    /// own folder, the newest revision's number, the revisions he wrote and his answer or dismissal, so only
    /// a real edit asks again. `old` keeps its own, so a hand undone has lost nothing.
    pub(super) async fn pages_carried(&self, old: &str, new: &str) {
        let (held, dismissed, opened) = {
            let store = self.store().lock().await;
            (
                store.attachments_of(&Holder::session(old)).unwrap_or_default(),
                store.dismissed_waiting(old).unwrap_or_default(),
                store.artifact_seen(old).unwrap_or_default(),
            )
        };
        for page in held.into_iter().filter(|one| one.kind == "artifact" && is_document(&one.detail)) {
            let folder = self.documents_of(new).join(folder_of(&page.target));
            let kept = revisions_named(&page.detail);
            // The new Job's own copy of the page is its id there, as the old Job's is here.
            let home = self.documents_of(new).join(DOCUMENTS.trim_matches('/')).join(page_name(&page.target));
            let target = home.to_string_lossy().into_owned();
            let copy = tokio::task::spawn_blocking(move || {
                let copied = copied_to(&folder, &kept)?;
                if let Some(newest) = copied.last() {
                    std::fs::create_dir_all(home.parent().unwrap_or(&home))?;
                    std::fs::copy(newest, &home)?;
                }
                Ok::<_, io::Error>(copied)
            });
            let Ok(Ok(copied)) = copy.await else { continue };
            let Ok(encoded) = ipc::encode(&copied) else { continue };
            let mut detail = page.detail.clone();
            detail.insert("revisions".to_string(), encoded);
            let now = self.instant().as_str().to_string();
            let mut store = self.store().lock().await;
            let _ = store.carry_owner_revisions(old, new, &page.target, &target);
            // What he dismissed of it stays dismissed, and what he read of it is read still, so a page waits to be
            // read again only where it did.
            for prefix in [DOCUMENT_ITEM_PREFIX, READ_ITEM_PREFIX] {
                if dismissed.contains(&format!("{prefix}{}", page.target)) {
                    let _ = store.dismiss_waiting(new, &format!("{prefix}{target}"));
                }
            }
            if let Some(kept) = opened.iter().find(|one| one.artifact == page.target) {
                let _ = store.keep_artifact_seen(new, &target, kept.revision);
            }
            let carried = KeptAttachment { holder: Holder::session(new), target, detail, since: now.clone(), changed_at: now, ..page };
            let _ = store.attach(&carried, false);
        }
    }
}

/// The paths a document's `revisions` names, oldest first, and none where it names none.
pub(super) fn revisions_named(detail: &BTreeMap<String, String>) -> Vec<String> {
    detail.get("revisions").and_then(|kept| ipc::decode("a document's revisions", kept.as_bytes()).ok()).unwrap_or_default()
}

/// The revisions at `kept` copied into `folder` under their own names, as the paths they now have.
fn copied_to(folder: &Path, kept: &[String]) -> io::Result<Vec<String>> {
    std::fs::create_dir_all(folder)?;
    kept.iter()
        .map(|one| {
            let from = Path::new(one);
            let to = folder.join(from.file_name().unwrap_or_default());
            std::fs::copy(from, &to)?;
            Ok(to.to_string_lossy().into_owned())
        })
        .collect()
}

/// Every `.html` file under `folder`, its subfolders too, as `written_job_page` allows them. A
/// symlinked folder is not followed, so a link back up cannot loop.
fn pages_under(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(dir) = open.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let (Ok(kind), path) = (entry.file_type(), entry.path()) else { continue };
            if kind.is_dir() {
                open.push(path);
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("html")) {
                found.push(path.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

/// The one line that tells a Job which sketches of its page will not draw, or `None` where all do.
pub(crate) fn sketches_that_will_not_draw(html: &str, path: &str) -> Option<String> {
    let sketches = sketches_in(html);
    let total = sketches.len();
    let failing: Vec<String> = sketches
        .iter()
        .enumerate()
        .filter_map(|(at, sketch)| {
            let problems = ipc::page_scene_problems(&sketch.scene);
            if problems.is_empty() {
                return None;
            }
            let named = sketch.title.as_ref().map(|title| format!(" (\"{title}\")")).unwrap_or_default();
            Some(format!("sketch {} of {total}{named} will not draw: {}.", at + 1, problems.join("; ")))
        })
        .collect();
    (!failing.is_empty()).then(|| format!("In the page {path}, {}", failing.join(" ")))
}

/// The sentence that names each For link of a page Fleet did not keep, or `None` where it kept them all.
fn links_dropped(dropped: &[String]) -> Option<String> {
    (!dropped.is_empty()).then(|| format!("The armada-for links {} were not kept.", dropped.join("; ")))
}

/// The key of a document's `detail` that names the file in a checkout its newest revision was written at, which
/// his Edit writes back to. Absent on a page carried from another Job.
pub(crate) const DOCUMENT_WRITTEN_AT: &str = "written_at";

/// The folder a page's revisions are in: its path under `.armada/documents/`, as one name.
pub(super) fn folder_of(path: &str) -> String {
    one_name(page_name(path))
}

/// A page's file name: its path under `.armada/documents/`, the same in every checkout and in Fleet's copy.
pub(crate) fn page_name(path: &str) -> &str {
    let relative = DOCUMENTS.trim_start_matches('/');
    path.rsplit_once(DOCUMENTS).map(|(_, under)| under).or_else(|| path.strip_prefix(relative)).unwrap_or(path)
}

/// Whether `named` names a page by its path: in a checkout, at Fleet's copy, or from a checkout's root.
pub(crate) fn is_page_path(named: &str) -> bool {
    named.contains(DOCUMENTS) || named.starts_with(DOCUMENTS.trim_start_matches('/'))
}

/// A page as the Job names it: `.armada/documents/<name>`, which it writes in whatever checkout it is in.
pub(crate) fn shown_page(path: &str) -> String {
    format!("{}{}", DOCUMENTS.trim_start_matches('/'), page_name(path))
}

/// `text` as one folder name: anything but letters, digits, `-` and `.` becomes `_`.
pub(crate) fn one_name(text: &str) -> String {
    text.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '.') { c } else { '_' }).collect()
}

/// The revisions in `folder` with extension `ext`, oldest first: `r1.<ext>`, `r2.<ext>` and on.
pub(crate) fn revisions_in(folder: &Path, ext: &str) -> Vec<PathBuf> {
    let mut held: Vec<(u32, PathBuf)> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = name.strip_prefix('r')?.strip_suffix(ext)?.strip_suffix('.')?.parse().ok()?;
            Some((number, entry.path()))
        })
        .collect();
    held.sort();
    held.into_iter().map(|(_, path)| path).collect()
}

/// Keep `bytes` as the next revision in `folder` unless the newest holds exactly them. The revisions,
/// oldest first, where one was kept.
pub(crate) fn keep(folder: &Path, bytes: &[u8], ext: &str) -> io::Result<Option<Vec<PathBuf>>> {
    std::fs::create_dir_all(folder)?;
    let held = revisions_in(folder, ext);
    if let Some(newest) = held.last() {
        if std::fs::read(newest)? == bytes {
            return Ok(None);
        }
    }
    let next = folder.join(format!("r{}.{ext}", held.len() + 1));
    let staged = folder.join(format!("r{}.writing", held.len() + 1));
    std::fs::write(&staged, bytes)?;
    std::fs::rename(&staged, next)?;
    Ok(Some(revisions_in(folder, ext)))
}
