// The channels the agents' sign-in travels over. Each is one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleAgentSignIn({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getAgentSignIn, () => connection()?.agentSignIn.get() ?? unsent);
  ipc.handle(CHANNELS.startAgentSignIn, () => connection()?.agentSignIn.start() ?? unsent);
}
