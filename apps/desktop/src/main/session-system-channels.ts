// The channels a Job's system travels over: one request to Fleet each, and the renderer names no port.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { ProcessTarget } from "../shared/api/session-system";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** A Job's id, as the string it is: what crosses from the renderer is not trusted. `null` for anything else. */
export function sessionOf(sessionId: unknown): string | null {
  return typeof sessionId === "string" && sessionId !== "" ? sessionId : null;
}

/** A pid or a port, exactly one, as a whole positive number. `null` for anything else. */
export function targetOf(target: unknown): ProcessTarget | null {
  if (typeof target !== "object" || target === null) return null;
  const { pid, port } = target as { pid?: unknown; port?: unknown };
  const whole = (value: unknown): value is number => typeof value === "number" && Number.isInteger(value) && value > 0;
  if (whole(pid) && port === undefined) return { pid };
  if (whole(port) && pid === undefined && port <= 65535) return { port };
  return null;
}

export function handleSessionSystem({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getSessionSystem, (_event, sessionId: unknown) => {
    const id = sessionOf(sessionId);
    return id === null ? unsent : (connection()?.sessionSystem.get(id) ?? unsent);
  });
  ipc.handle(CHANNELS.stopProcess, (_event, sessionId: unknown, target: unknown) => {
    const id = sessionOf(sessionId);
    const process = targetOf(target);
    return id === null || process === null ? unsent : (connection()?.sessionSystem.stop(id, process) ?? unsent);
  });
}
