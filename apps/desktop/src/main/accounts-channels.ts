// The channels the account travels over. Each is one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const id = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value > 0;

export function handleAccounts({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getAccounts, () => connection()?.accounts.get() ?? unsent);
  ipc.handle(CHANNELS.startSignIn, () => connection()?.accounts.startSignIn() ?? unsent);
  ipc.handle(CHANNELS.signOut, (_event, accountId: unknown) => (id(accountId) ? (connection()?.accounts.signOut(accountId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.setActiveAccount, (_event, accountId: unknown) =>
    id(accountId) ? (connection()?.accounts.setActive(accountId) ?? unsent) : unsent,
  );
}
