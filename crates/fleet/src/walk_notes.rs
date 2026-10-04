//! What a person pointed at while walking a Job's served mock, kept on the Job
//! and carried to the next Drone. Protocol 23.16.
//!
//! **On the Job, never in the worktree.** A Prototype stops at Build with its
//! mock served (`crate::walking`) and its worktree is throwaway, so each note
//! is a row beside the Job's record and each frame a file under
//! `<machine>/walks/<job_id>/`. Reclaiming the worktree takes neither.
//!
//! **A person's act, and nothing moves.** Capturing or removing a note leaves
//! the Job where it stands. What delivers the notes is `request_changes`
//! carrying `with_walk_notes`: every unsent note is appended to the note it
//! hands the Drone, and marked sent once that act has landed — so a refused
//! send leaves every note unsent, to go with the next one.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::JobId;
use ipc::{CaptureElement, CaptureWalkNote, RemoveWalkNote, WalkNote, WalkNotes, WireError};
use store::{KeptWalkNote, WalkNoteRemoved, WalkServed};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::studios::MOST_A_FRAME_MAY_WEIGH;

/// A walk note whose `said` is blank. A 422: a point with nothing said about
/// it tells a Drone nothing.
const WALK_NOTE_BLANK: &str = "fleet.walk_note_blank";
/// A walk note asked of a Job that has ended. A 409: nothing will be sent back.
const WALK_NOTES_ENDED: &str = "fleet.walk_notes_job_ended";
/// A removal naming no note on this Job. A 422.
const NO_SUCH_WALK_NOTE: &str = "fleet.no_such_walk_note";
/// A removal of a note a Drone was already handed. A 409.
const WALK_NOTE_SENT: &str = "fleet.walk_note_sent";
/// A walk note's frame over [`MOST_A_FRAME_MAY_WEIGH`]. A 422.
const WALK_FRAME_TOO_LARGE: &str = "fleet.walk_frame_too_large";
/// A walk note's staged frame Fleet could not read or keep. A 422.
const WALK_FRAME_UNREADABLE: &str = "fleet.walk_frame_unreadable";

