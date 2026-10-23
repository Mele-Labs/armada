// The channels teaching travels over: one request to Fleet, and the renderer names no port. A body is checked for its
// shape here, since it crosses from a window; Fleet checks what it says.

import type { IpcMain } from "electron";

import type { ReadTeaching, SaveTeaching } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const unread = { ok: false, outcome: { ok: false, why: "not_understood" } } as const;

const isBody = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const optionalText = (value: unknown): value is string | undefined => value === undefined || typeof value === "string";

export function handleTeaching({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.readTeaching, (_event, read: unknown) => {
    if (!isBody(read) || typeof read.said !== "string" || (read.reach !== "repository" && read.reach !== "everywhere")) return unread;
    return connection()?.teaching.read(read as ReadTeaching) ?? unsent;
  });
  ipc.handle(CHANNELS.saveTeaching, (_event, save: unknown) => {
    if (!isBody(save) || (save.reach !== "repository" && save.reach !== "everywhere")) return unread;
    return connection()?.teaching.save(save as SaveTeaching) ?? unsent;
  });
  ipc.handle(CHANNELS.listWordRules, (_event, repository: unknown, sessionId: unknown) => {
    if (!optionalText(repository) || !optionalText(sessionId)) return unread;
    return connection()?.teaching.rules(repository, sessionId) ?? unsent;
  });
}
