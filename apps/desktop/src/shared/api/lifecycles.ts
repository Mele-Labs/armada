// Workspaces, lifecycles and rules, and where a Job's tickets, pull requests, deploys and started Jobs stand: the four reads,
// and the event that carries a Job's whole. Beside them, the two writes of one piece in Kit's folder; a piece in a repository's goes by
// `commitToRepository`. Nothing here moves an instance. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  AllWorkspaces,
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
  RepositoryAutonomy,
  RepositoryLifecycleRules,
  RepositoryLifecycles,
  RepositoryWorkspaces,
  SaveLifecyclePiece,
  WorkflowChange,
} from "@armada/protocol";

/** What a read came to: what Fleet answered with, or the refusal to word. */
export type LifecyclesActed<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type LifecyclesApi = {
  /** The Workspaces (sets of repositories) that hold one repository, and the files Fleet set aside. `GET /workspaces`. */
  listRepositoryWorkspaces: (manifestId: string) => Promise<LifecyclesActed<RepositoryWorkspaces>>;
  /** Every Workspace Fleet can see, whichever repositories it holds, and the files Fleet set aside. `GET /workspaces/all`. */
  listWorkspaces: () => Promise<LifecyclesActed<AllWorkspaces>>;
  /** The lifecycles that apply to one repository, where each was kept, and the files set aside. `GET /lifecycles`. */
  listLifecycles: (manifestId: string) => Promise<LifecyclesActed<RepositoryLifecycles>>;
  /** The rules that apply to one repository, where each was kept, and the files set aside. `GET /lifecycles/rules`. */
  listLifecycleRules: (manifestId: string) => Promise<LifecyclesActed<RepositoryLifecycleRules>>;
  /** Armada's default for every kind of act and each layer of autonomy defaults that applies to one repository, unresolved, with where each was kept. `GET /autonomy`. */
  listAutonomyDefaults: (manifestId: string) => Promise<LifecyclesActed<RepositoryAutonomy>>;
  /** `POST /lifecycles/pieces/save`: write one piece to Kit's folder, checked by Fleet's own parser first. */
  saveLifecyclePiece: (body: SaveLifecyclePiece) => Promise<LifecyclesActed<LifecyclePieceSaved>>;
  /** `POST /lifecycles/pieces/delete`: delete the Kit file that declares one piece. */
  deleteLifecyclePiece: (body: DeleteLifecyclePiece) => Promise<LifecyclesActed<LifecyclePieceDeleted>>;
  /** What one Job holds and where it stands, with its moves oldest first. A Job that holds nothing answers three empty lists. */
  getJobLifecycles: (sessionId: string) => Promise<LifecyclesActed<JobLifecycles>>;
  /** `job_lifecycles.changed`: one Job's lifecycles, whole, after an instance was taken on or moved. */
  onJobLifecyclesChanged: (onChanged: (held: JobLifecycles) => void) => () => void;
  /** What one Job holds of its workflow, lifecycles and rules, and what was changed in it. `GET /sessions/copy`. */
  getJobCopy: (sessionId: string) => Promise<LifecyclesActed<JobCopy>>;
  /** A step added, moved, reworded or dropped in the Job's own copy of its workflow. It applies to that Job at once. `POST /sessions/workflow/steps`. */
  changeSessionSteps: (change: ChangeSessionSteps) => Promise<LifecyclesActed<JobCopy>>;
  /** A lifecycle or a rule put in the Job's own copy, or taken out of it. `POST /sessions/copy`. */
  changeJobCopy: (change: ChangeJobCopy) => Promise<LifecyclesActed<JobCopy>>;
  /** The Job's copy of its workflow proposed for every Job: a ready change that waits on the owner's Approve. `POST /sessions/copy/keep`. */
  keepJobWorkflow: (sessionId: string) => Promise<LifecyclesActed<WorkflowChange>>;
  /** One rule or lifecycle the Job changed proposed for every Job, as a file in Kit within the Job's repository: it waits on the owner's Approve on the Job's draft. `POST /sessions/copy/keep_piece`. */
  keepJobPiece: (body: KeepJobPiece) => Promise<LifecyclesActed<JobPieceProposed>>;
};

export const LIFECYCLES_CHANNELS = {
  listRepositoryWorkspaces: "bridge:list-repository-workspaces",
  listWorkspaces: "bridge:list-workspaces",
  listLifecycles: "bridge:list-lifecycles",
  listLifecycleRules: "bridge:list-lifecycle-rules",
  listAutonomyDefaults: "bridge:list-autonomy-defaults",
  saveLifecyclePiece: "bridge:save-lifecycle-piece",
  deleteLifecyclePiece: "bridge:delete-lifecycle-piece",
  getJobLifecycles: "bridge:get-job-lifecycles",
  getJobCopy: "bridge:get-job-copy",
  changeSessionSteps: "bridge:change-session-steps",
  changeJobCopy: "bridge:change-job-copy",
  keepJobWorkflow: "bridge:keep-job-workflow",
  keepJobPiece: "bridge:keep-job-piece",
  /** Main to every window, not a request. */
  jobLifecyclesChanged: "bridge:job-lifecycles-changed",
} as const;