/// The longest a pointed-at element's name runs before it is cut.
const NAME_AT_MOST: usize = 60;

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
    /// `capture_walk_note`, answered with every note on the Job.
    pub(crate) async fn capturing_walk_note(
        &self,
        job_id: ipc::JobId,
        capture: CaptureWalkNote,
    ) -> Result<WalkNotes, Refusal> {
        if capture.said.trim().is_empty() {
            return Err(self.walk_refusal(
                Refusal::Unacceptable,
                WALK_NOTE_BLANK,
                "a walk note's `said` cannot be blank".into(),
                &job_id,
            ));
        }
        let id = job_id.to_domain();
        self.walk_notes_open(&id).await?;
        let note_id = self.mint().ulid().as_str().to_string();
        // The Job is held before the frame is written, so a capture onto a
        // Job that is not there or has ended leaves no file behind.
        let frame = match capture.frame {
            None => None,
            Some(staged) => Some(self.walk_frame_kept(&job_id, &note_id, staged)?),
        };
        let pointed = capture.capture;
        let kept = KeptWalkNote {
            note_id,
            said: capture.said,
            at: self.now(),
            element: pointed_at(&pointed.element),
            selector: pointed.selector,
            location: pointed.location,
            served: pointed.served.map(|served| WalkServed {
                run: served.run,
                name: served.name,
                address: served.address,
            }),
            frame,
            sent: false,
        };
        let written = self.store().lock().await.keep_walk_note(&id, &kept);
        if let Err(why) = written {
            if let Some(frame) = &kept.frame {
                let _ = std::fs::remove_file(frame);
            }
            return Err(self.refusal(Adrift::Writing(why)));
        }
        self.walk_notes_answered(&id).await
    }

    /// `remove_walk_note`, answered with every note left on the Job.
    pub(crate) async fn removing_walk_note(
        &self,
        job_id: ipc::JobId,
        remove: RemoveWalkNote,
    ) -> Result<WalkNotes, Refusal> {
        let id = job_id.to_domain();
        self.walk_notes_open(&id).await?;
        let removed = self
            .store()
            .lock()
            .await
            .remove_walk_note(&id, &remove.id)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        match removed {
            WalkNoteRemoved::Removed { frame } => {
                // The row is the record; a frame left behind is disk, not a lie.
                if let Some(frame) = frame {
                    let _ = std::fs::remove_file(frame);
                }
            }
            WalkNoteRemoved::NoSuchNote => {
                return Err(self.walk_refusal(
                    Refusal::Unacceptable,
                    NO_SUCH_WALK_NOTE,
                    format!("this Job has no walk note `{}`", remove.id),
                    &job_id,
                ))
            }
            WalkNoteRemoved::AlreadySent => {
                return Err(self.walk_refusal(
                    Refusal::IllegalMove,
                    WALK_NOTE_SENT,
                    format!(
                        "walk note `{}` was already handed to a Drone, so it is part of what \
                         the work was done against and stays",
                        remove.id
                    ),
                    &job_id,
                ))
            }
        }
        self.walk_notes_answered(&id).await
    }

    /// Every walk note on a Job, oldest first, as `get_job` carries them.
    pub(crate) async fn walk_notes_of(&self, job_id: &JobId) -> Result<Vec<WalkNote>, Adrift> {
        let kept = self
            .store()
            .lock()
            .await
            .walk_notes(job_id)
            .map_err(Adrift::Reading)?;
        Ok(kept.into_iter().map(on_the_wire).collect())
    }

    /// The note `request_changes` delivers, and the walk notes it carries.
    ///
    /// Where `with_walk_notes` is false this is the person's note alone and
    /// nothing is carried. Where it is true every unsent note is appended,
    /// formatted for a Drone; a blank `note` then leaves the notes as the whole
    /// of it. **Whether the result is blank is the caller's to refuse**, which
    /// is what keeps a blank note with nothing unsent refused exactly as before.
    pub(crate) async fn with_walk_notes(
        &self,
        job_id: &JobId,
        note: &ipc::ChangesRequested,
    ) -> Result<(String, Vec<String>), Adrift> {
        if !note.with_walk_notes {
            return Ok((note.note.clone(), Vec::new()));
        }
        let unsent: Vec<KeptWalkNote> = self
            .store()
            .lock()
            .await
            .walk_notes(job_id)
            .map_err(Adrift::Reading)?
            .into_iter()
            .filter(|kept| !kept.sent)
            .collect();
        let carried = unsent.iter().map(|kept| kept.note_id.clone()).collect();
        Ok((for_a_drone(&note.note, &unsent), carried))
    }

    /// Mark the notes a landed `request_changes` carried as sent.
    pub(crate) async fn walk_notes_sent(
        &self,
        job_id: &JobId,
        carried: &[String],
    ) -> Result<(), Adrift> {
        if carried.is_empty() {
            return Ok(());
        }
        let at = self.now();
        self.store()
            .lock()
            .await
            .mark_walk_notes_sent(job_id, carried, &at)
            .map_err(Adrift::Writing)
    }

    /// Where one Job's walk frames are kept: under the machine directory,
    /// never under the worktree.
    pub(crate) fn walk_frames(&self, job_id: &JobId) -> PathBuf {
        PathBuf::from(&self.host().walk_frames_dir).join(job_id.as_str())
    }

    /// The Job exists and has not ended.
    async fn walk_notes_open(&self, job_id: &JobId) -> Result<(), Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        if job.status().is_terminal() {
            return Err(self.walk_refusal(
                Refusal::IllegalMove,
                WALK_NOTES_ENDED,
                format!(
                    "this Job is {} and nothing will be sent back, so it takes no walk notes",
                    job.status().as_wire()
                ),
                &ipc::JobId::from(job_id),
            ));
        }
        Ok(())
    }

    async fn walk_notes_answered(&self, job_id: &JobId) -> Result<WalkNotes, Refusal> {
        let notes = self
            .walk_notes_of(job_id)
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(WalkNotes { notes })
    }

    /// Copy a staged PNG into the Job's own keeping, named for its note, and
    /// answer with the kept file's absolute path. **Refused, not dropped**, for
    /// `capture_studio_note`'s reason.
    fn walk_frame_kept(
        &self,
        job_id: &ipc::JobId,
        note_id: &str,
        staged: ipc::StagedFrame,
    ) -> Result<String, Refusal> {
        let unreadable = |cause: std::io::Error| {
            self.walk_refusal(
                Refusal::Unacceptable,
                WALK_FRAME_UNREADABLE,
                format!(
                    "the frame at `{}` was not kept: {cause}",
                    staged.staged_path
                ),
                job_id,
            )
        };
        let byte_size = std::fs::metadata(&staged.staged_path)
            .map_err(unreadable)?
            .len();
        if byte_size > MOST_A_FRAME_MAY_WEIGH {
            return Err(self.walk_refusal(
                Refusal::Unacceptable,
                WALK_FRAME_TOO_LARGE,
                format!(
                    "a frame weighs at most {MOST_A_FRAME_MAY_WEIGH} bytes and this one weighs \
                     {byte_size}"
                ),
                job_id,
            ));
        }
        let dir = self.walk_frames(&job_id.to_domain());
        std::fs::create_dir_all(&dir).map_err(unreadable)?;
        let kept = dir.join(format!("{note_id}.png"));
        std::fs::copy(&staged.staged_path, &kept).map_err(unreadable)?;
        Ok(kept.to_string_lossy().to_string())
    }

    fn walk_refusal(
        &self,
        as_: fn(WireError) -> Refusal,
        code: &str,
        said: String,
        job_id: &ipc::JobId,
    ) -> Refusal {
        as_(WireError::raised(code, said, self.run_id()).about_job(job_id.clone()))
    }
}

