// A file a Session wrote opens only when that Session's ledger names it as one it wrote.

import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ shell: { openPath: async () => "" } }));

const { openSessionFile, readSessionArtifact, namesPage, MOST_BYTES, typeOfFile } = await import("./session-file");

import type { SessionRecord } from "@armada/protocol";

const record = {
  id: "S1",
  attachments: [
    { kind: "artifact", target: "/repo/docs/spike.md", state: "standing", detail: { form: "file" } },
    { kind: "artifact", target: "/tmp/shot.png", state: "standing", detail: { form: "image" } },
    { kind: "artifact", target: "https://example.com/artifact/p1", state: "standing", detail: { form: "page" } },
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
