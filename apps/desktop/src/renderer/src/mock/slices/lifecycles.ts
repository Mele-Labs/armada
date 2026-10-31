// The lifecycle reads and the two writes of one piece. A scenario that keeps pieces (`draft.pieces`) has them answered as Fleet answers them: those that apply to the repository asked
// about, each with where it is kept, the one that runs and what it replaced. A save or a delete changes them for the life of the window, refused as Fleet refuses what it will not keep.
// A scenario with none answers empty. **A Job answers what the scenario serves of it** (`workspaces-lanes-wire.ts`), the lifecycles, instances and moves a real Job's read carries, and a
// Job that holds nothing answers three empty lists, as Fleet does. The event fires when a served Job moves. A scenario that names Workspaces (`draft.workspace.workspaces`) has them and their autonomy
// layers answered and kept the same way (armada-cwj).

import type { AppliedLifecycle, AppliedRule, LifecycleDefinition, PieceKept, PieceScope, WireError, WorkspaceOfRepositories } from "@armada/protocol";

import type { LifecyclesActed, LifecyclesApi } from "../../../../shared/api/lifecycles";
import type { PiecesSeed } from "../pieces-seed";
import type { Slice } from "../fake-context";
import { servedLifecycles } from "../workspaces-lanes-wire";
import { workspacesWire } from "../workspaces-wire";

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
const TIERS: Readonly<Record<string, number>> = { everywhere: 0, workspace: 1, repository: 2 };
const KINDS = ["tracker_write", "forge_write", "draft", "start_job", "start_watch", "tell_agent", "run_command", "run_agent"];

/** Fleet's own sentence for a Workspace that holds nothing, or is named like a repository it holds. */
function workspaceUnfit(workspace: { id: string; repositories: readonly string[] }): string | undefined {
  if (workspace.repositories.length === 0) return "a Workspace holds at least one repository, and this holds none";
  const same = workspace.repositories.find((one) => one === workspace.id);
  return same === undefined ? undefined : `its id is also the id of \`${same}\`, a repository it holds, and a scope could not say which it meant`;
}

/**
 * What a read answers: those that apply, the one that runs for each id and scope with the ones it replaced marked as such. **Two Workspaces that hold the repository and both set one id at one place are
 * set aside, as Fleet sets them** (`crates/config/src/resolving.rs`): neither is answered, and the repository reads the one above. Autonomy layers are peers and all answered (`setAside` off).
 */