fn on_the_wire(kept: KeptWalkNote) -> WalkNote {
    WalkNote {
        id: kept.note_id,
        said: kept.said,
        at: ipc::Instant::from(&kept.at),
        element: kept.element,
        selector: kept.selector,
        location: kept.location,
        served: kept.served.map(|served| ipc::CaptureServed {
            run: served.run,
            name: served.name,
            address: served.address,
        }),
        frame: kept.frame,
        sent: kept.sent,
    }
}

/// What was pointed at, in one line a person reads: its role and its name —
/// `button “Save”`, `link “Pricing”`, `heading “Settings”`.
///
/// The name is the accessible label where the capture has one, else the
/// visible text, whitespace collapsed and cut at [`NAME_AT_MOST`] characters.
/// An element with neither is named by its role alone.
pub(crate) fn pointed_at(element: &CaptureElement) -> String {
    let tag = element.tag.trim().to_ascii_lowercase();
    let role = match tag.as_str() {
        "a" => "link",
        "button" => "button",
        "input" | "textarea" => "field",
        "select" => "dropdown",
        "img" | "svg" | "picture" => "image",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "p" => "paragraph",
        "li" => "list item",
        "ul" | "ol" => "list",
        "label" => "label",
        "nav" => "navigation",
        "table" => "table",
        "form" => "form",
        "dialog" => "dialog",
        "" => "element",
        other => other,
    };
    let name = element
        .label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
        .unwrap_or(&element.text);
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return role.to_string();
    }
    let name = match name.char_indices().nth(NAME_AT_MOST) {
        Some((cut, _)) => format!("{}…", name[..cut].trim_end()),
        None => name,
    };
    format!("{role} “{name}”")
}

/// The person's note with every unsent walk note appended, for a Drone.
fn for_a_drone(note: &str, unsent: &[KeptWalkNote]) -> String {
    let note = note.trim();
    if unsent.is_empty() {
        return note.to_string();
    }
    let mut said = String::new();
    if !note.is_empty() {
        said.push_str(note);
        said.push_str("\n\n");
    }
    said.push_str(
        "## What the person pointed at while walking the work\n\n\
         Each item is something they pointed at in the running work and what they said \
         about it, in the order they said it. Address every one.\n",
    );
    for (n, kept) in unsent.iter().enumerate() {
        let words = kept.said.trim().replace('\n', "\n   ");
        said.push_str(&format!(
            "\n{}. {}\n   - location: `{}`\n   - selector: `{}`\n   - said: {}\n",
            n + 1,
            kept.element,
            kept.location,
            kept.selector,
            words
        ));
    }
    said.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::pointed_at;
    use ipc::CaptureElement;

    fn element(tag: &str, text: &str, label: Option<&str>) -> CaptureElement {
        CaptureElement {
            tag: tag.to_string(),
            text: text.to_string(),
            label: label.map(str::to_string),
        }
    }

    #[test]
    fn an_element_is_named_the_way_a_person_reads_it() {
        assert_eq!(
            pointed_at(&element("BUTTON", " Save ", None)),
            "button “Save”"
        );
        assert_eq!(
            pointed_at(&element("a", "x", Some("Pricing"))),
            "link “Pricing”"
        );
        assert_eq!(pointed_at(&element("div", "", None)), "div");
        assert_eq!(
            pointed_at(&element("h2", "Account\n   settings", None)),
            "heading “Account settings”"
        );
        let long = "word ".repeat(30);
        assert!(pointed_at(&element("p", &long, None)).ends_with("…”"));
    }
}
