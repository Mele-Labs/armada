// The channels the flight controls travel over: a request to Fleet each, and the renderer names no port.

import type { IpcMain } from "electron";
import type { FlightControls } from "@armada/protocol";
import type { ApproveRepositoryTile, RepositoryTile } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** The two ids and the line of a press, or `null` for anything else: what crosses from the renderer is not trusted. */
export function shellOf(sessionId: unknown, command: unknown): { sessionId: string; command: string } | null {
  return typeof sessionId === "string" && sessionId !== "" && typeof command === "string" ? { sessionId, command } : null;
}

/** A committed tile a press names, `undefined` where it names none, or `null` for anything else. */
export function tileOf(tile: unknown): RepositoryTile | undefined | null {
  if (tile === undefined) return undefined;
  if (typeof tile !== "object" || tile === null) return null;
  const { repository, id } = tile as Record<string, unknown>;
  return typeof repository === "string" && repository !== "" && typeof id === "string" && id !== "" ? { repository, id } : null;
}

/** An approval of the command shown, or `null` for anything else. */
export function approvalOf(body: unknown): ApproveRepositoryTile | null {
  if (typeof body !== "object" || body === null) return null;
  const { repository, session_id, tile_id, command_hash } = body as Record<string, unknown>;
  const ok = (value: unknown): value is string => typeof value === "string" && value !== "";
  if (!ok(repository) || !ok(tile_id) || !ok(command_hash) || (session_id !== undefined && !ok(session_id))) return null;
  return { repository, tile_id, command_hash, ...(session_id === undefined ? {} : { session_id }) };
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
  ipc.handle(CHANNELS.getRepositoryFlightControls, (_event, repository: unknown, sessionId: unknown) => {
    const root = repositoryOf(repository);
    if (root === null || (sessionId !== undefined && repositoryOf(sessionId) === null)) return unsent;
    return connection()?.flightControls.getRepository(root, sessionId as string | undefined) ?? unsent;
  });
  ipc.handle(CHANNELS.saveRepositoryFlightControls, (_event, repository: unknown, controls: unknown) => {
    const save = repositorySaveOf(repository, controls);
    return save === null ? unsent : (connection()?.flightControls.saveRepository(save.repository, save.controls) ?? unsent);
  });
  ipc.handle(CHANNELS.runSessionShell, (_event, sessionId: unknown, command: unknown, tile: unknown) => {
    const line = shellOf(sessionId, command);
    const from = tileOf(tile);
    return line === null || from === null ? unsent : (connection()?.flightControls.runShell(line.sessionId, line.command, from) ?? unsent);
  });
  ipc.handle(CHANNELS.approveRepositoryTile, (_event, body: unknown) => {
    const approval = approvalOf(body);
    return approval === null ? unsent : (connection()?.flightControls.approve(approval) ?? unsent);
  });
}
