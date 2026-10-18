// The channel the usage summary travels over: one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** An offset in whole minutes within a day either way: what crosses from the renderer is not trusted. `null` for anything else. */
export function offsetOf(minutes: unknown): number | null {
  return typeof minutes === "number" && Number.isInteger(minutes) && Math.abs(minutes) <= 24 * 60 ? minutes : null;
}

export function handleUsageSummary({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getUsageSummary, (_event, minutes: unknown) => {
    const offset = offsetOf(minutes);
    return offset === null ? unsent : (connection()?.usageSummary.get(offset) ?? unsent);
  });
  ipc.handle(CHANNELS.getUsageRules, () => connection()?.usageSummary.rules() ?? unsent);
}
