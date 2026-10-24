// The shared home's channels check what crossed from the renderer before Fleet is asked, rebuild a row from the fields
// Fleet takes, and answer "not connected" with no Fleet.

import type { IpcMain } from "electron";
import { describe, expect, it } from "vitest";

import { CHANNELS } from "../shared/bridge";
import { handleAccounts } from "./accounts-channels";
import type { FleetConnection } from "./connection";

type Handler = (event: unknown, ...args: unknown[]) => unknown;

function channels(connection: FleetConnection | null): Map<string, Handler> {
  const handlers = new Map<string, Handler>();
  handleAccounts({ ipc: { handle: (channel: string, handler: Handler) => handlers.set(channel, handler) } as unknown as IpcMain, connection: () => connection });
  return handlers;
}

const called: string[] = [];
const said = (name: string) => async (...args: unknown[]) => (called.push(`${name} ${JSON.stringify(args)}`), { ok: true });
const accounts = {
  sharedHome: said("read"),
  syncSharedHome: said("sync"),
  addToSharedHome: said("add"),
  bringFromSharedHome: said("bring"),
  chooseSharedHomeSide: said("choose"),
  homeKinds: said("kinds"),
  setHomeKind: said("set"),
};
const connected = { accounts } as unknown as FleetConnection;
const unread = { ok: false, outcome: { ok: false, why: "not_understood" } };

describe("the shared home's channels", () => {
  it("pass a well-formed call to the host, a row as the fields Fleet takes and nothing more", async () => {
    called.length = 0;
    const handlers = channels(connected);
    const note = { login: "nickmele", kind: "memory", name: "Walks, not pictures", repository: "armada", state: "here" };
    await handlers.get(CHANNELS.getSharedHome)?.({}, "nickmele");
    await handlers.get(CHANNELS.syncSharedHome)?.({}, "nickmele");
    await handlers.get(CHANNELS.addToSharedHome)?.({}, note);
    await handlers.get(CHANNELS.bringFromSharedHome)?.({}, { login: "nickmele", kind: "mods", name: "ember" });
    await handlers.get(CHANNELS.chooseSharedHomeSide)?.({}, { login: "nickmele", kind: "mods", name: "dusk" }, "there");
    await handlers.get(CHANNELS.getHomeKinds)?.({}, "nickmele");
    await handlers.get(CHANNELS.setHomeKind)?.({}, "nickmele", "mods", true);
    expect(called).toEqual([
      'read ["nickmele"]',
      'sync ["nickmele"]',
      'add [{"login":"nickmele","kind":"memory","name":"Walks, not pictures","repository":"armada"}]',
      'bring [{"login":"nickmele","kind":"mods","name":"ember"}]',
      'choose [{"login":"nickmele","kind":"mods","name":"dusk"},"there"]',
      'kinds ["nickmele"]',
      'set ["nickmele","mods",true]',
    ]);
  });

  it("refuse a login, a row, a kind or a side that is not what it should be, without asking Fleet", async () => {
    called.length = 0;
    const handlers = channels(connected);
    expect(await handlers.get(CHANNELS.getSharedHome)?.({}, "")).toEqual(unread);
    expect(await handlers.get(CHANNELS.syncSharedHome)?.({}, 3)).toEqual(unread);
    expect(await handlers.get(CHANNELS.addToSharedHome)?.({}, { login: "nickmele", kind: "agent_file", name: "AGENTS.md" })).toEqual(unread);
    expect(await handlers.get(CHANNELS.bringFromSharedHome)?.({}, null)).toEqual(unread);
    expect(await handlers.get(CHANNELS.chooseSharedHomeSide)?.({}, { login: "nickmele", kind: "mods", name: "dusk" }, "both")).toEqual(unread);
    expect(await handlers.get(CHANNELS.setHomeKind)?.({}, "nickmele", "mods", "yes")).toEqual(unread);
    expect(called).toEqual([]);
  });

  it("say Fleet is not connected when there is no connection", async () => {
    const handlers = channels(null);
    const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } };
    expect(await handlers.get(CHANNELS.getSharedHome)?.({}, "nickmele")).toEqual(unsent);
    expect(await handlers.get(CHANNELS.setHomeKind)?.({}, "nickmele", "mods", true)).toEqual(unsent);
  });
});
