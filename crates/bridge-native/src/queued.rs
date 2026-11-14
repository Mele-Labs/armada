//! The ideas a person saved to start later (Dispatch's Queue for later), kept on this machine, as Electron's
//! `queued-ideas.ts` keeps them: the same fields, oldest first, in a file beside the list's order, so an
//! idea Classic saved can be carried across as it is. **Not Fleet's**: an idea is how this person works,
//! and Fleet's preference set is closed. The list draws each as a row under its repository's group.

use std::path::PathBuf;

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

/// One saved idea: the words, the repository's root and name as the picker gives it, and the model and
/// effort picked when it was saved (`QueuedIdea`, `draft/sessions.ts`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedIdea {
    pub id: String,
    pub text: String,
    pub root: String,
    pub repository: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// The project of the row it was saved from, by Fleet's id, which its Job is started for. None is no project,
    /// and an idea saved before the picker listed projects has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

impl QueuedIdea {
    /// What its row says: the first line of its words.
    pub fn title(&self) -> &str {
        self.text.lines().find(|line| !line.trim().is_empty()).map_or("", str::trim)
    }
}

/// The ideas held, and the file they are kept in. A mock list keeps none, so a walk never writes his.
#[derive(Default)]
pub struct Queued {
    pub ideas: Vec<QueuedIdea>,
    file: Option<PathBuf>,
    /// The ideas whose start is out, so a second press is not a second Job.
    pub starting: Vec<String>,
}

impl Global for Queued {}

/// The file beside the list's own (`jobs::deck_file`).
fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-queued-ideas.json"))
}

/// What the file holds, an idea at a time: one that is not whole is left out rather than failing the rest.
fn read(path: &PathBuf) -> Vec<QueuedIdea> {
    #[derive(Deserialize)]
    struct Kept {
        id: Option<String>,
        text: Option<String>,
        root: Option<String>,
        repository: Option<String>,
        model: Option<String>,
        effort: Option<String>,
        project: Option<String>,
    }
    let Ok(bytes) = std::fs::read(path) else { return Vec::new() };
    let some = |value: Option<String>| value.filter(|value| !value.is_empty());
    ipc::decode::<Vec<Kept>>("queued ideas", &bytes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|kept| {
            Some(QueuedIdea {
                id: some(kept.id)?,
                text: some(kept.text)?,
                root: some(kept.root)?,
                repository: some(kept.repository)?,
                model: some(kept.model),
                effort: some(kept.effort),
                project: some(kept.project),
            })
        })
        .collect()
}

/// The list on a live Fleet keeps its ideas in the file; a mock list holds them only while it runs.
pub fn begin(kept: bool, cx: &mut App) {
    let file = kept.then(file).flatten();
    let ideas = file.as_ref().map(read).unwrap_or_default();
    cx.set_global(Queued { ideas, file, starting: Vec::new() });
}

pub fn ideas(cx: &App) -> Vec<QueuedIdea> {
    cx.try_global::<Queued>().map(|queued| queued.ideas.clone()).unwrap_or_default()
}

fn change(cx: &mut App, change: impl FnOnce(&mut Queued)) {
    if cx.try_global::<Queued>().is_none() {
        begin(false, cx);
    }
    let queued = cx.global_mut::<Queued>();
    change(queued);
    if let Some(path) = &queued.file {
        let written = ipc::encode(&queued.ideas).map_err(|why| format!("{why:?}")).and_then(|bytes| std::fs::write(path, bytes).map_err(|why| why.to_string()));
        if let Err(why) = written {
            crate::log::Note::warn("the queued ideas were not kept").field("why", why).emit();
        }
    }
}

/// Save an idea at the end of the queue.
pub fn add(idea: QueuedIdea, cx: &mut App) {
    change(cx, |queued| queued.ideas.push(idea));
}

/// Take an idea off the queue: removed, or started.
pub fn remove(id: &str, cx: &mut App) {
    change(cx, |queued| {
        queued.ideas.retain(|idea| idea.id != id);
        queued.starting.retain(|one| one != id);
    });
}

/// Mark an idea's start as out, or back; false where it already was, so the press is not sent twice.
pub fn set_starting(id: &str, on: bool, cx: &mut App) -> bool {
    let mut was = false;
    change(cx, |queued| {
        was = queued.starting.iter().any(|one| one == id);
        queued.starting.retain(|one| one != id);
        if on {
            queued.starting.push(id.to_string());
        }
    });
    was != on
}

/// A key for a new idea, unique across launches, in the shape Electron's `newIdeaId` gives.
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static MADE: AtomicU64 = AtomicU64::new(0);
    let made = MADE.fetch_add(1, Ordering::Relaxed) + 1;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64);
    let salt = std::process::id() as u64 ^ now.rotate_left(17);
    format!("q-{}-{}-{}", radix36(now), radix36(made), &radix36(salt)[..4.min(radix36(salt).len())])
}

fn radix36(mut value: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[(value % 36) as usize]);
        value /= 36;
        if value == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_idea_keeps_its_project_and_one_kept_before_reads_with_none() {
        let path = std::env::temp_dir().join(format!("bridge-native-queued-{}.json", std::process::id()));
        let kept = QueuedIdea { id: "a".into(), text: "Fix the login".into(), root: "/r/sample-api".into(), repository: "sample-api".into(), model: None, effort: None, project: Some("storefront".into()) };
        std::fs::write(&path, ipc::encode(&vec![kept.clone()]).expect("encodes")).expect("writes");
        assert_eq!(read(&path), [kept], "the project comes back as it was saved");
        std::fs::write(&path, br#"[{"id":"b","text":"Old idea","root":"/r/armada","repository":"armada"}]"#).expect("writes");
        assert_eq!(read(&path).first().map(|one| one.project.clone()), Some(None), "an idea from before has no project");
        let _ = std::fs::remove_file(&path);
    }
}
