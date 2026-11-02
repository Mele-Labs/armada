// A Judge on demand: one read-only model call that reads one artifact of a Job when a person presses Judge on it, and
// answers with notes pinned to the parts they are about. `crates/ipc/src/judge.rs`,
// `docs/practices/protocol/a-job-is-judged-on-demand.md`. Since 31 Oct 2026.
//
// The header rules in `protocol.ts` hold here. Only a press starts one, and a note reaches the agent only if the person
// sends it. A Job's readings are `SessionRecord.judge_readings`, so they arrive with every `session.changed` and no event
// carries them. The Judge replaces the old Session judge (concerns in `waiting_for`) and the plan's adversarial review.

/** What kind of thing was read. */
export type JudgeKind =
  | "job"
  | "plan"
  | "pull_request"
  | "commit"
  | "file"
  | "page"
  | "doc"
  | "sketch"
  | "command"
  | "subagent"
  | "issue"
  | "note";

/** What is read. `id` is the kind's own: a pull request's number, a commit's row, a path, a page's address, the plan's version. One reading is kept per `(kind, id)` of a Job. */
export type JudgeArtifact = {
  kind: JudgeKind;
  id: string;
  title: string;
};

/** How far a reading has got. `failed` is never a result. */
export type JudgeState = "reading" | "done" | "failed";

/** How the person settled a note: `sent` to the agent as his message, or `dismissed` with the agent told nothing. */
export type JudgeNoteHow = "sent" | "dismissed";

/** A note once settled. Absent while it waits. */
export type JudgeNoteSettled = {
  how: JudgeNoteHow;
  at: string;
};

/** One thing the Judge found. */
export type JudgeNote = {
  /** Unique within its reading. */
  id: string;
  /** The part it is about, in the artifact's own terms: a plan group or task id, `path:line` of a diff, `p:` and the words of a document's block. Absent, the artifact as a whole. */
  about?: string;
  /** What is wrong, in a line. Heads the note. */
  claim: string;
  /** Where it is, as a person reads it: `auth/middleware.rs:88`. */
  where?: string;
  expected?: string;
  found?: string;
  /** What a person using the thing sees or cannot do because of it. */
  impact?: string;
  /** `path` or `path:line`. Absent where it cites nothing. */
  cites?: string[];
  settled?: JudgeNoteSettled;
};

/** One reading of one artifact of a Job, going or kept. */
export type JudgeReading = {
  /** Fleet's. A reading asked again for the same artifact is a new id. */
  id: string;
  artifact: JudgeArtifact;
  /** What the person asked. Empty asks what was missed. */
  prompt: string;
  state: JudgeState;
  asked_at: string;
  finished_at?: string;
  /** Why a failed reading failed, in Fleet's words. */
  failed?: string;
  /** The notes were opened. Fleet keeps it with the reading. */
  read: boolean;
  /** None is an answer when the state is `done`: it read the artifact and found nothing. */
  notes: JudgeNote[];
  /** What the vendor billed, in millionths of a dollar. Absent where the call ended without saying, which is not a cost of zero. */
  cost_micros?: number;
};

/**
 * `ask_judge`'s body (`POST /sessions/judge`). Answers a `JudgeReading` at once, `reading`. A reading already going for the
 * same artifact is answered, not started twice. 404 `fleet.no_such_session`, 409 `fleet.session_closed`.
 */
export type AskJudge = {
  session_id: string;
  artifact: JudgeArtifact;
  prompt?: string;
};

/** `send_judge_note`'s body (`POST /sessions/judge/send`): the note goes to the agent as the person's message. Answers the reading. 404 `fleet.no_such_judge_reading`, `fleet.no_such_judge_note`; a note already settled is answered as it stands. */
export type SendJudgeNote = {
  session_id: string;
  reading: string;
  note: string;
};

/** `dismiss_judge_note`'s body (`POST /sessions/judge/dismiss`): the note is settled and the agent is told nothing. Answers the reading. The same refusals. */
export type DismissJudgeNote = {
  session_id: string;
  reading: string;
  note: string;
};

/** `mark_judge_read`'s body (`POST /sessions/judge/read`): the reading's notes were opened. Answers the reading with `read` true. */
export type MarkJudgeRead = {
  session_id: string;
  reading: string;
};
