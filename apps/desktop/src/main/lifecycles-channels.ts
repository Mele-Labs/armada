// The channels Workspaces, lifecycles and rules travel over. Each is one request to Fleet; the renderer names no port,
// and a request that names no repository or Job is refused here.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

export function handleLifecycles({ ipc, connection }: Hosts): void {
  const host = () => connection()?.lifecycles;
  ipc.handle(CHANNELS.listRepositoryWorkspaces, (_event, manifestId: unknown) => (named(manifestId) ? (host()?.workspaces(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.listLifecycles, (_event, manifestId: unknown) => (named(manifestId) ? (host()?.lifecycles(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.listLifecycleRules, (_event, manifestId: unknown) => (named(manifestId) ? (host()?.rules(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.getJobLifecycles, (_event, sessionId: unknown) => (named(sessionId) ? (host()?.job(sessionId) ?? unsent) : unsent));
}
