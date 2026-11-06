// The channel an error card's facts travel over. The renderer names the facts and nothing else: no port, and a body that is
// not an error (no code, no message, no time) is refused here rather than sent.

import type { ReportBridgeError } from "@armada/protocol";
import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = {
  ipc: IpcMain;
  connection: () => FleetConnection | null;
};

const unsent = {
  ok: false,
  outcome: {
    ok: false,
    why: "not_connected",
  },
} as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

/** Whether a value from the renderer has what Fleet reads an error by. */
export function isBridgeError(value: unknown): value is ReportBridgeError {
  if (typeof value !== "object" || value === null) return false;
  const { code, message, at } = value as Record<string, unknown>;
  return named(code) && named(message) && named(at);
}

export function handleOperatorErrors({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.reportBridgeError, (_event, error: unknown) =>
    isBridgeError(error) ? (connection()?.operatorErrors.report(error) ?? unsent) : unsent,
  );
}
