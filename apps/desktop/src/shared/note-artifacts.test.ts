// A note sent to a Job, as Fleet keeps it on the Job's ledger, and the watch that tells the layer a note file changed.

import { describe, expect, it } from "vitest";
import type { Attachment, SessionRecord } from "@armada/protocol";

import { NotesWatch, noteArtifactOf, noteArtifactsOf } from "./note-artifacts";

const AT = "2026-10-07T13:48:02Z";

const artifact = (detail: Record<string, string>, kind = "artifact"): Attachment => ({
  kind,
  manifest_id: "armada",
  target: `note:${detail["note"] ?? "x"}`,
  state: "standing",
  detail,
  since: AT,
  changed_at: AT,
});

/** The details Fleet writes for a note, every one a string. */
const FULL = {
  form: "note",
  note: "20261007-140000-aaaa",
  title: "The title is cut off",
  text: "The title is cut off in the row\nAnd it is worse narrow",
  tag: "section",
  component: "Deck",
  screen: "Cockpit",
  address: "http://localhost:5173/",
  file: "/notes/20261007-140000-aaaa.json",
  picture: "/notes/20261007-140000-aaaa.png",
  status: "open",
  sent: AT,
};

describe("reading a note off an attachment", () => {
  it("reads every field Fleet kept", () => {
    expect(noteArtifactOf(artifact(FULL))).toEqual({
      note: "20261007-140000-aaaa",
      title: "The title is cut off",
      text: "The title is cut off in the row\nAnd it is worse narrow",
      tag: "section",
      component: "Deck",
      screen: "Cockpit",
      address: "http://localhost:5173/",
      file: "/notes/20261007-140000-aaaa.json",
      picture: "/notes/20261007-140000-aaaa.png",
      status: "open",
      sent: AT,
    });
  });

  it("leaves off what a note was never pinned to, and takes its text as the title where none was kept", () => {
    const { component, screen, address, file, picture, title, ...bare } = FULL;
    void [component, screen, address, file, picture, title];
    const read = noteArtifactOf(artifact(bare));
    expect(read).toEqual({ note: FULL.note, title: FULL.text, text: FULL.text, tag: "section", status: "open", sent: AT });
    for (const key of ["component", "screen", "address", "file", "picture"]) expect(read).not.toHaveProperty(key);
  });

  it("reads each state a note can be in", () => {
    for (const status of ["open", "done", "returned"]) expect(noteArtifactOf(artifact({ ...FULL, status }))?.status).toBe(status);
  });

  it("refuses a note missing a field it needs", () => {
    for (const key of ["note", "text", "tag", "sent", "status"] as const) {
      const { [key]: _gone, ...without } = FULL;
      expect(noteArtifactOf(artifact(without)), `without ${key}`).toBeNull();
    }
  });

  it("refuses a state it does not know", () => {
    expect(noteArtifactOf(artifact({ ...FULL, status: "archived" }))).toBeNull();
  });

  it("refuses what is not a note: another form of artifact, or another kind of attachment", () => {
    expect(noteArtifactOf(artifact({ ...FULL, form: "file" }))).toBeNull();
    expect(noteArtifactOf(artifact({ ...FULL, form: "sketch" }))).toBeNull();
    expect(noteArtifactOf(artifact(FULL, "pull_request"))).toBeNull();
    expect(noteArtifactOf({ ...artifact(FULL), detail: undefined })).toBeNull();
  });

  it("lists a Job's notes in the order its record keeps them, and none of the rest", () => {
    const record = {
      attachments: [artifact({ ...FULL, note: "a" }), artifact({ form: "file" }), artifact({ ...FULL, note: "b", status: "done" }), artifact({ ...FULL, note: "c" })],
    };
    expect(noteArtifactsOf(record).map((one) => [one.note, one.status])).toEqual([
      ["a", "open"],
      ["b", "done"],
      ["c", "open"],
    ]);
  });
});

describe("telling the layer a note file changed", () => {
  const job = (id: string, notes: Record<string, string>, state = "live"): Pick<SessionRecord, "id" | "state" | "attachments"> => ({
    id,
    state: state as SessionRecord["state"],
    attachments: Object.entries(notes).map(([note, status]) => artifact({ ...FULL, note, status })),
  });

  it("says nothing of the first list, where no Job holds a note", () => {
    const watch = new NotesWatch();
    expect(watch.read([job("a", {})])).toBe(false);
    expect(watch.read([job("a", {})])).toBe(false);
  });

  it("says nothing of a list that is the same as the last", () => {
    const watch = new NotesWatch();
    watch.read([job("a", { n1: "open" })]);
    expect(watch.read([job("a", { n1: "open" })])).toBe(false);
  });

  it("fires when a note's state moved", () => {
    const watch = new NotesWatch();
    watch.read([job("a", { n1: "open" })]);
    expect(watch.read([job("a", { n1: "done" })])).toBe(true);
    // Once told, the same list is quiet again.
    expect(watch.read([job("a", { n1: "done" })])).toBe(false);
    expect(watch.read([job("a", { n1: "returned" })])).toBe(true);
  });

  it("fires when a Job that holds notes ends, for its close marks them or lets them go", () => {
    const watch = new NotesWatch();
    watch.read([job("a", { n1: "open" })]);
    expect(watch.read([job("a", { n1: "open" }, "ended")])).toBe(true);
  });

  it("fires when a note was added to a Job it already knew", () => {
    const watch = new NotesWatch();
    watch.read([job("a", { n1: "open" })]);
    expect(watch.read([job("a", { n1: "open", n2: "open" })])).toBe(true);
  });

  it("fires for a Job first met holding notes, and not for one first met with none", () => {
    expect(new NotesWatch().read([job("a", { n1: "done" })])).toBe(true);
    expect(new NotesWatch().read([job("a", {})])).toBe(false);
  });

  it("fires when a Job met live ends, notes or none, for its close is when Fleet rewrites them", () => {
    const watch = new NotesWatch();
    watch.read([job("a", {})]);
    expect(watch.read([job("a", {}, "ended")])).toBe(true);
  });

  it("ignores a change to anything but a note artifact", () => {
    const watch = new NotesWatch();
    const before = job("a", { n1: "open" });
    watch.read([before]);
    const other = artifact({ form: "file" });
    expect(watch.read([{ ...before, attachments: [...before.attachments, other] }])).toBe(false);
  });
});
