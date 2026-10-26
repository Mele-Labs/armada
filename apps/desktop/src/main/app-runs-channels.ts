// The channels a repository's app run travels over. Each is one request to Fleet, and the renderer names no port and no path: a run
// is named, never a command, and Fleet reads a Job's checkout from its own record.

import type { IpcMain, WebContents } from "electron";
import type { AppRunDraftProcess, AppRunDraftSetup, NamedAppRun, PinAppRunPort, SaveAppRun, UnpinAppRunPort } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import type { OutputTo } from "./app-runs";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const text = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";
const record = (value: unknown): Record<string, unknown> | null => (typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : null);
const ACTS = new Set(["start", "stop", "restart"]);

/** A run's name and, where it is on a Job's worktree, the Job: nothing else crosses. */
function namedOf(value: unknown): NamedAppRun | null {
  const body = record(value);
  if (body === null || !text(body.name) || (body.job_id !== undefined && !text(body.job_id))) return null;
  return { name: body.name, ...(body.job_id === undefined ? {} : { job_id: body.job_id }) };
}

function processOf(value: unknown): AppRunDraftProcess | null {
  const one = record(value);
  if (one === null || !text(one.name) || !text(one.command)) return null;
  if ((one.dir !== undefined && typeof one.dir !== "string") || (one.port_env !== undefined && typeof one.port_env !== "string")) return null;
  return { name: one.name, command: one.command, ...(one.dir === undefined ? {} : { dir: one.dir }), ...(one.port_env === undefined ? {} : { port_env: one.port_env }) };
}

function setupOf(value: unknown): AppRunDraftSetup | null {
  const one = record(value);
  return one === null || !text(one.name) || !text(one.command) ? null : { name: one.name, command: one.command };
}

/** A confirmed run: its processes, the setup it saves, the Job it then starts on and the run it replaces. */
function saveOf(value: unknown): SaveAppRun | null {
  const body = record(value);
  if (body === null || typeof body.name !== "string" || !Array.isArray(body.processes) || (body.job_id !== undefined && !text(body.job_id))) return null;
  if ((body.setup !== undefined && !Array.isArray(body.setup)) || (body.replaces !== undefined && !text(body.replaces))) return null;
  const processes = body.processes.map(processOf);
  const setup = ((body.setup as unknown[] | undefined) ?? []).map(setupOf);
  if (processes.some((one) => one === null) || setup.some((one) => one === null)) return null;
  return {
    name: body.name,
    processes: processes as AppRunDraftProcess[],
    ...(setup.length === 0 ? {} : { setup: setup as AppRunDraftSetup[] }),
    ...(body.job_id === undefined ? {} : { job_id: body.job_id }),
    ...(body.replaces === undefined ? {} : { replaces: body.replaces }),
  };
}

function unpinOf(value: unknown): UnpinAppRunPort | null {
  const body = record(value);
  if (body === null || !text(body.process) || (body.name !== undefined && !text(body.name)) || (body.job_id !== undefined && !text(body.job_id))) return null;
  return { process: body.process, ...(body.name === undefined ? {} : { name: body.name }), ...(body.job_id === undefined ? {} : { job_id: body.job_id }) };
}

function pinOf(value: unknown): PinAppRunPort | null {
  const unpin = unpinOf(value);
  const port = record(value)?.port;
  if (unpin === null || typeof port !== "number" || !Number.isInteger(port)) return null;
  return { ...unpin, port };
}

/** A window's own `webContents`, as the host sends server output to it. */
const outputTo = (sender: WebContents): OutputTo => ({
  id: sender.id,
  send: (serverId, message) => {
    if (!sender.isDestroyed()) sender.send(CHANNELS.serverOutput, serverId, message);
  },
  onGone: (gone) => sender.once("destroyed", gone),
});

export function handleAppRuns({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.listAppRuns, (_event, of: unknown) => {
    const asked = record(of);
    if (asked !== null && text(asked.manifestId)) return connection()?.appRuns.list({ manifestId: asked.manifestId }) ?? unsent;
    if (asked !== null && text(asked.jobId)) return connection()?.appRuns.list({ jobId: asked.jobId }) ?? unsent;
    return unsent;
  });
  ipc.handle(CHANNELS.getAppRunDraft, (_event, manifestId: unknown) => (text(manifestId) ? (connection()?.appRuns.draft(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.actOnAppRun, (_event, act: unknown, manifestId: unknown, named: unknown) => {
    const body = namedOf(named);
    if (typeof act !== "string" || !ACTS.has(act) || !text(manifestId) || body === null) return unsent;
    return connection()?.appRuns.named(act as "start" | "stop" | "restart", manifestId, body) ?? unsent;
  });
  ipc.handle(CHANNELS.saveAppRun, (_event, manifestId: unknown, save: unknown) => {
    const body = saveOf(save);
    return text(manifestId) && body !== null ? (connection()?.appRuns.save(manifestId, body) ?? unsent) : unsent;
  });
  ipc.handle(CHANNELS.pinAppRunPort, (_event, manifestId: unknown, pin: unknown) => {
    const body = pinOf(pin);
    return text(manifestId) && body !== null ? (connection()?.appRuns.pin(manifestId, body) ?? unsent) : unsent;
  });
  ipc.handle(CHANNELS.unpinAppRunPort, (_event, manifestId: unknown, unpin: unknown) => {
    const body = unpinOf(unpin);
    return text(manifestId) && body !== null ? (connection()?.appRuns.unpin(manifestId, body) ?? unsent) : unsent;
  });
  ipc.handle(CHANNELS.followServer, (event, serverId: unknown) => (text(serverId) ? (connection()?.appRuns.follow(outputTo(event.sender), serverId) ?? unsent.outcome) : unsent.outcome));
  ipc.on(CHANNELS.unfollowServer, (event, serverId: unknown) => {
    if (text(serverId)) connection()?.appRuns.unfollow(event.sender.id, serverId);
  });
}
