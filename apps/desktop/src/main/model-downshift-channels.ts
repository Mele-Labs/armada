// The channels the default model's step down travels over. Each is one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleModelDownshift({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getModelDownshift, () => connection()?.modelDownshift.get() ?? unsent);
  ipc.handle(CHANNELS.acceptDownshift, () => connection()?.modelDownshift.accept() ?? unsent);
  ipc.handle(CHANNELS.restoreDownshift, () => connection()?.modelDownshift.restore() ?? unsent);
  ipc.handle(CHANNELS.keepDownshift, () => connection()?.modelDownshift.keep() ?? unsent);
  ipc.handle(CHANNELS.settleDownshift, () => connection()?.modelDownshift.settle() ?? unsent);
}
