// Workspaces, lifecycles and rules, and where a Job's tickets, pull requests, deploys and started Jobs stand: the four reads,
// and the event that carries a Job's whole. Reads only; nothing here saves a piece or moves an instance. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { JobLifecycles, Outcome, RepositoryLifecycleRules, RepositoryLifecycles, RepositoryWorkspaces } from "@armada/protocol";

/** What a read came to: what Fleet answered with, or the refusal to word. */
export type LifecyclesActed<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type LifecyclesApi = {
  /** The Workspaces (sets of repositories) that hold one repository, and the files Fleet set aside. `GET /workspaces`. */
  listRepositoryWorkspaces: (manifestId: string) => Promise<LifecyclesActed<RepositoryWorkspaces>>;
  /** The lifecycles that apply to one repository, where each was kept, and the files set aside. `GET /lifecycles`. */
  listLifecycles: (manifestId: string) => Promise<LifecyclesActed<RepositoryLifecycles>>;
  /** The rules that apply to one repository, where each was kept, and the files set aside. `GET /lifecycles/rules`. */
  listLifecycleRules: (manifestId: string) => Promise<LifecyclesActed<RepositoryLifecycleRules>>;
  /** What one Job holds and where it stands, with its moves oldest first. A Job that holds nothing answers three empty lists. */
  getJobLifecycles: (sessionId: string) => Promise<LifecyclesActed<JobLifecycles>>;
  /** `job_lifecycles.changed`: one Job's lifecycles, whole, after an instance was taken on or moved. */
  onJobLifecyclesChanged: (onChanged: (held: JobLifecycles) => void) => () => void;
};

export const LIFECYCLES_CHANNELS = {
  listRepositoryWorkspaces: "bridge:list-repository-workspaces",
  listLifecycles: "bridge:list-lifecycles",
  listLifecycleRules: "bridge:list-lifecycle-rules",
  getJobLifecycles: "bridge:get-job-lifecycles",
  /** Main to every window, not a request. */
  jobLifecyclesChanged: "bridge:job-lifecycles-changed",
} as const;
