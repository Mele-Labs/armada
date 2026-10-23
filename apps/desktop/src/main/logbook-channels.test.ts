// The Logbook channels check what crossed from the renderer before Fleet is asked, and answer "not connected" with no Fleet.

import type { IpcMain } from "electron";
import { describe, expect, it } from "vitest";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import { handleLogbook } from "./logbook-channels";

type Handler = (event: unknown, ...args: unknown[]) => unknown;

function channels(connection: FleetConnection | null): Map<string, Handler> {
  const handlers = new Map<string, Handler>();
  handleLogbook({ ipc: { handle: (channel: string, handler: Handler) => handlers.set(channel, handler) } as unknown as IpcMain, connection: () => connection });
  return handlers;
}

const called: string[] = [];
const logbook = {
  read: async () => (called.push("read"), { ok: true, entries: [] }),
  edit: async (id: string, text: string) => (called.push(`edit ${id} ${text}`), { ok: true }),
  lock: async (id: string, locked: boolean) => (called.push(`lock ${id} ${locked}`), { ok: true }),
  forget: async (id: string) => (called.push(`forget ${id}`), { ok: true }),
};
const connected = { logbook } as unknown as FleetConnection;

describe("the Logbook channels", () => {
  it("pass a well-formed call to the host", async () => {
    const handlers = channels(connected);
    await handlers.get(CHANNELS.readLogbook)?.({});
    await handlers.get(CHANNELS.editLogbookEntry)?.({}, "w1", "Words.");
    await handlers.get(CHANNELS.lockLogbookEntry)?.({}, "w1", true);
    await handlers.get(CHANNELS.forgetLogbookEntry)?.({}, "w1");
    expect(called).toEqual(["read", "edit w1 Words.", "lock w1 true", "forget w1"]);
  });

  it("refuse an id or a value that is not the type it should be, without asking Fleet", async () => {
    called.length = 0;
    const handlers = channels(connected);
    const unread = { ok: false, outcome: { ok: false, why: "not_understood" } };
    expect(await handlers.get(CHANNELS.editLogbookEntry)?.({}, "", "Words.")).toEqual(unread);
    expect(await handlers.get(CHANNELS.editLogbookEntry)?.({}, "w1", 3)).toEqual(unread);
    expect(await handlers.get(CHANNELS.lockLogbookEntry)?.({}, "w1", "yes")).toEqual(unread);
    expect(await handlers.get(CHANNELS.forgetLogbookEntry)?.({}, undefined)).toEqual(unread);
    expect(called).toEqual([]);
  });

  it("say Fleet is not connected when there is no connection", async () => {
    const handlers = channels(null);
    const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } };
    expect(await handlers.get(CHANNELS.readLogbook)?.({})).toEqual(unsent);
    expect(await handlers.get(CHANNELS.forgetLogbookEntry)?.({}, "w1")).toEqual(unsent);
  });
});
