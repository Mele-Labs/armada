import { afterEach, expect, test, vi } from "vitest";

import { addNote, editNote, markSent, messageOf, readNotes, removeNote } from "./job-notes";

const KEY = "mele-labs/armada#7";
const at = { path: "src/a.ts", line: 12, side: "right" } as const;

afterEach(() => window.localStorage.clear());

test("a note survives a remount and nothing is sent to the forge", () => {
  const forge = vi.spyOn(globalThis, "fetch");
  addNote(KEY, { ...at, body: "Take the lock after the call." });
  // A remount reads what is kept, not what the module held.
  expect(readNotes(KEY)).toMatchObject([{ path: "src/a.ts", line: 12, side: "right", body: "Take the lock after the call.", sent: false }]);
  expect(readNotes("mele-labs/armada#8")).toEqual([]);
  expect(forge).not.toHaveBeenCalled();
  forge.mockRestore();
});

test("a blank note is not kept, and a note is edited or removed by its id", () => {
  addNote(KEY, { ...at, body: "   " });
  expect(readNotes(KEY)).toEqual([]);
  const [one] = addNote(KEY, { ...at, body: "first" });
  editNote(KEY, one!.id, "second");
  expect(readNotes(KEY)[0]!.body).toBe("second");
  removeNote(KEY, one!.id);
  expect(readNotes(KEY)).toEqual([]);
});

test("the message numbers each unsent note with its file and line, and sending marks them sent", () => {
  addNote(KEY, { ...at, body: "first" });
  addNote(KEY, { path: "src/b.ts", line: 3, side: "left", body: "second" });
  const unsent = readNotes(KEY).filter((one) => !one.sent);
  expect(messageOf(7, unsent)).toBe("Notes on #7 from the owner, kept on his Mac and not posted to the pull request\n\n1. src/a.ts:12: first\n2. src/b.ts:3 (the removed line): second");
  markSent(KEY, unsent.map((one) => one.id));
  expect(readNotes(KEY).every((one) => one.sent)).toBe(true);
});
