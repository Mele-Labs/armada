// The channels the territory travels over: one request to Fleet each, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleTerritory({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getRepositoryAreas, () => connection()?.territory.areas() ?? unsent);
  ipc.handle(CHANNELS.getSlotTouches, () => connection()?.territory.touches() ?? unsent);
}
