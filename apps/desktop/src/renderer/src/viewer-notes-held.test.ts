// Notes held on a sketch, a file or a picture until they are sent: kept per item, and gone from that item when sent.

import { afterEach, describe, expect, it } from "vitest";
import type { HeldNote } from "@armada/components";
import { heldOn, setHeldOn } from "@armada/components/src/compositions/ViewerNotes/held";

const note = (text: string): HeldNote => ({ about: { kind: "file", path: "a.rs", from: 1, to: 1, quote: "x" }, text });

const KEYS = ["file:a.rs", "file:b.rs"];
afterEach(() => {
  for (const key of KEYS) setHeldOn(key, () => ({ notes: [] }));
});

describe("held notes", () => {
  it("are held on the item they were written on and not on another", () => {
    setHeldOn("file:a.rs", (was) => ({ ...was, notes: [...was.notes, note("on a")] }));
    expect(heldOn("file:a.rs").notes.map((one) => one.text)).toEqual(["on a"]);
    expect(heldOn("file:b.rs").notes).toEqual([]);
  });

  it("add up in the order held", () => {
    setHeldOn("file:a.rs", (was) => ({ ...was, notes: [...was.notes, note("first")] }));
    setHeldOn("file:a.rs", (was) => ({ ...was, notes: [...was.notes, note("second")] }));
    expect(heldOn("file:a.rs").notes.map((one) => one.text)).toEqual(["first", "second"]);
  });

  it("are cleared for the item that was sent and kept for the others", () => {
    setHeldOn("file:a.rs", (was) => ({ ...was, notes: [note("on a")] }));
    setHeldOn("file:b.rs", (was) => ({ ...was, notes: [note("on b")] }));
    setHeldOn("file:a.rs", (was) => ({ ...was, notes: [] }));
    expect(heldOn("file:a.rs").notes).toEqual([]);
    expect(heldOn("file:b.rs").notes.map((one) => one.text)).toEqual(["on b"]);
  });

  it("leave the Job picked for the item where the notes are cleared", () => {
    setHeldOn("file:a.rs", () => ({ notes: [note("on a")], job: "job-1" }));
    setHeldOn("file:a.rs", (was) => ({ ...was, notes: [] }));
    expect(heldOn("file:a.rs")).toEqual({ notes: [], job: "job-1" });
  });
});
