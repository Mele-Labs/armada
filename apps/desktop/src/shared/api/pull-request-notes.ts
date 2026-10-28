// The owner's notes on a pull request's lines, kept by Fleet. A slice imports protocol, never another slice;
// `../api.ts` and `../bridge.ts` compose them. `docs/concepts/pull-review.md`.

import type { Outcome, PullRequestNote, PullRequestNotes, SavePullRequestNote, SendPullRequestNotes } from "@armada/protocol";

/** What reading, removing or sending notes came to: every note on the pull request, or the refusal to word. */
export type PullRequestNotesAnswer = { ok: true; value: PullRequestNotes } | { ok: false; outcome: Outcome };

/** What saving one note came to: the note as kept, or the refusal to word. */
export type PullRequestNoteAnswer = { ok: true; value: PullRequestNote } | { ok: false; outcome: Outcome };

export type PullRequestNotesApi = {
  /** `GET /pull_request_notes/:manifestId/:number`: every note on the pull request, oldest first. */
  listPullRequestNotes: (manifestId: string, number: number) => Promise<PullRequestNotesAnswer>;
  /** `POST /pull_request_notes/:manifestId/:number`: makes a note, or with an `id` edits an open, unsent one's body. */
  savePullRequestNote: (manifestId: string, number: number, save: SavePullRequestNote) => Promise<PullRequestNoteAnswer>;
  /** `POST /pull_request_notes/:manifestId/:number/:id/remove`: answers every note left. */
  removePullRequestNote: (manifestId: string, number: number, id: string) => Promise<PullRequestNotesAnswer>;
  /** `POST /pull_request_notes/:manifestId/:number/sent`: records these notes as sent to a Job. Answers every note. */
  sendPullRequestNotes: (manifestId: string, number: number, send: SendPullRequestNotes) => Promise<PullRequestNotesAnswer>;
};

export const PULL_REQUEST_NOTES_CHANNELS = {
  listPullRequestNotes: "bridge:list-pull-request-notes",
  savePullRequestNote: "bridge:save-pull-request-note",
  removePullRequestNote: "bridge:remove-pull-request-note",
  sendPullRequestNotes: "bridge:send-pull-request-notes",
} as const;
