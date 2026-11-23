//! `show_prototype`: a scene the Job wrote under `scenes/` in its scratch folder, shown as a part and redrawn when it
//! is saved. **Fleet reads the files and native parses them**: this reads the scene's text, and from it only the
//! `src` of each `<fixture>` element, so it knows which files to read beside it.
//! `crates/ipc/operations/show_prototype.toml`.
//!
//! The part keeps the text it read, so the revision he approved outlives the scratch folder, which goes with the Job.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{CheckedPart, PartContent, PartState, PrototypePart, SceneFixture, SessionId, SessionRecord, ShowPrototype, MOST_FIXTURE_BYTES, MOST_SCENE_BYTES, SCENES_DIR};

use super::artifact_parts::{stored, PART_REFUSED};
use crate::daemon::Fleet;

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
        return Err(format!("{shown} weighs {} bytes and at most {most} are read. Cut it down", meta.len()));
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
    let mut weight = 0usize;
    for src in fixture_sources(&markup) {
        if let Err(why) = ipc::file_name_checked(&src) {
            problems.push(format!("fixture `src` {why}. A fixture is a plain file name beside the scene"));
            continue;
        }
        match text_of(&folder, &src, MOST_FIXTURE_BYTES) {
            Ok(text) => {
                weight += text.len();
                fixtures.push(SceneFixture { name: src, text });
            }
            Err(why) => problems.push(why),
        }
    }
    if weight > MOST_FIXTURE_BYTES {
        problems.push(format!("the fixtures weigh {weight} bytes together and at most {MOST_FIXTURE_BYTES} are read. Cut them down"));
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
    /// `show_prototype`: the scene read now, put as a part that asks, under the scene's file name.
    pub(crate) async fn prototype_shown(&self, caller: Option<api::Caller>, show: ShowPrototype) -> Result<SessionRecord, Refusal> {
        let checked = show.checked().map_err(|why| self.hosted_refusal(PART_REFUSED, &why))?;
        let (id, harness) = self.part_job(caller, show.session_id.as_ref()).await?;
        let prototype = prototype_read(&self.scratch_of(&id), &checked.scene, checked.state)
            .map_err(|why| self.hosted_refusal(PART_REFUSED, &format!("the prototype was not shown: {why}")))?;
        let part = CheckedPart { title: checked.title, id: checked.scene, content: PartContent::Prototype(prototype), asks: true };
        self.part_placed(&id, &harness, &part, checked.said).await
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
        let Ok((_, harness)) = self.part_job(None, Some(&SessionId::carried(id))).await else { return };
        let scratch = self.scratch_of(id);
        for row in held.iter().filter(|one| super::artifact_parts::is_sketch(one)) {
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
                let put = CheckedPart { title: String::new(), id: part.id.clone(), content: PartContent::Prototype(now), asks: part.asks };
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
