// Workspaces, lifecycles and rules, as main asks Fleet for them: one request per read or write, and the stream's
// `job_lifecycles.changed`, passed on to every window. Nothing is kept here, since the event carries a Job's whole.

import type {
  ChangeJobCopy,
  ChangeSessionSteps,
  DeleteLifecyclePiece,
  JobCopy,
  JobLifecycles,
  JobPieceProposed,
  KeepJobPiece,
  LifecyclePieceDeleted,
  LifecyclePieceSaved,
  Outcome,
  RepositoryLifecycleRules,
  RepositoryLifecycles,
  RepositoryWorkspaces,
  SaveLifecyclePiece,
  WorkflowChange,
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

  /** `GET /sessions/copy?session_id=`. */
  copy(sessionId: string): Promise<LifecyclesActed<JobCopy>> {
    return this.read(`/sessions/copy?session_id=${encodeURIComponent(sessionId)}`);
  }

  /** `POST /sessions/workflow/steps`. The Job's lanes change by `job_lifecycles.changed` and its row by `session.changed`. */
  steps(change: ChangeSessionSteps): Promise<LifecyclesActed<JobCopy>> {
    return this.send("/sessions/workflow/steps", change);
  }

  /** `POST /sessions/copy`. */
  piece(change: ChangeJobCopy): Promise<LifecyclesActed<JobCopy>> {
    return this.send("/sessions/copy", change);
  }

  /** `POST /sessions/copy/keep`. Saves nothing: the change waits on the owner's Approve. */
  keep(sessionId: string): Promise<LifecyclesActed<WorkflowChange>> {
    return this.send("/sessions/copy/keep", { session_id: sessionId });
  }

  /** `POST /sessions/copy/keep_piece`. Saves nothing: the proposal waits on the owner's Approve on the Job's draft. */
  keepPiece(body: KeepJobPiece): Promise<LifecyclesActed<JobPieceProposed>> {
    return this.send("/sessions/copy/keep_piece", body);
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
