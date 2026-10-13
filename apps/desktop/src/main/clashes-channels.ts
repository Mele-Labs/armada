// The channel the write clashes travel over: one request to Fleet, and the renderer names no port or Manifest.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleClashes({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getWriteClashes, () => connection()?.clashes.get() ?? unsent);
}
