// The channels Sleep mode travels over. Each is one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const text = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

export function handleSleep({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getSleep, () => connection()?.sleep.get() ?? unsent);
  ipc.handle(CHANNELS.setSleep, (_event, on: unknown) => (typeof on === "boolean" ? (connection()?.sleep.set(on) ?? unsent) : unsent));
  ipc.handle(CHANNELS.overrideSleep, (_event, id: unknown, words: unknown) =>
    text(id) && text(words) ? (connection()?.sleep.override(id, words.trim()) ?? unsent) : unsent,
  );
}
