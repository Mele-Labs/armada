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

/** A repository's root, or `null` for anything else. */
export function repositoryOf(repository: unknown): string | null {
  return typeof repository === "string" && repository !== "" ? repository : null;
}

/** A root and a file holding both lists, or `null` for anything else. What is in each list is Fleet's to refuse. */
export function repositorySaveOf(repository: unknown, controls: unknown): { repository: string; controls: FlightControls } | null {
  const root = repositoryOf(repository);
  if (root === null || typeof controls !== "object" || controls === null) return null;
  const { job, deck } = controls as Partial<FlightControls>;
  return Array.isArray(job) && Array.isArray(deck) ? { repository: root, controls: controls as FlightControls } : null;
}

export function handleFlightControls({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getFlightControls, () => connection()?.flightControls.get() ?? unsent);
  ipc.handle(CHANNELS.saveFlightControls, (_event, controls: FlightControls) => connection()?.flightControls.save(controls) ?? unsent);
  ipc.handle(CHANNELS.getRepositoryFlightControls, (_event, repository: unknown) => {
    const root = repositoryOf(repository);
    return root === null ? unsent : (connection()?.flightControls.getRepository(root) ?? unsent);
  });
  ipc.handle(CHANNELS.saveRepositoryFlightControls, (_event, repository: unknown, controls: unknown) => {
    const save = repositorySaveOf(repository, controls);
    return save === null ? unsent : (connection()?.flightControls.saveRepository(save.repository, save.controls) ?? unsent);
  });
  ipc.handle(CHANNELS.runSessionShell, (_event, sessionId: unknown, command: unknown) => {
    const line = shellOf(sessionId, command);
    return line === null ? unsent : (connection()?.flightControls.runShell(line.sessionId, line.command) ?? unsent);
  });
}
