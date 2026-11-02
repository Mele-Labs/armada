// The channels Autopilot's answers travel over. Each is one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const text = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

export function handleAutopilotDecisions({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getAutopilotDecisions, () => connection()?.autopilotDecisions.get() ?? unsent);
  ipc.handle(CHANNELS.reviewAutopilotDecision, (_event, id: unknown) => (text(id) ? (connection()?.autopilotDecisions.review(id) ?? unsent) : unsent));
  ipc.handle(CHANNELS.overrideAutopilotDecision, (_event, id: unknown, words: unknown) =>
    text(id) && text(words) ? (connection()?.autopilotDecisions.override(id, words.trim()) ?? unsent) : unsent,
  );
}
