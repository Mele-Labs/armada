// The channel a bay's history travels over: one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** A repository and a slot, each the type it is: what crosses from the renderer is not trusted. `null` for anything else. */
export function bayOf(manifestId: unknown, slot: unknown): { manifestId: string; slot: number } | null {
  if (typeof manifestId !== "string" || manifestId === "") return null;
  if (typeof slot !== "number" || !Number.isInteger(slot) || slot < 0) return null;
  return { manifestId, slot };
}

export function handleBayHistory({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getSlotHistory, (_event, manifestId: unknown, slot: unknown) => {
    const bay = bayOf(manifestId, slot);
    return bay === null ? unsent : (connection()?.bayHistory.get(bay.manifestId, bay.slot) ?? unsent);
  });
}
