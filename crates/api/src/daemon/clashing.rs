//! `list_write_clashes`, and the two reads beside it of what each agent has changed and where.
//!
//! **Its own surface**, for [`Sessions`](super::Sessions)' reason: whole on its own.

use std::future::Future;

use ipc::{ClashFile, ClashSide, ManifestId, RepositoryAreas, SessionFileChange, SlotTouches, WriteClashes};

use crate::daemon::Refusal;

pub trait Clashing: Send + Sync + 'static {
    /// `list_write_clashes` — the clashes standing in one repository, or those
    /// `holder` (a Job's or a Session's id) is in. `manifest_id` absent is the
    /// first served. Read from what Fleet last checked; asks git for nothing.
    fn list_write_clashes(
        &self,
        manifest_id: Option<ManifestId>,
        holder: Option<String>,
    ) -> impl Future<Output = Result<WriteClashes, Refusal>> + Send;

    /// `read_clash_file` — one file of the clash between `session_id` and the other holder, as
    /// each changed it. `other_id` is ignored where `other_kind` is `main`. Asks git, since a
    /// patch is too large to keep.
    fn read_clash_file(
        &self,
        session_id: String,
        other_kind: ClashSide,
        other_id: String,
        path: String,
    ) -> impl Future<Output = Result<ClashFile, Refusal>> + Send;

    /// `get_session_file_change` — one file the Job `session_id` has been seen writing, as its
    /// worktree changed it since its branch met the base, uncommitted edits included. Asks git.
    /// `seen` is the stamp of the change the caller holds; an equal one is answered without the diff.
    fn get_session_file_change(
        &self,
        session_id: String,
        path: String,
        seen: Option<String>,
    ) -> impl Future<Output = Result<SessionFileChange, Refusal>> + Send;

    /// `list_repository_areas` — the directories of every served repository's base, to two names
    /// deep, with the files under each. Read from git at the base's head, and kept against it.
    fn list_repository_areas(&self) -> impl Future<Output = Result<RepositoryAreas, Refusal>> + Send;

    /// `list_slot_touches` — what each Job or hosted Session holding a slot has changed in it, by
    /// repository. Read from git and kept a few seconds.
    fn list_slot_touches(&self) -> impl Future<Output = Result<SlotTouches, Refusal>> + Send;
}
