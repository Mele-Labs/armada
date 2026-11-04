// The channels where Jobs' time went travels over: one request to Fleet each, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import { sessionOf } from "./session-system-channels";

type Hosts = {
  ipc: IpcMain;
  connection: () => FleetConnection | null;
};

const unsent = {
  ok: false,
  outcome: {
    ok: false,
    why: "not_connected",
  },
} as const;

export function handleUsageTime({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getUsageTime, () => connection()?.usageTime.time() ?? unsent);
  ipc.handle(CHANNELS.getUsageHours, () => connection()?.usageTime.hours() ?? unsent);
  ipc.handle(CHANNELS.getJobTime, (_event, sessionId: unknown) => {
    const id = sessionOf(sessionId);
    return id === null ? unsent : (connection()?.usageTime.job(id) ?? unsent);
  });
}
