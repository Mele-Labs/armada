// A pull request's notes, kept in memory per `${manifestId}#${number}` and answered the way Fleet does: a save without an id makes an open
// note, with one edits the body only while the note is open and unsent, and a send marks the notes sent.
//
// **Mock only:** the first `listPullRequestNotes` after a `sendPullRequestNotes` for that pull request resolves the first note of the send,
// standing in for the Job calling `resolve_pull_request_note`, so a walk can show a note done. A test does it by hand with
// `resolvePullRequestNoteInMock`. Starts empty.

import type { PullRequestNote, WireError } from "@armada/protocol";

import type { PullRequestNotesApi } from "../../../../shared/api/pull-request-notes";
import type { Slice } from "../fake-context";

type Kept = { notes: PullRequestNote[]; resolveNext: string | undefined };

const kept = new Map<string, Kept>();
let made = 0;

const keyOf = (manifestId: string, number: number) => `${manifestId}#${number}`;
const heldFor = (key: string): Kept => {
  const found = kept.get(key);
  if (found !== undefined) return found;
  const fresh: Kept = { notes: [], resolveNext: undefined };
  kept.set(key, fresh);
  return fresh;
};

const refusal = (code: string, message: string) => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false as const, outcome: { ok: false as const, why: "refused" as const, error } };
};

/** What the Job's `resolve_pull_request_note` does: the note is done. Mock only; answers whether a note was found. */
export function resolvePullRequestNoteInMock(manifestId: string, number: number, id: string): boolean {
  const note = kept.get(keyOf(manifestId, number))?.notes.find((one) => one.id === id);
  if (note === undefined) return false;
  note.status = "done";
  return true;
}

/** The notes api over memory, for a window that starts empty. The panel's fixture reads take it too. */
export function mockPullRequestNotesApi(): PullRequestNotesApi {
  // Each window starts empty.
  kept.clear();
  made = 0;
  const all = (key: string) => ({ ok: true as const, value: { notes: structuredClone(heldFor(key).notes) } });
  return {
    listPullRequestNotes: async (manifestId, number) => {
      const key = keyOf(manifestId, number);
      const held = heldFor(key);
      if (held.resolveNext !== undefined) {
        resolvePullRequestNoteInMock(manifestId, number, held.resolveNext);
        held.resolveNext = undefined;
      }
      return all(key);
    },
    savePullRequestNote: async (manifestId, number, save) => {
      const held = heldFor(keyOf(manifestId, number));
      const body = save.body.trim();
      if (body === "") return refusal("fleet.note_blank", "A note needs words");
      if (save.id !== undefined) {
        const note = held.notes.find((one) => one.id === save.id);
        if (note === undefined) return refusal("fleet.note_not_found", `No note ${save.id} on this pull request`);
        if (note.status !== "open" || note.sent_to !== undefined) return refusal("fleet.note_sent", "A note already sent to a Job cannot be edited");
        note.body = body;
        return { ok: true, value: structuredClone(note) };
      }
      if (save.path === undefined || save.line === undefined || save.side === undefined) return refusal("fleet.note_place_missing", "A new note needs a path, a line and a side");
      made += 1;
      const note: PullRequestNote = {
        id: `note-${made}`,
        repository: manifestId,
        number,
        path: save.path,
        line: save.line,
        side: save.side,
        body,
        status: "open",
        created_at: new Date().toISOString(),
      };
      held.notes.push(note);
      return { ok: true, value: structuredClone(note) };
    },
    removePullRequestNote: async (manifestId, number, id) => {
      const key = keyOf(manifestId, number);
      const held = heldFor(key);
      if (!held.notes.some((one) => one.id === id)) return refusal("fleet.note_not_found", `No note ${id} on this pull request`);
      held.notes = held.notes.filter((one) => one.id !== id);
      return all(key);
    },
    sendPullRequestNotes: async (manifestId, number, send) => {
      const key = keyOf(manifestId, number);
      const held = heldFor(key);
      const chosen = send.note_ids.map((id) => held.notes.find((one) => one.id === id));
      if (chosen.length === 0 || chosen.some((one) => one === undefined)) return refusal("fleet.note_not_found", "A note named is not on this pull request");
      const at = new Date().toISOString();
      for (const note of chosen as PullRequestNote[]) {
        note.sent_to = send.session_id;
        note.sent_title = send.title;
        note.sent_at = at;
      }
      held.resolveNext = send.note_ids[0];
      return all(key);
    },
  };
}

export const pullRequestNotes: Slice<PullRequestNotesApi, Record<string, never>> = { name: "pullRequestNotes", state: {}, api: mockPullRequestNotesApi };
