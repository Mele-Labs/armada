// The channel a repository's open issues travel over. The renderer names a Manifest and no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleIssues({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.listIssues, (_event, manifestId: unknown, refresh: unknown) =>
    typeof manifestId === "string" && manifestId.trim() !== ""
      ? (connection()?.issues.list(manifestId, refresh === true) ?? unsent)
      : unsent,
  );
}
