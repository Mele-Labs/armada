// The channels reported bugs travel over. Each is one request to Fleet; the renderer names no port, and a request
// that names no report is refused here.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

export function handleBugReports({ ipc, connection }: Hosts): void {
  const host = () => connection()?.bugReports;
  ipc.handle(CHANNELS.listBugReports, () => host()?.list() ?? unsent);
  ipc.handle(CHANNELS.keepBugReport, (_event, id: unknown) => (named(id) ? (host()?.keep(id) ?? unsent) : unsent));
  ipc.handle(CHANNELS.dropBugReport, (_event, id: unknown) => (named(id) ? (host()?.drop(id) ?? unsent) : unsent));
}
