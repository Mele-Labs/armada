// The channels a pull request's notes travel over: a request to Fleet each, and the renderer names no port.

import type { IpcMain } from "electron";
import type { NoteSide, SavePullRequestNote, SendPullRequestNotes } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const text = (value: unknown): value is string => typeof value === "string" && value !== "";

/** A manifest id and a pull request number, or `null` for anything else: what crosses from the renderer is not trusted. */
export function pullRequestOf(manifestId: unknown, number: unknown): { manifestId: string; number: number } | null {
  return text(manifestId) && typeof number === "number" && Number.isInteger(number) && number > 0 ? { manifestId, number } : null;
}

/** A note to save, or `null` for anything else. Which of `path`, `line` and `side` a new note needs is Fleet's to refuse. */
export function noteSaveOf(save: unknown): SavePullRequestNote | null {
  if (typeof save !== "object" || save === null) return null;
  const { id, path, line, side, body } = save as Record<string, unknown>;
  if (typeof body !== "string") return null;
  if (id !== undefined && !text(id)) return null;
  if (path !== undefined && !text(path)) return null;
  if (line !== undefined && !(typeof line === "number" && Number.isInteger(line) && line > 0)) return null;
  if (side !== undefined && side !== "left" && side !== "right") return null;
  return {
    body,
    ...(id === undefined ? {} : { id }),
    ...(path === undefined ? {} : { path }),
    ...(line === undefined ? {} : { line }),
    ...(side === undefined ? {} : { side: side as NoteSide }),
  };
}

/** The notes sent and the Job they went to, or `null` for anything else. */
export function noteSendOf(send: unknown): SendPullRequestNotes | null {
  if (typeof send !== "object" || send === null) return null;
  const { note_ids, session_id, title } = send as Record<string, unknown>;
  if (!Array.isArray(note_ids) || note_ids.length === 0 || !note_ids.every(text) || !text(session_id) || typeof title !== "string") return null;
  return { note_ids: note_ids as string[], session_id, title };
}

export function handlePullRequestNotes({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.listPullRequestNotes, (_event, manifestId: unknown, number: unknown) => {
    const pr = pullRequestOf(manifestId, number);
    return pr === null ? unsent : (connection()?.pullRequestNotes.list(pr.manifestId, pr.number) ?? unsent);
  });
  ipc.handle(CHANNELS.savePullRequestNote, (_event, manifestId: unknown, number: unknown, save: unknown) => {
    const pr = pullRequestOf(manifestId, number);
    const note = noteSaveOf(save);
    return pr === null || note === null ? unsent : (connection()?.pullRequestNotes.save(pr.manifestId, pr.number, note) ?? unsent);
  });
  ipc.handle(CHANNELS.removePullRequestNote, (_event, manifestId: unknown, number: unknown, id: unknown) => {
    const pr = pullRequestOf(manifestId, number);
    return pr === null || !text(id) ? unsent : (connection()?.pullRequestNotes.remove(pr.manifestId, pr.number, id) ?? unsent);
  });
  ipc.handle(CHANNELS.sendPullRequestNotes, (_event, manifestId: unknown, number: unknown, send: unknown) => {
    const pr = pullRequestOf(manifestId, number);
    const sent = noteSendOf(send);
    return pr === null || sent === null ? unsent : (connection()?.pullRequestNotes.send(pr.manifestId, pr.number, sent) ?? unsent);
  });
}