function answered<T extends { id: string; scope: PieceScope }>(all: readonly { piece: T; kept: PieceKept }[], manifestId: string, workspaces: readonly WorkspaceOfRepositories[], setAside = true): { piece: T; kept: PieceKept }[] {
  const applying = all.filter((one) => applies(one.piece.scope, manifestId, workspaces)).map((one) => ({ ...one, kept: { ...one.kept, rank: { ...one.kept.rank, reach: reachOf(one.piece.scope, manifestId) } } }));
  const tied = (one: (typeof applying)[number]) =>
    setAside &&
    one.kept.rank.reach === "workspace" &&
    applying.some((each) => each !== one && each.piece.id === one.piece.id && each.kept.rank.reach === "workspace" && each.kept.rank.source === one.kept.rank.source && each.kept.rank.home === one.kept.rank.home && !sameScope(each.piece.scope, one.piece.scope));
  const here = applying.filter((one) => !tied(one));
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
    const named = scenario.draft?.workspace?.workspaces;
    const wired = named === undefined ? { workspaces: [], autonomy: [] } : workspacesWire(named, scenario.state?.holds?.repositories ?? []);
    const kept = {
      workspaces: structuredClone([...seed.workspaces, ...wired.workspaces]),
      lifecycles: structuredClone([...seed.lifecycles]),
      rules: structuredClone([...seed.rules]),
      autonomy: structuredClone([...wired.autonomy, ...(seed.autonomy ?? [])]),
    };
    return {
      listRepositoryWorkspaces: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, workspaces: kept.workspaces.filter((one) => one.repositories.includes(manifestId)), left_out: [] } }),
      listWorkspaces: async () => ({ ok: true, value: { workspaces: kept.workspaces, left_out: [] } }),
      listLifecycles: async (manifestId) => ({
        ok: true,
        value: { manifest_id: manifestId, lifecycles: answered(kept.lifecycles.map((one) => ({ piece: one.lifecycle, kept: one.kept })), manifestId, kept.workspaces).map(({ piece, kept: at }): AppliedLifecycle => ({ lifecycle: piece, kept: at })), left_out: [] },
      }),
      listLifecycleRules: async (manifestId) => ({
        ok: true,
        value: { manifest_id: manifestId, rules: answered(kept.rules.map((one) => ({ piece: one.rule, kept: one.kept })), manifestId, kept.workspaces).map(({ piece, kept: at }): AppliedRule => ({ rule: piece, kept: at })), left_out: [] },
      }),
      listAutonomyDefaults: async (manifestId) => ({
        ok: true,
        value: {
          manifest_id: manifestId,
          armada: [],
          layers: answered(kept.autonomy.map((one) => ({ piece: one.layer, kept: one.kept })), manifestId, kept.workspaces, false)
            .sort((a, b) => (TIERS[a.kept.rank.reach] ?? 0) - (TIERS[b.kept.rank.reach] ?? 0))
            .map(({ piece, kept: at }) => ({ id: piece.id, scope: piece.scope, kept: at, defaults: KINDS.flatMap((kind) => (piece.defaults[kind] === undefined ? [] : [{ kind, autonomy: piece.defaults[kind]! }])) })),
          left_out: [],
        },
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
        if (piece.kind === "autonomy") {
          const { autonomy } = piece;
          const scope = autonomy.scope ?? ({ kind: "everywhere" } as const);
          const found = kept.autonomy.findIndex((one) => own({ piece: one.layer, kept: one.kept }, autonomy.id, scope));
          if (found >= 0 && overwrite !== true) return refusal("fleet.piece_exists", `${file(autonomy.id)} already declares ${autonomy.id}`);
          kept.autonomy.splice(found >= 0 ? found : kept.autonomy.length, found >= 0 ? 1 : 0, { layer: { id: autonomy.id, scope, defaults: autonomy.defaults }, kept: at(autonomy.id, scope) });
          return done(found >= 0, autonomy.id, scope);
        }
        const { workspace } = piece;
        const bad = workspaceUnfit(workspace);
        if (bad !== undefined) return refusal("fleet.piece_unfit", bad);
        const found = kept.workspaces.findIndex((one) => one.source === "kit" && one.id === workspace.id);
        if (found >= 0 && overwrite !== true) return refusal("fleet.piece_exists", `${file(workspace.id)} already declares ${workspace.id}`);
        // Kit's file runs instead of the one a repository committed under the same id.
        for (const one of kept.workspaces.filter((each) => each.id === workspace.id && each.source !== "kit")) one.overridden_by = "kit";
        const hidden = kept.workspaces.find((one) => one.id === workspace.id && one.source !== "kit")?.source;
        kept.workspaces.splice(found >= 0 ? found : kept.workspaces.length, found >= 0 ? 1 : 0, { ...workspace, source: "kit", path: file(workspace.id), ...(hidden === undefined ? {} : { hides: hidden }) });
        return done(found >= 0, workspace.id, { kind: "everywhere" });
      },
      deleteLifecyclePiece: async ({ kind, id, scope = { kind: "everywhere" } }) => {
        const file = `${KIT}/${FOLDER[kind]}/${id}.toml`;
        const gone = <T extends { id: string; scope: PieceScope }>(all: { piece: T; kept: PieceKept }[]) => all.findIndex((one) => one.kept.rank.source === "kit" && one.piece.id === id && sameScope(one.piece.scope, scope));
        const at =
          kind === "lifecycle"
            ? gone(kept.lifecycles.map((one) => ({ piece: one.lifecycle, kept: one.kept })))
            : kind === "rule"
              ? gone(kept.rules.map((one) => ({ piece: one.rule, kept: one.kept })))
              : kind === "autonomy"
                ? gone(kept.autonomy.map((one) => ({ piece: one.layer, kept: one.kept })))
                : kept.workspaces.findIndex((one) => one.source === "kit" && one.id === id);
        if (at < 0) return refusal("fleet.piece_not_kept", `Your Kit has no ${kind} ${id}`);
        if (kind === "lifecycle") kept.lifecycles.splice(at, 1);
        else if (kind === "rule") kept.rules.splice(at, 1);
        else if (kind === "autonomy") kept.autonomy.splice(at, 1);
        else {
          kept.workspaces.splice(at, 1);
          for (const one of kept.workspaces.filter((each) => each.id === id)) delete one.overridden_by;
        }
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
