//! `show_prototype`: a scene the Job wrote under `scenes/` in its scratch folder, shown as a part and redrawn when it
//! is saved. **Fleet reads the files and native parses them**: this reads the scene's text, and from it only the
//! `src` of each `<fixture>` element, so it knows which files to read beside it.
//! `crates/ipc/operations/show_prototype.toml`.
//!
//! The part keeps the text it read, so the revision he approved outlives the scratch folder, which goes with the Job.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    ArtifactFor, AttachmentReport, CheckedPart, PartContent, PartState, PrototypePart, SceneFixture, SessionFact, SessionId, SessionRecord, SessionReport, ShowPrototype,
    MOST_PROTOTYPE_BYTES, MOST_SCENE_BYTES, SCENES_DIR,
};

use super::artifact_parts::{is_sketch, stored, PART_REFUSED, PART_UNNAMED};
use super::sketching::TARGET_PREFIX;
use crate::daemon::Fleet;

/// How long the scratch folders stay quiet before a save is read again, so a write that lands as several events, or
/// a scene and its fixtures saved one after the other, is one revision.
const QUIET: Duration = Duration::from_millis(150);

/// The `src` of each `<fixture>` element in `markup`, in order and once each. A light scan: an element it cannot
/// find a quoted `src` in names nothing, and native decides what the scene means.
pub(crate) fn fixture_sources(markup: &str) -> Vec<String> {
    let lower = markup.to_ascii_lowercase();
    let mut found: Vec<String> = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find("<fixture") {
        let start = from + at + "<fixture".len();
        from = start;
        // `<fixtures>` is another element.
        if !lower[start..].starts_with(|next: char| next.is_ascii_whitespace() || next == '/' || next == '>') {
            continue;
        }
        let end = lower[start..].find('>').map_or(lower.len(), |end| start + end);
        let tag = &markup[start..end];
        let tag_lower = &lower[start..end];
        let mut search = 0;
        while let Some(found_at) = tag_lower[search..].find("src") {
            let name_at = search + found_at;
            search = name_at + "src".len();
            let before_ok = tag_lower[..name_at].ends_with(|prev: char| prev.is_ascii_whitespace());
            let rest = tag[search..].trim_start();
            let Some(rest) = rest.strip_prefix('=') else { continue };
            let rest = rest.trim_start();
            let Some(quote) = rest.chars().next().filter(|quote| *quote == '"' || *quote == '\'') else { continue };
            if !before_ok {
                continue;
            }
            if let Some(value) = rest[1..].split(quote).next() {
                let value = value.trim().to_string();
                if !value.is_empty() && !found.contains(&value) {
                    found.push(value);
                }
            }
            break;
        }
    }
    found
}

/// The text of `name` in `folder`, or what is wrong with it. A symlink is not followed: a scene names a file in its
/// own folder, and a link could name any file Fleet may read.
fn text_of(folder: &Path, name: &str, most: usize) -> Result<String, String> {
    let path = folder.join(name);
    let shown = format!("`{SCENES_DIR}/{name}`");
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Err(format!("{shown} is not there. Write it under $ARMADA_SCRATCH/{SCENES_DIR}/ first")),
        Err(why) => return Err(format!("{shown} could not be read: {why}")),
    };
    if meta.file_type().is_symlink() {
        return Err(format!("{shown} is a symlink, which Fleet does not read. Write the file itself"));
    }
    if !meta.is_file() {
        return Err(format!("{shown} is not a file"));
    }
    if meta.len() as usize > most {
        return Err(format!("{shown} weighs {} bytes and at most {most} are read here, a scene and its fixtures weighing at most {MOST_PROTOTYPE_BYTES} together. Cut it down", meta.len()));
    }
    let bytes = std::fs::read(&path).map_err(|why| format!("{shown} could not be read: {why}"))?;
    String::from_utf8(bytes).map_err(|_| format!("{shown} is not UTF-8 text"))
}

