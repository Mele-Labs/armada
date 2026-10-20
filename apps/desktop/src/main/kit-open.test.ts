import { describe, expect, it, vi } from "vitest";
import type { KitInventory } from "@armada/protocol";

const { showItemInFolder, inEditor } = vi.hoisted(() => ({
  showItemInFolder: vi.fn(),
  inEditor: vi.fn(async (_state: unknown, _path: string): Promise<string | null> => null),
}));
vi.mock("electron", () => ({ shell: { showItemInFolder, openPath: async () => "" } }));
vi.mock("./open", () => ({ inEditor }));

import { NOTHING_YET } from "../shared/bridge";
import { namesKitPath, openKitFile } from "./kit-open";

const HOME = "/Users/user/.agent";
const setup: KitInventory = {
  harness: "Agent",
  home: HOME,
  present: true,
  kinds: [
    { kind: "skills", read: { what: "read", unreadable: [], items: [{ name: "a", source: `${HOME}/skills/a` }, { name: "rel", source: "skills/rel" }] } },
    { kind: "output_styles", read: { what: "read", unreadable: [], items: [{ name: "b", source: "built in to the agent" }] } },
    { kind: "allowlist", read: { what: "read", unreadable: [], items: [{ name: "git status", source: "/etc/passwd" }] } },
  ],
};

describe("opening a Kit item", () => {
  it("names a path the inventory lists, absolute or read under the home", () => {
    expect(namesKitPath(setup, `${HOME}/skills/a`)).toBe(true);
    expect(namesKitPath(setup, `${HOME}/skills/rel`)).toBe(true);
  });

  it("names nothing the inventory does not list, the allowlist's lines included", () => {
    expect(namesKitPath(setup, "/etc/hosts")).toBe(false);
    expect(namesKitPath(setup, "/etc/passwd")).toBe(false);
    expect(namesKitPath(setup, "built in to the agent")).toBe(false);
  });

  it("shows a listed path in Finder and opens it in the editor", async () => {
    const read = { ok: true as const, setup };
    expect(await openKitFile(NOTHING_YET, read, `${HOME}/skills/a`, "finder")).toEqual({ ok: true });
    expect(showItemInFolder).toHaveBeenCalledWith(`${HOME}/skills/a`);
    expect(await openKitFile(NOTHING_YET, read, `${HOME}/skills/a`, "editor")).toEqual({ ok: true });
    expect(inEditor).toHaveBeenCalledWith(NOTHING_YET, `${HOME}/skills/a`);
  });

  it("carries the editor's refusal through", async () => {
    inEditor.mockResolvedValueOnce("editor.command names nope, and nothing on PATH is called that.");
    const done = await openKitFile(NOTHING_YET, { ok: true, setup }, `${HOME}/skills/a`, "editor");
    expect(done).toEqual({ ok: false, detail: "editor.command names nope, and nothing on PATH is called that." });
  });

  it("refuses an unlisted path, an unknown way and an unread inventory without touching the OS", async () => {
    showItemInFolder.mockClear();
    inEditor.mockClear();
    expect((await openKitFile(NOTHING_YET, { ok: true, setup }, "/etc/hosts", "finder")).ok).toBe(false);
    expect((await openKitFile(NOTHING_YET, { ok: true, setup }, `${HOME}/skills/a`, "rm")).ok).toBe(false);
    expect((await openKitFile(NOTHING_YET, { ok: false, outcome: { ok: false, why: "not_connected" } }, `${HOME}/skills/a`, "editor")).ok).toBe(false);
    expect(showItemInFolder).not.toHaveBeenCalled();
    expect(inEditor).not.toHaveBeenCalled();
  });
});
