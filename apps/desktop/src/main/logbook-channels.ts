// The channels the Logbook page travels over: one request to Fleet each, and the renderer names no port. What crosses
// from the renderer is not trusted, so an id and a text are checked to be the strings they are before Fleet is asked.

import type { IpcMain } from "electron";

import type { OrganizeAction, OrganizeProposal } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const unread = { ok: false, outcome: { ok: false, why: "not_understood" } } as const;

const ACTIONS = ["merge", "stale", "bugs"];
const action = (value: unknown): value is OrganizeAction => typeof value === "string" && ACTIONS.includes(value);
const proposal = (value: unknown): value is OrganizeProposal => {
  const one = value as OrganizeProposal | null;
  return typeof one === "object" && one !== null && named(one.id) && typeof one.text === "string" && typeof one.note === "string" && Array.isArray(one.entry_ids) && one.entry_ids.every(named);
};

const named = (value: unknown): value is string => typeof value === "string" && value !== "";

export function handleLogbook({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.readLogbook, () => connection()?.logbook.read() ?? unsent);
  ipc.handle(CHANNELS.editLogbookEntry, (_event, id: unknown, text: unknown) =>
    !named(id) || typeof text !== "string" ? unread : (connection()?.logbook.edit(id, text) ?? unsent),
  );
  ipc.handle(CHANNELS.lockLogbookEntry, (_event, id: unknown, locked: unknown) =>
    !named(id) || typeof locked !== "boolean" ? unread : (connection()?.logbook.lock(id, locked) ?? unsent),
  );
  ipc.handle(CHANNELS.tidyLogbook, () => connection()?.logbook.tidy() ?? unsent);
  ipc.handle(CHANNELS.organizeLogbook, (_event, which: unknown) => (!action(which) ? unread : (connection()?.logbook.organize(which) ?? unsent)));
  ipc.handle(CHANNELS.applyOrganizeLogbook, (_event, which: unknown, proposals: unknown) =>
    !action(which) || !Array.isArray(proposals) || !proposals.every(proposal) ? unread : (connection()?.logbook.applyOrganize(which, proposals) ?? unsent),
  );
  ipc.handle(CHANNELS.forgetLogbookEntry, (_event, id: unknown) => (!named(id) ? unread : (connection()?.logbook.forget(id) ?? unsent)));
}
