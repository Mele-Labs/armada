//! `mark_annotation_done`: a hosted session marks done a note Bridge sent it. The note may sit in
//! a checkout the session holds no slot for, such as the preview's, where the slot guard refuses a
//! write; Fleet's door is the way in, and only for a note sent to the session asking.
//! `docs/concepts/session.md`.
//!
//! **A note sent to a Job is an artifact of form `note` on it**, keyed `note:<id>`, its `status`
//! following the file's. A Job that closes after `all_done` marks its open notes done; one that
//! closes unfinished lets them go, so Bridge lists them under no Job. Each rewrite moves the
//! artifact, so its `session.changed` is Bridge's word that a note file changed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::annotations::{is_note_id, marked_done, note_read, unsent, NoteRead, NotMarked};
use ipc::{AnnotationMarked, MarkAnnotationDone, SessionNote};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;

const NOTE_REFUSED: &str = "fleet.annotation_refused";
const NOTE_UNWRITTEN: &str = "fleet.annotation_unwritten";

const ARTIFACT: &str = "artifact";
const NOTE_FORM: &str = "note";
const TARGET_PREFIX: &str = "note:";
/// The most of his words a note artifact's `title` carries.
const TITLE_CHARS: usize = 80;

/// A note's `status` on its artifact: the file's `open` or `done`, or `returned` where the Job
/// closed unfinished and the note went back to no Job.
const OPEN: &str = "open";
const DONE: &str = "done";
const RETURNED: &str = "returned";

/// What a note artifact shows of a note, read from what Bridge sent or from its file.
pub(super) struct NoteShown {
    pub(super) id: String,
    pub(super) text: String,
    pub(super) screen: Option<String>,
    pub(super) component: Option<String>,
    pub(super) tag: String,
    pub(super) address: Option<String>,
}

impl NoteShown {
    /// A pinned note Bridge's own layer took, which alone carries a note id.
    pub(super) fn pinned(note: &SessionNote) -> Option<NoteShown> {
        let id = note.note_id.as_deref().filter(|id| is_note_id(id))?;
        Some(NoteShown {
            id: id.to_string(),
            text: note.said.trim().to_string(),
            screen: note.place.screen.clone(),
            component: note.place.component.clone(),
            tag: note.place.element.tag.clone(),
            address: Some(note.address.clone()),
        })
    }

    fn read(note: &NoteRead) -> NoteShown {
        NoteShown {
            id: note.id.clone(),
            text: note.text.trim().to_string(),
            screen: note.screen.clone(),
            component: note.component.clone(),
            tag: note.element.tag.clone(),
            address: None,
        }
    }

