// The channels starting out travels over: a request to Fleet each, and the renderer names no port. What crosses from the
// renderer is not trusted, so each is checked for its shape here and refused as unsent where it has none.

import type { IpcMain } from "electron";
import type { DropRunFact, RunPlace, SetRunMemoryPlace } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

const PLACES: readonly unknown[] = ["this_mac", "repository", "synced", "cloud"] satisfies RunPlace[];

/** The body of a place choice, or `null` for anything else. An absent `synced_folder` stays absent, as Fleet reads it. */
export function placeOf(body: unknown): SetRunMemoryPlace | null {
  if (typeof body !== "object" || body === null) return null;
  const { repository, place, synced_folder } = body as Record<string, unknown>;
  if (!named(repository) || !PLACES.includes(place)) return null;
  if (synced_folder !== undefined && typeof synced_folder !== "string") return null;
  return { repository, place: place as RunPlace, ...(synced_folder === undefined ? {} : { synced_folder }) };
}

/** The four words that name a line to drop, or `null` for anything else. Fleet refuses a kind it does not know. */
export function dropOf(body: unknown): DropRunFact | null {
  if (typeof body !== "object" || body === null) return null;
  const { repository, workspace, kind, command } = body as Record<string, unknown>;
  if (!named(repository) || !named(workspace) || !named(kind) || !named(command)) return null;
  return { repository, workspace, kind: kind as DropRunFact["kind"], command };
}

export function handleOnboarding({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.listImportFolders, () => connection()?.onboarding.importFolders() ?? unsent);
  ipc.handle(CHANNELS.getImportRuns, (_event, path: unknown) =>
    named(path) ? (connection()?.onboarding.importRuns(path) ?? unsent) : unsent,
  );
  // No argument: Fleet runs the install only for a press, and the host sends `requested: true` itself.
  ipc.handle(CHANNELS.installAgentMod, () => connection()?.onboarding.installMod() ?? unsent);
  ipc.handle(CHANNELS.getRunMemory, (_event, repository: unknown) =>
    named(repository) ? (connection()?.onboarding.runMemory(repository) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.getRunMemoryPlace, (_event, repository: unknown) =>
    named(repository) ? (connection()?.onboarding.runMemoryPlace(repository) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.setRunMemoryPlace, (_event, body: unknown) => {
    const place = placeOf(body);
    return place === null ? unsent : (connection()?.onboarding.setRunMemoryPlace(place) ?? unsent);
  });
  ipc.handle(CHANNELS.dropRunFact, (_event, body: unknown) => {
    const drop = dropOf(body);
    return drop === null ? unsent : (connection()?.onboarding.dropRunFact(drop) ?? unsent);
  });
}
