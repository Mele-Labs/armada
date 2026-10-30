// The lifecycle reads and the two writes of one piece. A scenario that keeps pieces (`draft.pieces`) has them answered as Fleet answers them: those that apply to the repository asked
// about, each with where it is kept, the one that runs and what it replaced. A save or a delete changes them for the life of the window, refused as Fleet refuses what it will not keep.
// A scenario with none answers empty. **A Job answers what the scenario serves of it** (`workspaces-lanes-wire.ts`), the lifecycles, instances and moves a real Job's read carries, and a
// Job that holds nothing answers three empty lists, as Fleet does. The event fires when a served Job moves.

import type { AppliedLifecycle, AppliedRule, LifecycleDefinition, PieceKept, PieceScope, WireError, WorkspaceOfRepositories } from "@armada/protocol";

import type { LifecyclesActed, LifecyclesApi } from "../../../../shared/api/lifecycles";
import type { PiecesSeed } from "../pieces-seed";
import type { Slice } from "../fake-context";
import { servedLifecycles } from "../workspaces-lanes-wire";

const refusal = (code: string, message: string): LifecyclesActed<never> => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false, outcome: { ok: false, why: "refused", error } };
};

/** A Job's own copy is changed by the scenario that serves the Job (`workspaces-lanes.ts`, through the draft's `copy` acts); the wired mock holds none to change. */
const noCopy = refusal("fleet.job_holds_no_copy", "This Job holds no copy to change");

const KIT = "/Users/user/.armada";
const FOLDER = { lifecycle: "lifecycles", rule: "rules", workspace: "workspaces", autonomy: "autonomy" } as const;

const sameScope = (a: PieceScope, b: PieceScope): boolean => JSON.stringify(a) === JSON.stringify(b);

/** Whether a piece of this scope applies to a repository: everywhere, or within a Workspace that holds it or that repository alone. */
const applies = (scope: PieceScope, manifestId: string, workspaces: readonly WorkspaceOfRepositories[]): boolean =>
  scope.kind === "everywhere" || scope.id === manifestId || workspaces.some((one) => one.id === scope.id && one.repositories.includes(manifestId));

/** Where a piece of one scope ranks for a repository, as Fleet reports it: how narrowly the scope names the repository. */
const reachOf = (scope: PieceScope, manifestId: string): string => (scope.kind === "everywhere" ? "everywhere" : scope.id === manifestId ? "repository" : "workspace");

const PLACE: Readonly<Record<string, number>> = { kit: 0, repository: 1, armada: 2 };

/** What a read answers: those that apply, the one that runs for each id and scope with the ones it replaced marked as such. */
function answered<T extends { id: string; scope: PieceScope }>(all: readonly { piece: T; kept: PieceKept }[], manifestId: string, workspaces: readonly WorkspaceOfRepositories[]): { piece: T; kept: PieceKept }[] {
  const here = all.filter((one) => applies(one.piece.scope, manifestId, workspaces)).map((one) => ({ ...one, kept: { ...one.kept, rank: { ...one.kept.rank, reach: reachOf(one.piece.scope, manifestId) } } }));
  return here.map((one) => {
    const best = here
      .filter((each) => each.piece.id === one.piece.id && sameScope(each.piece.scope, one.piece.scope))
      .sort((a, b) => (PLACE[a.kept.rank.source] ?? 9) - (PLACE[b.kept.rank.source] ?? 9))[0]!;
    return best === one ? one : { ...one, kept: { ...one.kept, overridden_by: best.kept.rank } };
  });
}

/** What a lifecycle must be for Fleet's parser to keep it: its moves and ends name states it has. */
function unfit(lifecycle: LifecycleDefinition): string | undefined {
  const ids = new Set(lifecycle.states.map((one) => one.id));
  if (ids.size !== lifecycle.states.length) return `${lifecycle.id} names a state twice`;
  const missing = [...lifecycle.transitions.flatMap((one) => [one.from, one.to]), ...lifecycle.ends].find((one) => !ids.has(one));
  return missing === undefined ? undefined : `${lifecycle.id} names ${missing}, which is not one of its states`;
}

