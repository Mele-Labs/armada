// Workspaces, lifecycles and rules, as main asks Fleet for them: one request per read or write, and the stream's
// `job_lifecycles.changed`, passed on to every window. Nothing is kept here, since the event carries a Job's whole.

import type {
  DeleteLifecyclePiece,
  JobLifecycles,
  LifecyclePieceDeleted,
  LifecyclePieceSaved,
  Outcome,
  RepositoryLifecycleRules,
  RepositoryLifecycles,
  RepositoryWorkspaces,
  SaveLifecyclePiece,
} from "@armada/protocol";
import type { LifecyclesActed } from "../shared/api/lifecycles";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class LifecyclesHost {
  private readonly port: () => number | null;
  private told: (held: JobLifecycles) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed Job goes: main's, which owns the windows. */
  onChanged(told: (held: JobLifecycles) => void): void {
    this.told = told;
  }

  /** `job_lifecycles.changed`. */
  changed(held: JobLifecycles): void {
    this.told(held);
  }

  /** `GET /workspaces?manifest_id=`. */
  workspaces(manifestId: string): Promise<LifecyclesActed<RepositoryWorkspaces>> {
    return this.read(`/workspaces?manifest_id=${encodeURIComponent(manifestId)}`);
  }

  /** `GET /lifecycles?manifest_id=`. */
  lifecycles(manifestId: string): Promise<LifecyclesActed<RepositoryLifecycles>> {
    return this.read(`/lifecycles?manifest_id=${encodeURIComponent(manifestId)}`);
  }

  /** `GET /lifecycles/rules?manifest_id=`. */
  rules(manifestId: string): Promise<LifecyclesActed<RepositoryLifecycleRules>> {
    return this.read(`/lifecycles/rules?manifest_id=${encodeURIComponent(manifestId)}`);
  }

  /** `POST /lifecycles/pieces/save`. */
  save(body: SaveLifecyclePiece): Promise<LifecyclesActed<LifecyclePieceSaved>> {
    return this.send("/lifecycles/pieces/save", body);
  }

  /** `POST /lifecycles/pieces/delete`. */
  remove(body: DeleteLifecyclePiece): Promise<LifecyclesActed<LifecyclePieceDeleted>> {
    return this.send("/lifecycles/pieces/delete", body);
  }

  /** `GET /sessions/lifecycles?session_id=`. */
  job(sessionId: string): Promise<LifecyclesActed<JobLifecycles>> {
    return this.read(`/sessions/lifecycles?session_id=${encodeURIComponent(sessionId)}`);
  }

  private read<T>(path: string): Promise<LifecyclesActed<T>> {
    return this.ask("GET", path);
  }

  private send<T>(path: string, body: unknown): Promise<LifecyclesActed<T>> {
    return this.ask("POST", path, body);
  }

  private async ask<T>(method: "GET" | "POST", path: string, body?: unknown): Promise<LifecyclesActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }
}
