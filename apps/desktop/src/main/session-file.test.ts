// A file a Session wrote opens only when that Session's ledger names it as one it wrote.

import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ shell: { openPath: async () => "" } }));

const { openSessionFile } = await import("./session-file");

import type { SessionRecord } from "@armada/protocol";

const record = {
  id: "S1",
  attachments: [
    { kind: "artifact", target: "/repo/docs/spike.md", state: "standing", detail: { form: "file" } },
    { kind: "artifact", target: "https://example.com/artifact/p1", state: "standing", detail: { form: "page" } },
  ],
} as unknown as SessionRecord;

describe("opening a file a Session wrote", () => {
  it("opens a path the ledger names as a file", async () => {
    const open = vi.fn(async (_path: string) => "");
    expect(await openSessionFile(record, "/repo/docs/spike.md", open)).toEqual({ ok: true });
    expect(open).toHaveBeenCalledWith("/repo/docs/spike.md");
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