/// The scene `scene` in `scratch`'s `scenes/` folder and each fixture it names beside it, as a part reads at this
/// moment, or every reason it cannot be read in one sentence an agent can act on.
pub(crate) fn prototype_read(scratch: &Path, scene: &str, scene_state: Option<String>) -> Result<PrototypePart, String> {
    let folder = scratch.join(SCENES_DIR);
    let markup = text_of(&folder, scene, MOST_SCENE_BYTES)?;
    let mut problems = Vec::new();
    let mut fixtures = Vec::new();
    // The scene and its fixtures weigh at most one cap together, so each fixture may use what the rest left.
    let mut left = MOST_PROTOTYPE_BYTES.saturating_sub(markup.len());
    for src in fixture_sources(&markup) {
        if let Err(why) = ipc::file_name_checked(&src) {
            problems.push(format!("fixture `src` {why}. A fixture is a plain file name beside the scene"));
            continue;
        }
        match text_of(&folder, &src, left) {
            Ok(text) => {
                left -= text.len();
                fixtures.push(SceneFixture { name: src, text });
            }
            Err(why) => problems.push(why),
        }
    }
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(PrototypePart { scene: scene.to_string(), markup, fixtures, scene_state, trouble: None })
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
    /// `show_prototype`: the scene read now, put as a part that asks nothing, under the scene's file name. A prototype
    /// shows flow and never the look, so there is nothing to approve: it goes to his thread and Artifacts, and his note
    /// on it reaches the Job as a message.
    pub(crate) async fn prototype_shown(self: Arc<Self>, caller: Option<api::Caller>, show: ShowPrototype) -> Result<SessionRecord, Refusal> {
        let checked = show.checked().map_err(|why| self.hosted_refusal(PART_REFUSED, &why))?;
        let (id, harness) = self.prototype_job(caller, show.session_id.as_ref()).await?;
        // Watched before it is read, so a save between the two is heard.
        self.scenes_watched();
        let prototype = prototype_read(&self.scratch_of(&id), &checked.scene, checked.state)
            .map_err(|why| self.hosted_refusal(PART_REFUSED, &format!("the prototype was not shown: {why}")))?;
        // The rest at their defaults, so a field another branch adds to `CheckedPart` needs nothing here.
        #[allow(clippy::needless_update)]
        let part = CheckedPart { title: checked.title, id: checked.scene, content: PartContent::Prototype(prototype), asks: false, ..CheckedPart::default() };
        self.prototype_placed(&id, &harness, &part, checked.said).await
    }

    /// The job the call places and its harness. Mirrors the lookup in `part_added`.
    async fn prototype_job(&self, caller: Option<api::Caller>, named: Option<&SessionId>) -> Result<(String, String), Refusal> {
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(PART_UNNAMED, "a part needs a job to go on, and this call places none"))?;
        let harness = {
            let store = self.store().lock().await;
            store.session(&id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.hosted_refusal(PART_UNNAMED, "a job Fleet knows"))?.harness
        };
        Ok((id, harness))
    }

    /// `part` put where it goes: the artifact that holds the scene already, else a new one. Mirrors the placement in
    /// `part_added`.
    async fn prototype_placed(&self, id: &str, harness: &str, part: &CheckedPart, said: Option<ArtifactFor>) -> Result<SessionRecord, Refusal> {
        let held = self.held_rows(id).await?;
        let earlier = held.iter().filter(|one| is_sketch(one)).find(|one| stored(&one.detail).iter().any(|held| held.id == part.id)).map(|one| one.target.clone());
        let target = match earlier {
            Some(target) => target,
            None => self.prototype_container(id, harness, &part.title, &held).await?,
        };
        self.part_put(id, harness, &target, part, said).await
    }

    /// A scene-less artifact titled `title`, with a target unique among the Job's. Mirrors `container_made`.
    async fn prototype_container(&self, id: &str, harness: &str, title: &str, held: &[ipc::Attachment]) -> Result<String, Refusal> {
        let mut target = format!("{TARGET_PREFIX}{title}");
        let mut again = 1;
        while held.iter().any(|one| one.target == target) {
            again += 1;
            target = format!("{TARGET_PREFIX}{title} ({again})");
        }
        let detail = [("form", "sketch"), ("by", "agent"), ("title", title)].into_iter().map(|(key, word)| (key.to_string(), word.to_string())).collect();
        self.report_session(SessionReport {
            harness: harness.to_string(),
            session_id: SessionId::carried(id),
            fact: SessionFact::Attached { attachment: AttachmentReport { kind: String::from("artifact"), target: target.clone(), detail } },
        })
        .await?;
        Ok(target)
    }

    /// **Every Job's scratch folder watched for a write under its `scenes/`**, from the first prototype shown. A save
    /// is a hint: after a quiet moment the files are read again by [`Self::scenes_touched`], so a scene redraws
    /// whatever mod the Job runs, and one the mod never reported (a path outside the Job's folder) is heard. The
    /// task holds Fleet weakly, so it ends with it.
    fn scenes_watched(self: &Arc<Self>) {
        let mut held = self.hosts().scene_watch.lock().unwrap_or_else(PoisonError::into_inner);
        if held.is_some() {
            return;
        }
        let root = crate::scratch::scratch_under(Path::new(&self.host().attachments_dir), "");
        let _ = std::fs::create_dir_all(&root);
        let (send, mut heard) = tokio::sync::mpsc::unbounded_channel::<Vec<PathBuf>>();
        *held = adapters::FolderWatch::start(&root, move |paths| {
            // A scratch folder holds a browser's profile and a test run's output: only what is under a `scenes/` goes on.
            let scenes: Vec<PathBuf> = paths.into_iter().filter(|path| path.components().any(|part| part.as_os_str() == SCENES_DIR)).collect();
            if !scenes.is_empty() {
                let _ = send.send(scenes);
            }
        });
        if held.is_none() {
            return;
        }
        let canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
        let fleet = Arc::downgrade(self);
        tokio::spawn(async move {
            while let Some(mut paths) = heard.recv().await {
                while let Ok(Some(more)) = tokio::time::timeout(QUIET, heard.recv()).await {
                    paths.extend(more);
                }
                let Some(fleet) = fleet.upgrade() else { return };
                fleet.scenes_heard(&root, &canonical, paths).await;
            }
        });
    }

    /// What the watch heard, each path named as `<scratch>/<job>/scenes/<file>` as Fleet names it (the file system
    /// says the real path, which is not the one a folder was named by where a link is in it), and told to the Job
    /// it is under.
    async fn scenes_heard(&self, root: &Path, canonical: &Path, paths: Vec<PathBuf>) {
        let mut by_job: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for path in paths {
            let Ok(inside) = path.strip_prefix(canonical).or_else(|_| path.strip_prefix(root)) else { continue };
            let parts: Vec<&str> = inside.iter().filter_map(|part| part.to_str()).collect();
            if let [job, scenes, _file] = parts.as_slice() {
                if *scenes == SCENES_DIR {
                    by_job.entry((*job).to_string()).or_default().push(root.join(inside).to_string_lossy().into_owned());
                }
            }
        }
        for (job, paths) in by_job {
            self.scenes_touched(&job, &paths).await;
        }
    }

    /// **Redraw on save.** `paths` are what the Job wrote; each one under its scratch `scenes/` folder that is the
    /// scene of a prototype it showed, or a fixture of one, puts that part again as its next revision, with the state
    /// it had. A part he approved or withdrew is left as he left it, and a save that reads as the part already does
    /// puts nothing. A file Fleet cannot read now (half written, or gone) is the part's `trouble` and its last good
    /// text stays. Best effort, as a ledger row is.
    pub(crate) fn scenes_touched<'a>(&'a self, id: &'a str, paths: &'a [String]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(self.scenes_redrawn(id, paths))
    }

    async fn scenes_redrawn(&self, id: &str, paths: &[String]) {
        let scenes = crate::scratch::scratch_under(Path::new(&self.host().attachments_dir), id).join(SCENES_DIR);
        let names: Vec<String> = paths
            .iter()
            .filter_map(|path| Path::new(path).strip_prefix(&scenes).ok())
            .filter_map(|rest| rest.to_str().filter(|rest| !rest.contains('/')))
            .map(str::to_string)
            .collect();
        if names.is_empty() {
            return;
        }
        let Ok(held) = self.held_rows(id).await else { return };
        let Ok((_, harness)) = self.prototype_job(None, Some(&SessionId::carried(id))).await else { return };
        let scratch = self.scratch_of(id);
        for row in held.iter().filter(|one| is_sketch(one)) {
            for part in stored(&row.detail) {
                let PartContent::Prototype(was) = &part.content else { continue };
                if !matches!(part.state, PartState::Open | PartState::SentBack) {
                    continue;
                }
                if !names.iter().any(|name| *name == was.scene || was.fixtures.iter().any(|fixture| fixture.name == *name)) {
                    continue;
                }
                let read = prototype_read(&scratch, &was.scene, was.scene_state.clone());
                let read_whole = read.is_ok();
                let now = read.unwrap_or_else(|why| PrototypePart { trouble: Some(why), ..was.clone() });
                if now == *was {
                    continue;
                }
                // The rest at their defaults, so a field another branch adds to `CheckedPart` needs nothing here.
                #[allow(clippy::needless_update)]
                let put = CheckedPart { title: String::new(), id: part.id.clone(), content: PartContent::Prototype(now), asks: part.asks, ..CheckedPart::default() };
                // The record is published whole and an artifact's parts weigh at most so much: a save that grew past it
                // keeps the last text and says so.
                if self.part_put(id, &harness, &row.target, &put, None).await.is_err() && read_whole {
                    let trouble = String::from("the saved scene and its fixtures are more than an artifact keeps. Cut them down");
                    let held = CheckedPart { content: PartContent::Prototype(PrototypePart { trouble: Some(trouble), ..was.clone() }), ..put };
                    let _ = self.part_put(id, &harness, &row.target, &held, None).await;
                }
            }
        }
    }
}
