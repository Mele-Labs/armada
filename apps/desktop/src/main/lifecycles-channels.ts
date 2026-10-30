// The channels Workspaces, lifecycles and rules travel over. Each is one request to Fleet; the renderer names no port,
// and a request that names no repository, Job or piece is refused here.

import type { IpcMain } from "electron";

import type { DeleteLifecyclePiece, SaveLifecyclePiece } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

const KINDS = ["lifecycle", "rule", "workspace", "autonomy"];
const isKind = (value: unknown): value is DeleteLifecyclePiece["kind"] => typeof value === "string" && KINDS.includes(value);
const isScope = (value: unknown): boolean =>
  value === undefined ||
  (typeof value === "object" && value !== null && ((value as { kind?: unknown }).kind === "everywhere" || ((value as { kind?: unknown }).kind === "within" && named((value as { id?: unknown }).id))));

/** One piece to save, or `null` for anything else. What the piece says is Fleet's parser's to refuse. */
export function saveOf(body: unknown): SaveLifecyclePiece | null {
  if (typeof body !== "object" || body === null) return null;
  const { piece, overwrite } = body as Record<string, unknown>;
  if (typeof piece !== "object" || piece === null || !isKind((piece as { kind?: unknown }).kind)) return null;
  if (overwrite !== undefined && typeof overwrite !== "boolean") return null;
  return { piece: piece as SaveLifecyclePiece["piece"], ...(overwrite === undefined ? {} : { overwrite }) };
}

/** One piece to delete, or `null` for anything else. */
export function deleteOf(body: unknown): DeleteLifecyclePiece | null {
  if (typeof body !== "object" || body === null) return null;
  const { kind, id, scope } = body as Record<string, unknown>;
  if (!isKind(kind) || !named(id) || !isScope(scope)) return null;
  return { kind, id, ...(scope === undefined ? {} : { scope: scope as DeleteLifecyclePiece["scope"] }) };
}

export function handleLifecycles({ ipc, connection }: Hosts): void {
  const host = () => connection()?.lifecycles;
  ipc.handle(CHANNELS.listRepositoryWorkspaces, (_event, manifestId: unknown) => (named(manifestId) ? (host()?.workspaces(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.listLifecycles, (_event, manifestId: unknown) => (named(manifestId) ? (host()?.lifecycles(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.listLifecycleRules, (_event, manifestId: unknown) => (named(manifestId) ? (host()?.rules(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.saveLifecyclePiece, (_event, body: unknown) => {
    const save = saveOf(body);
    return save === null ? unsent : (host()?.save(save) ?? unsent);
  });
  ipc.handle(CHANNELS.deleteLifecyclePiece, (_event, body: unknown) => {
    const remove = deleteOf(body);
    return remove === null ? unsent : (host()?.remove(remove) ?? unsent);
  });
  ipc.handle(CHANNELS.getJobLifecycles, (_event, sessionId: unknown) => (named(sessionId) ? (host()?.job(sessionId) ?? unsent) : unsent));
}
