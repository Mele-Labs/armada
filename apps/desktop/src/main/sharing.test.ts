// What crosses from the renderer to commit, share or add a piece is checked for its shape before Fleet is asked, and a package is
// written only where the person chose, always ending in the package's extension.

import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ app: { getPath: () => "/Downloads" }, BrowserWindow: { fromWebContents: () => null }, dialog: {} }));

const { addOf, bindOf, commitOf, packageNameOf, pieceOf, savePackageFile, shareOf } = await import("./sharing");

describe("a piece's selector", () => {
  it("takes the four kinds by the id each uses, and nothing else", () => {
    expect(pieceOf({ kind: "flight_controls", job: ["run-tests"] })).toEqual({ kind: "flight_controls", job: ["run-tests"] });
    expect(pieceOf({ kind: "workflow", workflow_id: "release" })).toEqual({ kind: "workflow", workflow_id: "release" });
    expect(pieceOf({ kind: "review_tool", id: "security-pass" })).toEqual({ kind: "review_tool", id: "security-pass" });
    expect(pieceOf({ kind: "flight_controls", job: [3] })).toBeNull();
    expect(pieceOf({ kind: "workflow", id: "release" })).toBeNull();
    expect(pieceOf({ kind: "manifest" })).toEqual({ kind: "manifest" });
    expect(pieceOf({ kind: "theme", id: "x" })).toBeNull();
  });

  it("takes a Workspace, lifecycle, rule or autonomy default by its id, and the scope its file declares", () => {
    expect(pieceOf({ kind: "workspace", id: "my-store" })).toEqual({ kind: "workspace", id: "my-store" });
    expect(pieceOf({ kind: "lifecycle", id: "ticket" })).toEqual({ kind: "lifecycle", id: "ticket" });
    expect(pieceOf({ kind: "lifecycle_rule", id: "progress", scope: { kind: "within", id: "my-store" } })).toEqual({ kind: "lifecycle_rule", id: "progress", scope: { kind: "within", id: "my-store" } });
    expect(pieceOf({ kind: "autonomy", id: "defaults", scope: { kind: "everywhere", extra: 1 } })).toEqual({ kind: "autonomy", id: "defaults", scope: { kind: "everywhere" } });
    expect(pieceOf({ kind: "lifecycle", id: "ticket", scope: { kind: "within" } })).toBeNull();
    expect(pieceOf({ kind: "lifecycle", id: "ticket", scope: "everywhere" })).toBeNull();
    expect(pieceOf({ kind: "lifecycle" })).toBeNull();
  });
});

describe("the bodies", () => {
  it("a commit names a repository and a piece, and never a Job", () => {
    const piece = { kind: "workflow", workflow_id: "release" };
    expect(commitOf({ repository: "/r", piece })).toEqual({ repository: "/r", piece });
    // A Job named by an older caller never reaches Fleet, which would refuse the body.
    expect(commitOf({ repository: "/r", session_id: "s1", piece })).toEqual({ repository: "/r", piece });
    expect(commitOf({ repository: "/r", piece: { kind: "manifest" } })).toEqual({ repository: "/r", piece: { kind: "manifest" } });
    expect(commitOf({ repository: "/r", piece, overwrite: "yes" })).toBeNull();
    expect(commitOf({ piece })).toBeNull();
  });

  it("a removal is sent only when it is one, since a Fleet older than the field refuses a body that has it", () => {
    const piece = { kind: "lifecycle", id: "ticket" };
    expect(commitOf({ repository: "/r", piece, remove: true })).toEqual({ repository: "/r", piece, remove: true });
    expect(commitOf({ repository: "/r", piece, remove: false })).toEqual({ repository: "/r", piece });
    expect(commitOf({ repository: "/r", piece, remove: "yes" })).toBeNull();
  });

  it("a share holds at least one piece, each with its repository", () => {
    const one = { repository: "/r", piece: { kind: "review_tool", id: "security-pass" } };
    expect(shareOf({ pieces: [one] })).toEqual({ pieces: [one] });
    expect(shareOf({ pieces: [] })).toBeNull();
    expect(shareOf({ pieces: [{ piece: one.piece }] })).toBeNull();
  });

  it("an add carries text, and a bind an unbound id and a repository", () => {
    expect(addOf({ text: "version = 1", replace: true })).toEqual({ text: "version = 1", replace: true });
    expect(addOf({ text: " " })).toBeNull();
    expect(bindOf({ unbound_id: "u1", repository: "/r" })).toEqual({ unbound_id: "u1", repository: "/r" });
    expect(bindOf({ unbound_id: "u1" })).toBeNull();
  });
});

describe("saving a package", () => {
  const host = (chosen: string | undefined) => ({ choose: vi.fn(async (_name: string) => chosen), write: vi.fn(async (_path: string, _text: string) => undefined) });

  it("offers Fleet's name, with no folder, and writes the text where the person chose", async () => {
    const h = host("/Users/me/Desktop/run-tests.armada.toml");
    await expect(savePackageFile("../../run-tests.armada.toml", "version = 1\n", h)).resolves.toEqual({ saved: true, name: "run-tests.armada.toml" });
    expect(h.choose).toHaveBeenCalledWith("run-tests.armada.toml");
    expect(h.write).toHaveBeenCalledWith("/Users/me/Desktop/run-tests.armada.toml", "version = 1\n");
  });

  it("puts the extension back where the person typed over it", async () => {
    const h = host("/Users/me/Desktop/tiles");
    await savePackageFile("run-tests.armada.toml", "version = 1\n", h);
    expect(h.write).toHaveBeenCalledWith("/Users/me/Desktop/tiles.armada.toml", "version = 1\n");
  });

  it("writes nothing for a closed dialog or an empty text", async () => {
    const closed = host(undefined);
    await expect(savePackageFile("a.armada.toml", "version = 1\n", closed)).resolves.toEqual({ saved: false, why: "cancelled" });
    const empty = host("/x.armada.toml");
    await expect(savePackageFile("a.armada.toml", "", empty)).resolves.toEqual({ saved: false, why: "failed" });
    expect(closed.write).not.toHaveBeenCalled();
    expect(empty.choose).not.toHaveBeenCalled();
  });

  it("names a package with no name pieces", () => {
    expect(packageNameOf("")).toBe("pieces.armada.toml");
    expect(packageNameOf("tiles")).toBe("tiles.armada.toml");
  });
});