export const lifecycles: Slice<LifecyclesApi, Record<string, never>> = {
  name: "lifecycles",
  state: {},
  api: (scenario) => {
    const seed: PiecesSeed = scenario.draft?.pieces ?? { workspaces: [], lifecycles: [], rules: [] };
    const kept = { workspaces: structuredClone([...seed.workspaces]), lifecycles: structuredClone([...seed.lifecycles]), rules: structuredClone([...seed.rules]) };
    return {
      listRepositoryWorkspaces: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, workspaces: kept.workspaces.filter((one) => one.repositories.includes(manifestId)), left_out: [] } }),
      listLifecycles: async (manifestId) => ({
        ok: true,
        value: { manifest_id: manifestId, lifecycles: answered(kept.lifecycles.map((one) => ({ piece: one.lifecycle, kept: one.kept })), manifestId, kept.workspaces).map(({ piece, kept: at }): AppliedLifecycle => ({ lifecycle: piece, kept: at })), left_out: [] },
      }),
      listLifecycleRules: async (manifestId) => ({
        ok: true,
        value: { manifest_id: manifestId, rules: answered(kept.rules.map((one) => ({ piece: one.rule, kept: one.kept })), manifestId, kept.workspaces).map(({ piece, kept: at }): AppliedRule => ({ rule: piece, kept: at })), left_out: [] },
      }),
      listKeepDrafts: async (manifestId) => ({
        ok: true,
        value: { manifest_id: manifestId, drafts: (seed.keeps ?? []).filter((one) => one.manifestId === manifestId).map(({ kind, id, text }) => ({ kind, id, text })) },
      }),
      saveLifecyclePiece: async ({ piece, overwrite }) => {
        const file = (id: string) => `${KIT}/${FOLDER[piece.kind]}/${id}.toml`;
        const at = (id: string, scope: PieceScope): PieceKept => ({ rank: { reach: scope.kind === "everywhere" ? "everywhere" : "repository", source: "kit" }, path: file(id) });
        const own = (one: { piece: { id: string; scope: PieceScope }; kept: PieceKept }, id: string, scope: PieceScope) => one.kept.rank.source === "kit" && one.piece.id === id && sameScope(one.piece.scope, scope);
        const done = (replaced: boolean, id: string, scope: PieceScope) => ({ ok: true as const, value: { kind: piece.kind, id, scope, file: file(id), replaced } });
        if (piece.kind === "lifecycle") {
          const { lifecycle } = piece;
          const bad = unfit(lifecycle);
          if (bad !== undefined) return refusal("fleet.piece_unfit", bad);
          const found = kept.lifecycles.findIndex((one) => own({ piece: one.lifecycle, kept: one.kept }, lifecycle.id, lifecycle.scope));
          if (found >= 0 && overwrite !== true) return refusal("fleet.piece_exists", `${file(lifecycle.id)} already declares ${lifecycle.id}`);
          kept.lifecycles.splice(found >= 0 ? found : kept.lifecycles.length, found >= 0 ? 1 : 0, { lifecycle, kept: at(lifecycle.id, lifecycle.scope) });
          return done(found >= 0, lifecycle.id, lifecycle.scope);
        }
        if (piece.kind === "rule") {
          const { rule } = piece;
          const found = kept.rules.findIndex((one) => own({ piece: one.rule, kept: one.kept }, rule.id, rule.scope));
          if (found >= 0 && overwrite !== true) return refusal("fleet.piece_exists", `${file(rule.id)} already declares ${rule.id}`);
          kept.rules.splice(found >= 0 ? found : kept.rules.length, found >= 0 ? 1 : 0, { rule, kept: at(rule.id, rule.scope) });
          return done(found >= 0, rule.id, rule.scope);
        }
        return refusal("fleet.piece_unfit", "This window keeps no Workspace or autonomy default");
      },
      deleteLifecyclePiece: async ({ kind, id, scope = { kind: "everywhere" } }) => {
        const file = `${KIT}/${FOLDER[kind]}/${id}.toml`;
        const gone = <T extends { id: string; scope: PieceScope }>(all: { piece: T; kept: PieceKept }[]) => all.findIndex((one) => one.kept.rank.source === "kit" && one.piece.id === id && sameScope(one.piece.scope, scope));
        const at = kind === "lifecycle" ? gone(kept.lifecycles.map((one) => ({ piece: one.lifecycle, kept: one.kept }))) : kind === "rule" ? gone(kept.rules.map((one) => ({ piece: one.rule, kept: one.kept }))) : -1;
        if (at < 0) return refusal("fleet.piece_not_kept", `Your Kit has no ${kind} ${id}`);
        if (kind === "lifecycle") kept.lifecycles.splice(at, 1);
        else kept.rules.splice(at, 1);
        return { ok: true, value: { kind, id, scope, file } };
      },
      getJobLifecycles: async (sessionId) => ({ ok: true, value: servedLifecycles.get(sessionId) ?? { session_id: sessionId, lifecycles: [], instances: [], moves: [] } }),
      onJobLifecyclesChanged: (onChanged) => servedLifecycles.listen(onChanged),
      onKeepDraftsChanged: () => () => undefined,
      getJobCopy: async () => noCopy,
      changeSessionSteps: async () => noCopy,
      changeJobCopy: async () => noCopy,
      keepJobWorkflow: async () => noCopy,
      keepJobPiece: async () => noCopy,
    };
  },
};
