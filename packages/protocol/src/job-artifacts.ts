// A Job's sketches, drafts and pages as one record. `crates/ipc/src/artifact.rs`,
// `docs/practices/protocol/a-jobs-artifacts-as-one-record.md`, `docs/concepts/artifact.md`.
//
// The header rules in `protocol.ts` hold here. Nothing is published: Bridge reads again on a `session.changed`.

/** What an artifact is. A page is a document under `.armada/documents/`. */
export type ArtifactForm = "sketch" | "draft" | "page";

/** Who wrote a revision. Always `agent` today. */
export type ArtifactBy = "agent" | "owner";

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
  /** The Needs you item that answers it now: `sketch:<title>`, `document:<path>` or `draft:<row id>`. */
  item?: string;
};
