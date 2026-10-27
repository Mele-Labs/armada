import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { mockPullRequestNotesApi, resolvePullRequestNoteInMock } from "../mock/slices/pull-request-notes";
import { pullRequestNotesReads } from "./fleet-reads";
import { addNote, editNote, forgetNotes, loadNotes, messageOf, readNotes, removeNote, sendNotes } from "./job-notes";
import type { NoteHandle } from "./job-notes";

const at = { path: "src/a.ts", line: 12, side: "right" } as const;
let handle: NoteHandle;

beforeEach(() => {
  handle = { reads: pullRequestNotesReads(mockPullRequestNotesApi()), manifestId: "armada", number: 7 };
  forgetNotes();
});
afterEach(() => forgetNotes());

test("a note is kept by Fleet, so it survives a reload, and nothing is sent to the forge", async () => {
  const forge = vi.spyOn(globalThis, "fetch");
  await addNote(handle, { ...at, body: "Take the lock after the call." });
  // A reload keeps nothing in the window: what is read back is what Fleet holds.
  forgetNotes();
  expect(readNotes(handle)).toEqual([]);
  await loadNotes(handle);
  expect(readNotes(handle)).toMatchObject([{ path: "src/a.ts", line: 12, side: "right", body: "Take the lock after the call.", sent: false, done: false }]);
  expect(forge).not.toHaveBeenCalled();
  forge.mockRestore();
});

test("a blank note is not kept, and a note is edited or removed by its id", async () => {
  await addNote(handle, { ...at, body: "   " });
  expect(readNotes(handle)).toEqual([]);
  await addNote(handle, { ...at, body: "first" });
  const [one] = readNotes(handle);
  await editNote(handle, one!.id, "second");
  expect(readNotes(handle)[0]!.body).toBe("second");
  await removeNote(handle, one!.id);
  expect(readNotes(handle)).toEqual([]);
});

test("the message numbers each note with its file, line and id and names the resolve tool, and sending marks them sent to that Job", async () => {
  await addNote(handle, { ...at, body: "first" });
  await addNote(handle, { path: "src/b.ts", line: 3, side: "left", body: "second" });
  const unsent = readNotes(handle);
  expect(messageOf(7, unsent)).toBe(
    "Notes on #7 from the owner, kept on his Mac and not posted to the pull request. When you have dealt with a note, call resolve_pull_request_note with its id.\n\n1. src/a.ts:12 [note-1]: first\n2. src/b.ts:3 on the removed line [note-2]: second",
  );
  expect(await sendNotes(handle, unsent.map((one) => one.id), { id: "job-1", title: "Tide cache" })).toBe(true);
  expect(readNotes(handle).map((one) => [one.sent, one.title])).toEqual([
    [true, "Tide cache"],
    [true, "Tide cache"],
  ]);
});

test("a note the Job resolved reads as done once the notes are read again", async () => {
  await addNote(handle, { ...at, body: "first" });
  await sendNotes(handle, ["note-1"], { id: "job-1", title: "Tide cache" });
  resolvePullRequestNoteInMock("armada", 7, "note-1");
  await loadNotes(handle);
  expect(readNotes(handle)[0]).toMatchObject({ sent: true, done: true });
});
