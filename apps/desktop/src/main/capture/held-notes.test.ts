// The notes held on a Job until the owner sends them, as main keeps them across a Bridge restart.
// `docs/practices/capture-window.md`, *Held notes*.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, it, vi } from "vitest";
import type { SendSessionNote } from "@armada/protocol";

import { HeldNotes, heldOn } from "./held-notes";

const note = (sessionId: string, said: string): SendSessionNote => ({
  session_id: sessionId,
  said,
  address: "http://localhost:5173/",
  place: { selector: "main > button", element: { tag: "button", text: "Reload" }, location: "/" },
  state: {},
  crop: { media_type: "image/png", data: "iVBORw0KGgo=" },
});

const file = () => join(mkdtempSync(join(tmpdir(), "held-notes-")), "held-notes.json");

let minted = 0;
const store = (at: string | null, changed: (sessionId: string) => void = () => {}) =>
  new HeldNotes(at, changed, () => `n${(minted += 1)}`, () => "2026-10-19T14:00:00.000Z");

it("holds each note on its Job with an id and a time, the crop with it, and none on another Job", () => {
  const held = store(null);
  const first = held.hold(note("a", "Too tight"));
  held.hold(note("a", "Wrong colour"));
  held.hold(note("b", "Elsewhere"));
  expect(held.of("a").map((one) => one.note.said)).toEqual(["Too tight", "Wrong colour"]);
  expect(held.of("a")[0]).toEqual({ id: first.id, at: "2026-10-19T14:00:00.000Z", note: { ...note("a", "Too tight"), session_id: undefined } });
  expect(held.of("a")[0]!.note).not.toHaveProperty("session_id");
  expect(held.of("a")[0]!.note.crop).toEqual({ media_type: "image/png", data: "iVBORw0KGgo=" });
  expect(held.of("c")).toEqual([]);
});

it("keeps them across a restart: a second store on the same file reads what the first held, edited and removed", () => {
  const at = file();
  const before = store(at);
  const one = before.hold(note("a", "Too tight"));
  const two = before.hold(note("a", "Wrong colour"));
  before.hold(note("b", "Elsewhere"));
  expect(before.edit("a", one.id, "  Too tight on the left  ")).toBe(true);
  expect(before.remove("a", two.id)).toBe(true);
  const after = store(at);
  expect(after.of("a")).toEqual([{ id: one.id, at: one.at, note: { ...one.note, said: "Too tight on the left" } }]);
  expect(after.of("b").map((held) => held.note.said)).toEqual(["Elsewhere"]);
});

it("drops only the notes it was told were sent, and a Job left with none leaves the file", () => {
  const at = file();
  const held = store(at);
  const one = held.hold(note("a", "Too tight"));
  const two = held.hold(note("a", "Wrong colour"));
  held.drop("a", [one.id]);
  expect(held.of("a").map((left) => left.id)).toEqual([two.id]);
  held.drop("a", [two.id]);
  expect(JSON.parse(readFileSync(at, "utf8"))).toEqual({ jobs: {} });
});

it("refuses an edit to nothing and an id it does not hold on that Job, and tells nobody", () => {
  const changed = vi.fn();
  const held = store(null, changed);
  const one = held.hold(note("a", "Too tight"));
  changed.mockClear();
  expect(held.edit("a", one.id, "   ")).toBe(false);
  expect(held.edit("b", one.id, "Moved")).toBe(false);
  expect(held.remove("b", one.id)).toBe(false);
  expect(held.of("a")[0]!.note.said).toBe("Too tight");
  expect(changed).not.toHaveBeenCalled();
});

it("tells which Job changed on every hold, edit, remove and drop", () => {
  const changed = vi.fn();
  const held = store(null, changed);
  const one = held.hold(note("a", "Too tight"));
  held.edit("a", one.id, "Looser");
  held.remove("a", one.id);
  held.hold(note("b", "Elsewhere"));
  held.drop("b", held.of("b").map((left) => left.id));
  expect(changed.mock.calls).toEqual([["a"], ["a"], ["a"], ["b"], ["b"]]);
});

it("starts empty on a file that is missing or not its own, and holds from there", () => {
  const at = file();
  expect(store(at).of("a")).toEqual([]);
  writeFileSync(at, "{ not json");
  expect(store(at).of("a")).toEqual([]);
  writeFileSync(at, JSON.stringify({ jobs: { a: [{ id: 1 }] } }));
  const held = store(at);
  expect(held.of("a")).toEqual([]);
  held.hold(note("a", "Too tight"));
  expect(store(at).of("a").map((one) => one.note.said)).toEqual(["Too tight"]);
});

it("holds a note saved on a Job's walk, and on a review's walk its Job holds, at the address the note names; sends the rest as before", () => {
  const review = { walkId: "w1", manifestId: "armada", number: 12, title: "#12 Take a draft out", address: "https://pr-12.preview.invalid/app" };
  expect(heldOn({ session: { id: "a", title: "Retry backoff cap" } }, "http://localhost:5173/table")).toEqual({ sessionId: "a", address: "http://localhost:5173/table" });
  expect(heldOn({ review: { ...review, holder: { id: "d", title: "Take a draft out" } } }, "http://localhost:41311/")).toEqual({ sessionId: "d", address: "https://pr-12.preview.invalid/app" });
  expect(heldOn({ review }, "http://localhost:41311/")).toBeNull();
  expect(heldOn({ studio: { id: "s1", name: "Ledger" } }, "http://localhost:5173/")).toBeNull();
});
