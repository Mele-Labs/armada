// What the Workflows page asks of Fleet for the Workspace's lifecycles and rules: the reads for the repository it is on and the rest of its Workspace, and the acts a change
// to them comes to (`write.ts`). **Fleet is the authority on every refusal**: a piece it will not keep comes back in its own words and nothing is written.

import type { Outcome, PieceToCommit } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeApi } from "../../../shared/api";
import type { PiecesRead } from "./levels";
import type { Op, PieceId } from "./write";

/** The calls on Bridge's api the page uses for pieces. */
export type PiecesApi = Pick<BridgeApi, "commitToRepository" | "deleteLifecyclePiece" | "listLifecycleRules" | "listLifecycles" | "listRepositoryWorkspaces" | "saveLifecyclePiece">;

/** A repository with a Manifest: the id its pieces are read under, the name its tab carries and the root a commit goes to. */
export type Served = { manifestId: string; name: string; root: string };

export type Read = { ok: true; read: PiecesRead } | { ok: false; outcome: Outcome };

/** Everything Fleet keeps for a repository and the others of the Workspace that runs for it. The first Workspace that runs is the one drawn. */
export async function readPieces(api: PiecesApi, current: string, served: readonly Served[]): Promise<Read> {
  const where = await api.listRepositoryWorkspaces(current);
  if (!where.ok) return where;
  const workspace = where.value.workspaces.find((one) => one.overridden_by === undefined && one.repositories.includes(current));
  const wanted = [current, ...(workspace?.repositories ?? []).filter((one) => one !== current)];
  const repositories: PiecesRead["repositories"][number][] = [];
  for (const id of wanted) {
    const name = served.find((one) => one.manifestId === id)?.name;
    // A repository the window does not serve has no root to commit to and no name to be a tab by.
    if (name === undefined && id !== current) continue;
    const [lifecycles, rules] = await Promise.all([api.listLifecycles(id), api.listLifecycleRules(id)]);
    const failed = !lifecycles.ok ? lifecycles : !rules.ok ? rules : undefined;
    if (failed !== undefined) {
      if (id === current) return { ok: false, outcome: failed.outcome };
      continue;
    }
    if (!lifecycles.ok || !rules.ok) continue;
    repositories.push({ id, name: name ?? id, lifecycles: lifecycles.value.lifecycles, rules: rules.value.rules });
  }
  return { ok: true, read: { ...(workspace === undefined ? {} : { workspace }), repositories } };
}

/** What a run of acts came to: done, with the pieces a repository holds that a pull request is removing, or the words that stopped it. `old` is a Fleet that does not serve the route, which is no error to say. */
export type Ran = { ok: true; removing: PieceId[] } | { ok: false; said: string; old?: true };

/** A route this Fleet does not serve answers 404 with no refusal in it. */
const missing = (outcome: Outcome): boolean => !outcome.ok && outcome.why === "transport" && outcome.fault.why === "unanswerable" && outcome.fault.status === 404;

const toCommit = ({ kind, id, scope }: PieceId): PieceToCommit => ({ kind: kind === "rule" ? "lifecycle_rule" : "lifecycle", id, scope });

/** The repository whose folder holds a file. */
export const rootOf = (path: string, served: readonly Served[]): string | undefined => served.find((one) => path.startsWith(`${one.root}/`))?.root;

/** Does the saves and deletes of `ops`, in order, stopping at the first Fleet refuses. */
export async function runOps(api: PiecesApi, ops: readonly Op[], served: readonly Served[]): Promise<Ran> {
  const removing: PieceId[] = [];
  for (const op of ops) {
    let answer: { ok: true } | { ok: false; outcome: Outcome };
    if (op.op === "save") answer = await api.saveLifecyclePiece({ piece: op.piece, ...(op.overwrite ? { overwrite: true } : {}) });
    else if (op.op === "delete") answer = await api.deleteLifecyclePiece({ kind: op.id.kind, id: op.id.id, scope: op.id.scope });
    else {
      const repository = rootOf(op.path, served);
      if (repository === undefined) return { ok: false, said: "The repository that holds it is not open here" };
      answer = await api.commitToRepository({ repository, piece: toCommit(op.id), remove: true });
      // A piece a repository holds goes when the pull request that removes it merges, so a read still shows it until then.
      removing.push(op.id);
    }
    if (!answer.ok) return missing(answer.outcome) ? { ok: false, said: "", old: true } : { ok: false, said: refusalWords(answer.outcome) };
  }
  return { ok: true, removing };
}
