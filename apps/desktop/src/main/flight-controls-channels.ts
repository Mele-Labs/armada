// The channels the flight controls travel over: a request to Fleet each, and the renderer names no port.

import type { IpcMain } from "electron";
import type { FlightControls } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** The two ids and the line of a press, or `null` for anything else: what crosses from the renderer is not trusted. */
export function shellOf(sessionId: unknown, command: unknown): { sessionId: string; command: string } | null {
  return typeof sessionId === "string" && sessionId !== "" && typeof command === "string" ? { sessionId, command } : null;
}

export function handleFlightControls({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getFlightControls, () => connection()?.flightControls.get() ?? unsent);
  ipc.handle(CHANNELS.saveFlightControls, (_event, controls: FlightControls) => connection()?.flightControls.save(controls) ?? unsent);
  ipc.handle(CHANNELS.runSessionShell, (_event, sessionId: unknown, command: unknown) => {
    const line = shellOf(sessionId, command);
    return line === null ? unsent : (connection()?.flightControls.runShell(line.sessionId, line.command) ?? unsent);
  });
}
