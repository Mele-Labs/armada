// The channel the write clashes travel over: one request to Fleet, and the renderer names no port or Manifest.

import type { IpcMain } from "electron";

import type { ClashSide } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const unread = { ok: false, outcome: { ok: false, why: "not_understood" } } as const;

/** What the renderer sent for a clash's other side: a `kind` of the two there are and an id, each the type it is. */
const otherOf = (other: unknown): { kind: ClashSide; id: string } | null => {
  const { kind, id } = (other ?? {}) as { kind?: unknown; id?: unknown };
  return (kind === "session" || kind === "main") && typeof id === "string" ? { kind, id } : null;
};

export function handleClashes({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getWriteClashes, () => connection()?.clashes.get() ?? unsent);
  ipc.handle(CHANNELS.readClashFile, (_event, sessionId: unknown, other: unknown, path: unknown) => {
    const side = otherOf(other);
    if (typeof sessionId !== "string" || sessionId === "" || side === null || typeof path !== "string" || path === "") return unread;
    return connection()?.clashes.file(sessionId, side, path) ?? unsent;
  });
  ipc.handle(CHANNELS.readSessionFileChange, (_event, sessionId: unknown, path: unknown, seen: unknown) => {
    if (typeof sessionId !== "string" || sessionId === "" || typeof path !== "string" || path === "" || (seen !== undefined && typeof seen !== "string")) return unread;
    return connection()?.clashes.fileChange(sessionId, path, seen) ?? unsent;
  });
}
