// The mock's notes answer as Fleet does, and a Job's resolve is stood in for once after a send.

import { expect, test } from "vitest";

import { pullRequestNotes, resolvePullRequestNoteInMock } from "./pull-request-notes";

const make = () => pullRequestNotes.api({} as never, {} as never);
const ok = <T extends { ok: boolean }>(answer: T) => {
  expect(answer.ok).toBe(true);
  return (answer as unknown as { value: never }).value;
};
const PLACE = { path: "src/a.ts", line: 3, side: "right" as const };

test("a note is made open, edited while unsent, and refused once sent", async () => {
  const api = make();
  const note = ok(await api.savePullRequestNote("m", 7, { ...PLACE, body: " Why? " })) as { id: string; status: string; body: string };
  expect(note).toMatchObject({ id: "note-1", status: "open", body: "Why?" });
  expect(ok(await api.savePullRequestNote("m", 7, { id: note.id, body: "Why this?" }))).toMatchObject({ body: "Why this?" });
  await api.sendPullRequestNotes("m", 7, { note_ids: [note.id], session_id: "s1", title: "Fix" });
  expect(await api.savePullRequestNote("m", 7, { id: note.id, body: "x" })).toMatchObject({ ok: false });
  expect(await api.savePullRequestNote("m", 7, { body: "no place" })).toMatchObject({ ok: false });
});

test("notes are kept per pull request and removed by id", async () => {
  const api = make();
  await api.savePullRequestNote("m", 1, { ...PLACE, body: "a" });
  const b = ok(await api.savePullRequestNote("m", 1, { ...PLACE, body: "b" })) as { id: string };
  expect(ok(await api.listPullRequestNotes("m", 2))).toEqual({ notes: [] });
  expect((ok(await api.removePullRequestNote("m", 1, b.id)) as { notes: unknown[] }).notes).toHaveLength(1);
  expect(await api.removePullRequestNote("m", 1, "nope")).toMatchObject({ ok: false });
});

test("the first list after a send shows the first sent note done, once", async () => {
  const api = make();
  await api.savePullRequestNote("m", 1, { ...PLACE, body: "a" });
  await api.savePullRequestNote("m", 1, { ...PLACE, body: "b" });
  const sent = ok(await api.sendPullRequestNotes("m", 1, { note_ids: ["note-1", "note-2"], session_id: "s1", title: "Fix" })) as { notes: { status: string; sent_to?: string }[] };
  expect(sent.notes.map((n) => [n.status, n.sent_to])).toEqual([["open", "s1"], ["open", "s1"]]);
  const listed = ok(await api.listPullRequestNotes("m", 1)) as { notes: { status: string }[] };
  expect(listed.notes.map((n) => n.status)).toEqual(["done", "open"]);
  expect(resolvePullRequestNoteInMock("m", 1, "note-2")).toBe(true);
  expect((ok(await api.listPullRequestNotes("m", 1)) as { notes: { status: string }[] }).notes.map((n) => n.status)).toEqual(["done", "done"]);
});
