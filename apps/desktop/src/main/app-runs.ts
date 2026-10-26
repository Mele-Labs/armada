// A repository's app run from Bridge, as main holds it: the requests to Fleet, the stream's `app_run.changed` passed on to every
// window, and each server a window reads the output of. Nothing of a run is kept here, since every answer and event carries it whole.
//
// **One socket per window and server**, so each window that opens a log is sent the log as it stands by Fleet itself, and a
// window that closes takes its sockets with it.

import WebSocket from "ws";

import type { AppRun, AppRunDraft, AppRunList, NamedAppRun, Outcome, PinAppRunPort, SaveAppRun, SavedAppRun, ServerMessage, UnpinAppRunPort } from "@armada/protocol";
import type { AppRunActed, AppRunsOf } from "../shared/api/app-runs";
import { ask } from "./request";
import { HOST } from "./runtime-file";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** Where a window's server output goes: its own `webContents`, in the app. */
export type OutputTo = {
  id: number;
  send: (serverId: string, message: ServerMessage) => void;
  /** Called once, when the window goes. */
  onGone: (gone: () => void) => void;
};

const scope = (manifestId: string) => `?manifest_id=${encodeURIComponent(manifestId)}`;

export class AppRunsHost {
  private readonly port: () => number | null;
  private told: (run: AppRun) => void = () => {};
  /** By window, then by server. */
  private readonly following = new Map<number, Map<string, WebSocket>>();

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed run goes: main's, which owns the windows. */
  onChanged(told: (run: AppRun) => void): void {
    this.told = told;
  }

  /** `app_run.changed`. */
  changed(run: AppRun): void {
    this.told(run);
  }

  async list(of: AppRunsOf): Promise<AppRunActed<AppRunList>> {
    const query = "jobId" in of ? `?job_id=${encodeURIComponent(of.jobId)}` : scope(of.manifestId);
    return await this.act<AppRunList>("GET", `/app_runs${query}`);
  }

  async draft(manifestId: string): Promise<AppRunActed<AppRunDraft>> {
    return await this.act<AppRunDraft>("GET", `/app_runs/draft${scope(manifestId)}`);
  }

  async named(act: "start" | "stop" | "restart", manifestId: string, named: NamedAppRun): Promise<AppRunActed<AppRun>> {
    return await this.act<AppRun>("POST", `/app_runs/${act}${scope(manifestId)}`, named);
  }

  async save(manifestId: string, save: SaveAppRun): Promise<AppRunActed<SavedAppRun>> {
    return await this.act<SavedAppRun>("POST", `/app_runs/save${scope(manifestId)}`, save);
  }

  async pin(manifestId: string, pin: PinAppRunPort): Promise<AppRunActed<AppRun>> {
    return await this.act<AppRun>("POST", `/app_runs/pin${scope(manifestId)}`, pin);
  }

  async unpin(manifestId: string, unpin: UnpinAppRunPort): Promise<AppRunActed<AppRun>> {
    return await this.act<AppRun>("POST", `/app_runs/unpin${scope(manifestId)}`, unpin);
  }

  /** Opens `observe_server` for one window. The same server asked again by the same window is the reading it has. */
  follow(to: OutputTo, serverId: string): Outcome {
    const port = this.port();
    if (port === null) return NOT_CONNECTED;
    let held = this.following.get(to.id);
    if (held === undefined) {
      held = new Map();
      this.following.set(to.id, held);
      to.onGone(() => this.forget(to.id));
    }
    if (held.has(serverId)) return { ok: true };
    const socket = new WebSocket(`ws://${HOST}:${port}/servers/${encodeURIComponent(serverId)}/observe`);
    held.set(serverId, socket);
    socket.on("message", (data: WebSocket.RawData) => {
      let message: ServerMessage;
      try {
        message = JSON.parse(String(data)) as ServerMessage;
      } catch {
        return;
      }
      to.send(serverId, message);
    });
    // `following.ts`'s reason: an `error` with nothing listening is thrown by Node.
    socket.on("error", () => {});
    socket.on("close", () => {
      if (this.following.get(to.id)?.get(serverId) === socket) this.following.get(to.id)?.delete(serverId);
    });
    return { ok: true };
  }

  unfollow(windowId: number, serverId: string): void {
    const socket = this.following.get(windowId)?.get(serverId);
    if (socket === undefined) return;
    this.following.get(windowId)?.delete(serverId);
    socket.removeAllListeners();
    socket.on("error", () => {});
    socket.close();
  }

  private forget(windowId: number): void {
    for (const serverId of [...(this.following.get(windowId)?.keys() ?? [])]) this.unfollow(windowId, serverId);
    this.following.delete(windowId);
  }

  private async act<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<AppRunActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as T };
  }
}
