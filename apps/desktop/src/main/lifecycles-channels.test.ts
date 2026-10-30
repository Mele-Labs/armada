// What the renderer may ask main to save or delete of a Workspace, lifecycle or rule is checked for its shape before Fleet is asked. What the piece says is Fleet's parser's to refuse.

import { describe, expect, it, vi } from "vitest";
import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import { deleteOf, handleLifecycles, keepOf, saveOf } from "./lifecycles-channels";

const lifecycle = { kind: "lifecycle", lifecycle: { id: "ticket", name: "Ticket", scope: { kind: "everywhere" }, states: [], transitions: [], ends: [] } };

describe("a piece to save", () => {
  it("is one of the four kinds, with overwrite only as a flag", () => {
    expect(saveOf({ piece: lifecycle })).toEqual({ piece: lifecycle });
    expect(saveOf({ piece: lifecycle, overwrite: true })).toEqual({ piece: lifecycle, overwrite: true });
    expect(saveOf({ piece: lifecycle, overwrite: "yes" })).toBeNull();
    expect(saveOf({ piece: { kind: "theme" } })).toBeNull();
    expect(saveOf({ piece: "ticket" })).toBeNull();
    expect(saveOf({})).toBeNull();
    expect(saveOf(null)).toBeNull();
  });
});

describe("a piece to delete", () => {
  it("names its kind and id, and the scope its file declares", () => {
    expect(deleteOf({ kind: "rule", id: "progress" })).toEqual({ kind: "rule", id: "progress" });
    expect(deleteOf({ kind: "lifecycle", id: "ticket", scope: { kind: "within", id: "01MF" } })).toEqual({ kind: "lifecycle", id: "ticket", scope: { kind: "within", id: "01MF" } });
    expect(deleteOf({ kind: "lifecycle", id: "ticket", scope: { kind: "within" } })).toBeNull();
    expect(deleteOf({ kind: "lifecycle", id: " " })).toBeNull();
    expect(deleteOf({ kind: "tile", id: "ticket" })).toBeNull();
    expect(deleteOf(undefined)).toBeNull();
  });
});

describe("a piece of a Job's copy to keep for every Job", () => {
  it("is a rule or a lifecycle, named by the Job and the id it declares", () => {
    expect(keepOf({ session_id: "s1", kind: "rule", id: "ask_for_review" })).toEqual({ session_id: "s1", kind: "rule", id: "ask_for_review" });
    expect(keepOf({ session_id: "s1", kind: "lifecycle", id: "ticket", scope: { kind: "everywhere" } })).toEqual({ session_id: "s1", kind: "lifecycle", id: "ticket" });
    expect(keepOf({ session_id: "s1", kind: "workspace", id: "mine" })).toBeNull();
    expect(keepOf({ session_id: " ", kind: "rule", id: "ask_for_review" })).toBeNull();
    expect(keepOf({ session_id: "s1", kind: "rule", id: "" })).toBeNull();
    expect(keepOf(undefined)).toBeNull();
  });
});

describe("the Keep for every Job drafts of a repository, asked of main", () => {
  /** `handleLifecycles` over a Fleet whose Keep drafts read is `keepDrafts`, and the handler it registered for each channel. */
  const registered = (keepDrafts: (manifestId: string) => unknown, connected = true) => {
    const handlers = new Map<string, (event: unknown, ...args: unknown[]) => unknown>();
    const ipc = { handle: (channel: string, handler: (event: unknown, ...args: unknown[]) => unknown) => void handlers.set(channel, handler) } as unknown as IpcMain;
    handleLifecycles({ ipc, connection: () => (connected ? ({ lifecycles: { keepDrafts } } as unknown as FleetConnection) : null) });
    return handlers.get(CHANNELS.listKeepDrafts)!;
  };

  it("are read for the repository named, as Fleet answers", () => {
    const keepDrafts = vi.fn(() => ({ ok: true, value: { manifest_id: "01WEB", drafts: [] } }));
    expect(registered(keepDrafts)({}, "01WEB")).toEqual({ ok: true, value: { manifest_id: "01WEB", drafts: [] } });
    expect(keepDrafts).toHaveBeenCalledWith("01WEB");
  });

  it("are not asked of Fleet for a repository that is not named, or with no Fleet connected", () => {
    const keepDrafts = vi.fn();
    const read = registered(keepDrafts);
    for (const asked of [undefined, null, 7, " "]) expect(read({}, asked)).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
    expect(keepDrafts).not.toHaveBeenCalled();
    expect(registered(keepDrafts, false)({}, "01WEB")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});
