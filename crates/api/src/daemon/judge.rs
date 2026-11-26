//! A Judge on demand, and the person's three acts on what it read. Its own surface, for
//! [`SharedHome`](super::SharedHome)'s reason: a handler taking only this cannot reach for a Job's record.
//! `docs/concepts/judge.md`; `docs/practices/protocol/a-job-is-judged-on-demand.md`.

use std::future::Future;

use ipc::{
    AskJudge, DeleteJudgeType, DismissJudgeNote, JudgeReading, JudgeTypes, ManifestId, MarkJudgeRead, SaveJudgeType, SendJudgeNote,
};

use crate::daemon::Refusal;

pub trait JudgeReadings: Send + Sync + 'static {
    /// `ask_judge` — one read-only model call that reads one artifact of a Job. **Answers at once**, with the
    /// reading `reading`; the call runs in a task. A reading already going for the same artifact is answered,
    /// not started twice. [`Refusal::NotFound`] (`fleet.no_such_session`) for no such Job,
    /// [`Refusal::IllegalMove`] (`fleet.session_closed`) for one that ended.
    fn ask_judge(
        self: std::sync::Arc<Self>,
        ask: AskJudge,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;

    /// `send_judge_note` — a note goes to the agent as the person's message, then is settled. A note already
    /// settled is answered as it stands. [`Refusal::NotFound`] (`fleet.no_such_session`,
    /// `fleet.no_such_judge_reading` for a reading the Job does not hold, `fleet.no_such_judge_note`).
    fn send_judge_note(
        self: std::sync::Arc<Self>,
        send: SendJudgeNote,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;

    /// `dismiss_judge_note` — a note is settled and the agent is told nothing. The same refusals.
    fn dismiss_judge_note(
        self: std::sync::Arc<Self>,
        dismiss: DismissJudgeNote,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;

    /// `mark_judge_read` — the person opened the reading's notes. The agent is told nothing.
    /// [`Refusal::NotFound`] for no such Job or reading.
    fn mark_judge_read(
        self: std::sync::Arc<Self>,
        read: MarkJudgeRead,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;

    /// `list_judge_types` — the types one project sees for a repository, resolved most specific first: Kit's for
    /// the project, Kit's for all, the repository's, Armada's. Read from the folders each time.
    /// With no repository named, the types a Job in a folder with no Manifest sees: Armada's and Kit's.
    /// [`Refusal`] for a repository Fleet does not serve.
    fn list_judge_types(
        self: std::sync::Arc<Self>,
        manifest_id: Option<ManifestId>,
        project: Option<String>,
    ) -> impl Future<Output = Result<JudgeTypes, Refusal>> + Send;

    /// `save_judge_type` — write one type in Kit's folder (for one project or all) or the repository's.
    /// [`Refusal::Unacceptable`] for a blank or wordless name, a blank prompt, an id the place already defines
    /// unless `overwrite`, and a project with the repository's scope; [`Refusal::Fault`] where it cannot be written.
    fn save_judge_type(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        save: SaveJudgeType,
    ) -> impl Future<Output = Result<JudgeTypes, Refusal>> + Send;

    /// `delete_judge_type` — remove the file that defines an id in one place. Deleting Kit's copy of an Armada id
    /// is how Armada's prompt comes back. [`Refusal::Unacceptable`] where the place holds none.
    fn delete_judge_type(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        delete: DeleteJudgeType,
    ) -> impl Future<Output = Result<JudgeTypes, Refusal>> + Send;
}
