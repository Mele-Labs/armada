// A Job's sketches, drafts and pages as one record. `crates/ipc/src/artifact.rs`,
// `docs/practices/protocol/a-jobs-artifacts-as-one-record.md`, `docs/concepts/artifact.md`.
//
// The header rules in `protocol.ts` hold here. Nothing is published: Bridge reads again on a `session.changed`,
// which an archive, a restore and his Edit publish.

/** What an artifact is. A page is a document under `.armada/documents/`. */
export type ArtifactForm = "sketch" | "draft" | "page";

/** Who wrote a revision, `owner` for his Edit, or who archived an artifact. */
export type ArtifactBy = "agent" | "owner";

/** Who folded an artifact away, and when. Only `restore_artifact` lifts it; a new revision does not. */
export type ArtifactArchived = {
  by: ArtifactBy;
  at: string;
};

/**
 * `POST /sessions/artifacts/archive` and `POST /sessions/artifacts/restore`, each answered with the `JobArtifact` as it
 * now reads. Bridge's press archives as `owner`. 422 `fleet.artifact_not_held` for an id the Job does not hold, naming
 * at most 20 it does; `fleet.artifact_archived_already` and `fleet.artifact_not_archived`; `fleet.artifact_unnamed`
 * and `fleet.session_unnamed` for a Job not named or not known.
 */
export type ArtifactRef = {
  /** The artifact's `id`. */
  artifact: string;
  session_id?: string;
};

/**
 * `POST /sessions/artifacts/edit`: his Edit of a page's or a draft's words, answered with the `JobArtifact` as it now
 * reads, its newest revision `by` `owner`. The Job is sent each change first. A page's `text` is its whole HTML, the same
 * elements and attributes in the same order, with words changed only in text, headings, lists and table cells; a
 * draft's is its markdown. 409 `fleet.edit_not_newest`, naming the newest; 422 `fleet.edit_not_words` for a sketch or
 * an edit of anything but words, naming the first element that differs; `fleet.edit_settled`, `fleet.edit_archived`,
 * `fleet.artifact_not_held` and `fleet.session_unnamed`. 500 `fleet.edit_unkept`.
 */
export type SaveArtifactEdit = {
  session_id: string;
  /** The artifact's `id`: the page's path or `draft:<title>`. */
  artifact: string;
  /** The revision he edited, which must be the newest. */
  revision: number;
  text: string;
};

/** Where an artifact's newest revision stands. */
export type ArtifactState =
  | "reading"
  | "asking"
  | "approved"
  | "sent_back"
  | "withdrawn"
  | "dismissed";

/** One revision of an artifact. */
export type ArtifactRevision = {
  /** 1 and on, in the order they were kept. */
  number: number;
  /** When it was kept. */
  at: string;
  by: ArtifactBy;
  form: ArtifactForm;
};

/**
 * `GET /sessions/artifacts?session_id=` answers a list of these, newest first by when the newest revision was kept.
 * 422 `fleet.session_unnamed` for a Job Fleet does not know.
 */
export type JobArtifact = {
  /** Stable across revisions: `sketch:<title>`, the page's path, or `draft:<title>`. */
  id: string;
  /** The newest revision's title. */
  title: string;
  /** The newest revision's form. */
  form: ArtifactForm;
  /** Oldest first. Never empty. */
  revisions: ArtifactRevision[];
  state: ArtifactState;
  /** His note on a send back, his words with an approval, or his reason with a draft's dismissal. */
  words?: string;
  /**
   * The Needs you item that answers it now: `sketch:<title>`, `document:<path>`, `draft:<row id>`, or `plan:<version>`
   * for a sketch the pending plan names. Absent while an archive keeps it off his list.
   */
  item?: string;
  /** The plan version a sketch goes with: the pending one naming it, or the one whose Approve settled it. */
  in_plan?: number;
  /** Set while archived. `state` still reads as it stood. */
  archived?: ArtifactArchived;
  /** What the newest revision is for, and what it links to. */
  for_line?: ArtifactFor;
};

/** What a link on a For line points at. `link` is the agent's address; the rest are Fleet's. */
export type ArtifactLinkKind = "plan_step" | "task" | "pull_request" | "link";

/** One link on a For line. */
export type ArtifactLink = {
  kind: ArtifactLinkKind;
  label: string;
  /** A plan group's id, a plan task's id, `#<n>`, or an `http(s)` address. */
  target: string;
};

/**
 * What an artifact is for: the agent's one sentence (at most 200 characters), its links, then the links Fleet knows
 * (the task in progress, its plan step, each open pull request the Job holds). Kept per revision.
 */
export type ArtifactFor = {
  text?: string;
  links: ArtifactLink[];
};

/** A link as the agent gives it to `show_sketch` or `propose_draft`: `http://` or `https://`, at most 5. */
export type ForLink = {
  label: string;
  url: string;
};
