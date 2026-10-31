// A file a Session wrote opens only when that Session's ledger names it as one it wrote.

import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ shell: { openPath: async () => "" } }));

const { namesFile, openSessionFile, readSessionArtifact, namesPage, MOST_BYTES, typeOfFile } = await import("./session-file");

import type { SessionRecord } from "@armada/protocol";

const record = {
  id: "S1",
  attachments: [
    { kind: "artifact", target: "/repo/docs/spike.md", state: "standing", detail: { form: "file" } },
    { kind: "artifact", target: "/tmp/shot.png", state: "standing", detail: { form: "image" } },
    { kind: "artifact", target: "https://example.com/artifact/p1", state: "standing", detail: { form: "page" } },
    { kind: "artifact", target: "note:n1", state: "standing", detail: { form: "note", note: "n1", file: "/notes/n1.json", picture: "/notes/n1.png" } },
  ],
} as unknown as SessionRecord;

describe("opening a file a Session wrote", () => {
  it("opens a path the ledger names as a file", async () => {
    const open = vi.fn(async (_path: string) => "");
    expect(await openSessionFile(record, "/repo/docs/spike.md", open)).toEqual({ ok: true });
    expect(open).toHaveBeenCalledWith("/repo/docs/spike.md");
  });

  it("opens a picture the ledger names as looked at", async () => {
    const open = vi.fn(async (_path: string) => "");
    expect(await openSessionFile(record, "/tmp/shot.png", open)).toEqual({ ok: true });
  });

  it("opens the picture a sent note recorded, and refuses the note's other paths and a note that kept none", async () => {
    const open = vi.fn(async (_path: string) => "");
    expect(await openSessionFile(record, "/notes/n1.png", open)).toEqual({ ok: true });
    expect(await openSessionFile(record, "/notes/n1.json", open)).toMatchObject({ ok: false, why: "not_addressable" });
    expect(await openSessionFile(record, "/notes/n2.png", open)).toMatchObject({ ok: false, why: "not_addressable" });
    // A file form does not stand in for a picture: only a note's own `picture` is named.
    const bare = { id: "S2", attachments: [{ kind: "artifact", target: "note:n3", state: "standing", detail: { form: "note", note: "n3", file: "/notes/n3.json" } }] } as unknown as SessionRecord;
    expect(await openSessionFile(bare, "/notes/n3.png", open)).toMatchObject({ ok: false });
    expect(open).toHaveBeenCalledTimes(1);
  });

  it("refuses a path the ledger does not name, a page's address, and a session Bridge does not hold", async () => {
    const open = vi.fn(async (_path: string) => "");
    expect(await openSessionFile(record, "/etc/passwd", open)).toMatchObject({ ok: false, why: "not_addressable" });
    expect(await openSessionFile(record, "https://example.com/artifact/p1", open)).toMatchObject({ ok: false });
    expect(await openSessionFile(undefined, "/repo/docs/spike.md", open)).toMatchObject({ ok: false });
    expect(open).not.toHaveBeenCalled();
  });

  it("says when the machine declined", async () => {
    expect(await openSessionFile(record, "/repo/docs/spike.md", async () => "no such file")).toEqual({
      ok: false,
      why: "refused",
      address: "/repo/docs/spike.md",
      detail: "no such file",
    });
  });
});

describe("reading a file for the panel", () => {
  const reads = (size: number, bytes: number[] = [104, 105]) => ({ size: async () => size, read: vi.fn(async () => new Uint8Array(bytes)) });

  it("reads a file the ledger names, and says what kind it is", async () => {
    const on = reads(2);
    expect(await readSessionArtifact(record, "/repo/docs/spike.md", on)).toEqual({ ok: true, bytes: new Uint8Array([104, 105]), type: "text/markdown" });
    expect(await readSessionArtifact(record, "/tmp/shot.png", on)).toMatchObject({ ok: true, type: "image/png" });
  });

  it("reads the picture a sent note recorded, as an image, and not the note's file", async () => {
    const on = reads(2, [137, 80]);
    expect(await readSessionArtifact(record, "/notes/n1.png", on)).toMatchObject({ ok: true, type: "image/png" });
    expect(await readSessionArtifact(record, "/notes/n1.json", on)).toEqual({ ok: false, why: "not_addressable" });
  });

  it("refuses a path the ledger does not name, and reads nothing", async () => {
    const on = reads(2);
    expect(await readSessionArtifact(record, "/etc/passwd", on)).toEqual({ ok: false, why: "not_addressable" });
    expect(await readSessionArtifact(record, "https://example.com/artifact/p1", on)).toMatchObject({ ok: false });
    expect(await readSessionArtifact(undefined, "/repo/docs/spike.md", on)).toMatchObject({ ok: false });
    expect(on.read).not.toHaveBeenCalled();
  });

  it("refuses a file over the cap before it is held", async () => {
    const on = reads(MOST_BYTES + 1);
    expect(await readSessionArtifact(record, "/repo/docs/spike.md", on)).toEqual({ ok: false, why: "too_big", limit: MOST_BYTES });
    expect(on.read).not.toHaveBeenCalled();
  });

  it("refuses a file that is not text, and says when it could not be read", async () => {
    expect(await readSessionArtifact(record, "/repo/docs/spike.md", reads(3, [1, 0, 2]))).toEqual({ ok: false, why: "binary" });
    const gone = { size: async () => { throw new Error("ENOENT"); }, read: async () => new Uint8Array() };
    expect(await readSessionArtifact(record, "/repo/docs/spike.md", gone)).toEqual({ ok: false, why: "unreadable" });
  });

  it("reads a file by its extension", () => {
    expect([typeOfFile("a.MD"), typeOfFile("a.svg"), typeOfFile("a.rs")]).toEqual(["text/markdown", "image/svg+xml", "text/plain"]);
  });
});