    /// The artifact's detail, every value a string as the ledger keeps them. `file` and `picture` are
    /// absolute paths, where the note's file was found; the picture is never carried inline.
    fn detail(&self, file: Option<&Path>, status: &str, at: &str) -> BTreeMap<String, String> {
        let title: String = self.text.lines().next().unwrap_or_default().chars().take(TITLE_CHARS).collect();
        let picture = file
            .map(|file| file.with_file_name(format!("{}.png", self.id)))
            .filter(|picture| picture.is_file());
        [
            ("form", Some(NOTE_FORM.to_string())),
            ("by", Some(String::from("person"))),
            ("note", Some(self.id.clone())),
            ("title", Some(title)),
            ("text", Some(self.text.clone())),
            ("tag", Some(self.tag.clone())),
            ("component", self.component.clone()),
            ("screen", self.screen.clone()),
            ("address", self.address.clone()),
            ("file", file.map(|file| file.display().to_string())),
            ("picture", picture.map(|picture| picture.display().to_string())),
            ("status", Some(status.to_string())),
            ("sent", Some(at.to_string())),
        ]
        .into_iter()
        .filter_map(|(key, value)| Some((key.to_string(), value?)))
        .collect()
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
    /// Find the note under the served root, set its `status` to `done` and its `updatedAt` to now,
    /// and write it back through a rename. Refused unless its `sent.sessionId` is this session.
    pub(crate) async fn annotation_done(
        &self,
        caller: Option<api::Caller>,
        mark: MarkAnnotationDone,
    ) -> Result<AnnotationMarked, Refusal> {
        let id = self.waiting_session(caller, mark.session_id.as_ref())?;
        if !is_note_id(&mark.note_id) {
            return Err(self.hosted_refusal(
                NOTE_REFUSED,
                &format!("{:?} is not a note id, which is letters, digits and `-`", mark.note_id),
            ));
        }
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
        let root = PathBuf::from(served.root());
        let file = format!("{}.json", mark.note_id);
        let looked = adapters::annotation_places(&root);
        let Some(path) = looked.iter().map(|dir| dir.join(&file)).find(|path| path.is_file()) else {
            let named: Vec<String> = looked.iter().map(|dir| dir.display().to_string()).collect();
            return Err(self.hosted_refusal(
                NOTE_REFUSED,
                &format!("no note {file} in {}", named.join(", ")),
            ));
        };
        let bytes = std::fs::read(&path)
            .map_err(|why| self.hosted_fault(NOTE_UNWRITTEN, &format!("{}: {why}", path.display())))?;
        let now = self.now().as_str().to_string();
        let marked = marked_done(&bytes, &id, &now).map_err(|why| match why {
            NotMarked::NotSentHere => self.hosted_refusal(NOTE_REFUSED, "that note was not sent to this job"),
            NotMarked::Unreadable(why) => {
                self.hosted_refusal(NOTE_REFUSED, &format!("{} is not a note: {why}", path.display()))
            }
        })?;
        written(&path, &marked)
            .map_err(|why| self.hosted_fault(NOTE_UNWRITTEN, &format!("{}: {why}", path.display())))?;
        let read = note_read(&bytes).ok();
        if self.notes_moved(&id, &[(read, mark.note_id.clone(), path.clone())], DONE, &now).await {
            let _ = self.published_hosted(&id).await;
        }
        Ok(AnnotationMarked { path: path.display().to_string() })
    }

    /// Keep each note sent to `id` as an artifact on it. A note whose file is not found is still
    /// kept, from what was sent; nothing here refuses the send it follows.
    pub(super) async fn notes_kept(&self, id: &str, notes: Vec<NoteShown>) {
        if notes.is_empty() {
            return;
        }
        let root = self.notes_root(id).await;
        let at = self.now().as_str().to_string();
        let mut changed = false;
        {
            let mut store = self.store().lock().await;
            let Ok(Some(session)) = store.session(id) else {
                return;
            };
            for note in notes {
                let file = root.as_deref().and_then(|root| note_file(root, &note.id));
                let kept = KeptAttachment {
                    holder: Holder::session(id),
                    kind: ARTIFACT.to_string(),
                    manifest_id: session.manifest_id.clone().unwrap_or_default(),
                    target: format!("{TARGET_PREFIX}{}", note.id),
                    state: AttachmentState::Standing,
                    detail: note.detail(file.as_deref(), OPEN, &at),
                    since: at.clone(),
                    changed_at: at.clone(),
                };
                changed |= store.attach(&kept, false).unwrap_or(false);
            }
        }
        if changed {
            let _ = self.published_hosted(id).await;
        }
    }

    /// The window-scope notes a plain message carries, by what Bridge names in it, kept as
    /// [`Self::notes_kept`] keeps a pinned note: read from their files, which must exist.
    pub(super) async fn window_notes_kept(&self, id: &str, named: Vec<String>) {
        if named.is_empty() {
            return;
        }
        let Some(root) = self.notes_root(id).await else {
            return;
        };
        let shown = named
            .iter()
            .filter_map(|note| std::fs::read(note_file(&root, note)?).ok())
            .filter_map(|bytes| note_read(&bytes).ok())
            .map(|read| NoteShown::read(&read))
            .collect();
        self.notes_kept(id, shown).await;
    }

    /// A Job's close settles each open note sent to it: done where it had said it was done and
    /// nothing came after, let go otherwise.
    pub(super) async fn notes_settled(&self, id: &str) {
        let Some(root) = self.notes_root(id).await else {
            return;
        };
        let done = self.store().lock().await.done_since(id).ok().flatten().is_some();
        let at = self.now().as_str().to_string();
        let (session, stamp) = (id.to_string(), at.clone());
        let settled = tokio::task::spawn_blocking(move || settled_under(&root, &session, done, &stamp))
            .await
            .unwrap_or_default();
        let settled: Vec<_> = settled.into_iter().map(|(read, path)| (Some(read.clone()), read.id, path)).collect();
        if self.notes_moved(id, &settled, if done { DONE } else { RETURNED }, &at).await {
            let _ = self.published_hosted(id).await;
        }
    }

    /// Each rewritten note's artifact, its `status` moved and its state kept. A note sent before
    /// artifacts were kept, or one a message carried unnamed, gets one now, given back where the Job
    /// has ended. Whether any row changed.
    async fn notes_moved(&self, id: &str, notes: &[(Option<NoteRead>, String, PathBuf)], status: &str, at: &str) -> bool {
        if notes.is_empty() {
            return false;
        }
        let mut store = self.store().lock().await;
        let holder = Holder::session(id);
        let (Ok(Some(session)), Ok(held)) = (store.session(id), store.attachments_of(&holder)) else {
            return false;
        };
        let mut changed = false;
        for (read, note, path) in notes {
            let target = format!("{TARGET_PREFIX}{note}");
            let row = held.iter().find(|one| one.kind == ARTIFACT && one.target == target);
            changed |= match row {
                Some(row) => {
                    let mut detail = row.detail.clone();
                    detail.insert("status".to_string(), status.to_string());
                    store.follow(row, row.state, &detail, at).unwrap_or(false)
                }
                None => {
                    let Some(read) = read else { continue };
                    let ended = session.state == store::SessionState::Ended;
                    let kept = KeptAttachment {
                        holder: holder.clone(),
                        kind: ARTIFACT.to_string(),
                        manifest_id: session.manifest_id.clone().unwrap_or_default(),
                        target,
                        state: if ended { AttachmentState::GivenBack } else { AttachmentState::Standing },
                        detail: NoteShown::read(read).detail(Some(path), status, at),
                        since: at.to_string(),
                        changed_at: at.to_string(),
                    };
                    store.attach(&kept, false).unwrap_or(false)
                }
            };
        }
        changed
    }

    /// The root of the repository `id` is in, under which its notes are looked for.
    async fn notes_root(&self, id: &str) -> Option<PathBuf> {
        let manifest = self.store().lock().await.session(id).ok().flatten()?.manifest_id?;
        self.repositories().serving(&manifest).map(|served| PathBuf::from(served.root()))
    }
}

/// The note ids a plain message names the way Bridge's window-scope send does: its picture as
/// `annotation-<id>.png`, or its line `- Note file: .armada/annotations/<id>.json`.
pub(super) fn window_notes_named(sent: &ipc::SendSessionMessage) -> Vec<String> {
    let by_picture = sent
        .attachments
        .iter()
        .filter_map(|one| one.name.strip_prefix("annotation-")?.strip_suffix(".png"));
    let by_line = sent
        .text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- Note file: .armada/annotations/")?.strip_suffix(".json"));
    let mut named: Vec<String> = Vec::new();
    for id in by_picture.chain(by_line).filter(|id| is_note_id(id)) {
        if !named.iter().any(|one| one == id) {
            named.push(id.to_string());
        }
    }
    named
}

/// The note `id`'s file under `root`, in the first place that holds one.
fn note_file(root: &Path, id: &str) -> Option<PathBuf> {
    let file = format!("{id}.json");
    adapters::annotation_places(root).into_iter().map(|dir| dir.join(&file)).find(|path| path.is_file())
}

/// Each open note under `root` sent to `session`, rewritten: done where `done`, let go where not.
fn settled_under(root: &Path, session: &str, done: bool, at: &str) -> Vec<(NoteRead, PathBuf)> {
    let mut settled = Vec::new();
    for dir in adapters::annotation_places(root) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            // Most notes went to another Job or to none: only one naming this Job is parsed.
            if !bytes.windows(session.len()).any(|window| window == session.as_bytes()) {
                continue;
            }
            let Ok(read) = note_read(&bytes) else {
                continue;
            };
            if read.status != OPEN || read.sent.as_ref().map(|sent| sent.session_id.as_str()) != Some(session) {
                continue;
            }
            let rewritten = if done { marked_done(&bytes, session, at) } else { unsent(&bytes, session, at) };
            match rewritten.map(|bytes| written(&path, &bytes)) {
                Ok(Ok(())) => settled.push((read, path)),
                Ok(Err(why)) => eprintln!("job {session}: note {} was not rewritten on close: {why}", path.display()),
                Err(_) => {}
            }
        }
    }
    settled
}

/// `bytes` to `path` through a partial beside it, so Bridge never reads half a note. The partial
/// does not end in `.json`, so Bridge's listing never reads one a crash leaves.
fn written(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    let partial = PathBuf::from(format!("{}.{}-{nanos}.partial", path.display(), std::process::id()));
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&partial);
    })
}
