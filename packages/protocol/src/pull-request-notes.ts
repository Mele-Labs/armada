// The owner's notes on a pull request's lines, kept by Fleet so the Job a note was sent to can resolve it.
// `crates/ipc/src/pull_request_notes.rs`, `docs/concepts/pull-review.md`. The header rules in `protocol.ts` hold here.

/** Which side of the diff a note's line is on. */
export type NoteSide = "left" | "right";

/** Whether a Job has dealt with the note. */
export type NoteStatus = "open" | "done";

/** One note on one line of a pull request. */
export type PullRequestNote = {
  /** Fleet-made. */
  id: string;
  /** `owner/name`, as the forge names the repository. */
  repository: string;
  number: number;
  path: string;
  line: number;
  side: NoteSide;
  /** Never blank, and trimmed. */
  body: string;
  status: NoteStatus;
  /** The Job the note was sent to. Absent until it is sent. */
  sent_to?: string;
  /** That Job's title when the note was sent. */
  sent_title?: string;
  sent_at?: string;
  created_at: string;
};

/** Every note on one pull request, oldest first: `list_pull_request_notes`, and what the commands that change the notes answer. */
export type PullRequestNotes = { notes: PullRequestNote[] };

/** Without `id` a save makes a note and `path`, `line` and `side` are required. With `id` it edits that note's `body`, only while the note is open and unsent. */
export type SavePullRequestNote = {
  id?: string;
  path?: string;
  line?: number;
  side?: NoteSide;
  body: string;
};

/** These notes were sent to this Job. Fleet records it; Bridge sends the message itself. */
export type SendPullRequestNotes = {
  note_ids: string[];
  session_id: string;
  /** The Job's title now, kept so a note still says who has it after the Job is renamed. */
  title: string;
};