describe("reading a Job's document", () => {
  const held = {
    id: "S2",
    attachments: [
      {
        kind: "artifact",
        target: "/work/.armada/documents/findings.html",
        state: "standing",
        detail: { form: "document", title: "Findings", revisions: JSON.stringify(["/store/documents/S2/findings/r1.html", "/store/documents/S2/findings/r2.html"]) },
      },
      { kind: "artifact", target: "/work/.armada/documents/broken.html", state: "standing", detail: { form: "document", revisions: "[nope" } },
    ],
  } as unknown as SessionRecord;

  it("names each revision the document row lists and nothing else", () => {
    expect(namesFile(held, "/store/documents/S2/findings/r1.html")).toBe(true);
    expect(namesFile(held, "/store/documents/S2/findings/r2.html")).toBe(true);
    expect(namesFile(held, "/store/documents/S2/findings/r3.html")).toBe(false);
    expect(namesFile(held, "/store/documents/S3/findings/r1.html")).toBe(false);
    expect(namesFile(held, "/work/.armada/documents/findings.html")).toBe(false);
    expect(namesFile(held, "/work/.armada/documents/broken.html")).toBe(false);
    expect(namesFile(held, "/etc/passwd")).toBe(false);
  });

  it("names each redraw a sketch row lists and nothing else", () => {
    const drawn = {
      id: "S5",
      attachments: [{ kind: "artifact", target: "sketch:Flow", state: "standing", detail: { form: "sketch", title: "Flow", revisions: JSON.stringify(["/store/sketches/S5/flow/r1.json", "/store/sketches/S5/flow/r2.json"]) } }],
    } as unknown as SessionRecord;
    expect(namesFile(drawn, "/store/sketches/S5/flow/r1.json")).toBe(true);
    expect(namesFile(drawn, "/store/sketches/S5/flow/r2.json")).toBe(true);
    expect(namesFile(drawn, "/store/sketches/S5/flow/r3.json")).toBe(false);
    expect(namesFile(drawn, "sketch:Flow")).toBe(false);
  });

  it("does not take a revision path from a row that is not a document", () => {
    const other = { id: "S4", attachments: [{ kind: "artifact", target: "/x", state: "standing", detail: { form: "page", revisions: JSON.stringify(["/store/r1.html"]) } }] } as unknown as SessionRecord;
    expect(namesFile(other, "/store/r1.html")).toBe(false);
  });

  it("reads a listed revision and refuses an unlisted one without reading it", async () => {
    const on = { size: async () => 2, read: vi.fn(async () => new Uint8Array([104, 105])) };
    expect(await readSessionArtifact(held, "/store/documents/S2/findings/r2.html", on)).toMatchObject({ ok: true, type: "text/plain" });
    on.read.mockClear();
    expect(await readSessionArtifact(held, "/store/documents/S2/findings/r9.html", on)).toEqual({ ok: false, why: "not_addressable" });
    expect(on.read).not.toHaveBeenCalled();
  });
});

describe("showing a page", () => {
  it("names only an http(s) address the ledger holds as a page or a doc", () => {
    const withPages = {
      id: "S1",
      attachments: [
        { kind: "artifact", target: "https://example.com/p", detail: { form: "page" } },
        { kind: "artifact", target: "https://example.com/d", detail: { form: "doc" } },
        { kind: "artifact", target: "file:///etc/passwd", detail: { form: "page" } },
        { kind: "artifact", target: "https://example.com/f", detail: { form: "file" } },
      ],
    } as unknown as SessionRecord;
    expect(namesPage(withPages, "https://example.com/p")).toBe(true);
    expect(namesPage(withPages, "https://example.com/d")).toBe(true);
    expect(namesPage(withPages, "file:///etc/passwd")).toBe(false);
    expect(namesPage(withPages, "https://example.com/f")).toBe(false);
    expect(namesPage(withPages, "https://evil.example/")).toBe(false);
    expect(namesPage(undefined, "https://example.com/p")).toBe(false);
  });
});
