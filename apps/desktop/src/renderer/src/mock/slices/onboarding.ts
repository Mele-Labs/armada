// Starting out, answered from memory for the life of the window: the folders Claude Code kept sessions for, a run history
// for any repository that asks, a place that can be chosen and a line that can be dropped. A `cloud` with no address and a
// `synced` with no folder are refused as Fleet refuses them. A scenario draws its own run history with `draft.runFacts`, and
// one that wants the mod's install to fail, or no folders, supplies its own through `Scenario.behaves`.
// A repository kept on this Mac refuses any other place, and marking it moves its place to This Mac, as Fleet does.

import type { DropRunFact, ImportGit, ImportRuns, RunFact, RunMemory, RunMemoryPlace, WireError } from "@armada/protocol";

import type { OnboardingAnswer, OnboardingApi } from "../../../../shared/api/onboarding";
import type { Slice } from "../fake-context";

/** How a repository runs unless the scenario says: the tests and the dev server, as the folder's past sessions ran them. */
const FACTS: readonly RunFact[] = [
  { workspace: ".", kind: "tests", command: "pnpm test", succeeded: true, last_ran: "2026-10-18T16:20:00Z", source: "claude_code_sessions" },
  { workspace: ".", kind: "dev_server", command: "pnpm dev", port: 5173, succeeded: true, last_ran: "2026-10-18T16:22:00Z", source: "claude_code_sessions" },
];

/** The folders Claude Code is said to hold sessions for, by hours since the last, under the folder most of them share. */
const HOME = "/Users/user/code";
const PAST: readonly { path: string; hours: number; sessions: number; git: ImportGit }[] = [
  { path: `${HOME}/my-app`, hours: 2, sessions: 14, git: { state: "root" } },
  { path: `${HOME}/api`, hours: 3 * 24, sessions: 6, git: { state: "root" } },
  { path: `${HOME}/site`, hours: 12 * 24, sessions: 3, git: { state: "root" } },
  { path: `${HOME}/scratch`, hours: 41 * 24, sessions: 2, git: { state: "not_a_repository" } },
];

const refusal = (code: string, message: string): OnboardingAnswer<never> => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false, outcome: { ok: false, why: "refused", error } };
};

const keyOf = (fact: Pick<RunFact, "workspace" | "kind" | "command">): string => `${fact.workspace}\u0000${fact.kind}\u0000${fact.command}`;

export const onboarding: Slice<OnboardingApi, Record<string, never>> = {
  name: "onboarding",
  state: {},
  api: (scenario, fleet) => {
    const facts = scenario.draft?.runFacts ?? FACTS;
    const places = new Map<string, RunMemoryPlace>();
    const dropped = new Map<string, Set<string>>();
    const kept = new Set<string>();
    // The default history was seen in Claude Code's sessions once an import read them, and by Armada's own Jobs before that.
    let imported = false;
    const placeOf = (repository: string): RunMemoryPlace => places.get(repository) ?? { repository, place: "this_mac" };
    const memoryOf = (repository: string): RunMemory => {
      const { place } = placeOf(repository);
      const gone = dropped.get(repository) ?? new Set<string>();
      const workspaces = [...new Set(facts.map((fact) => fact.workspace))].map((workspace) => ({
        workspace,
        runs: facts.filter((fact) => fact.workspace === workspace && !gone.has(keyOf(fact))).map((seen) => ({ ...seen, source: facts === FACTS && !imported ? "jobs" : seen.source })).map((fact) =>
          place === "repository"
            ? { ...fact, state: "waiting" as const, why: "No Job branch to write it on yet" }
            : { ...fact, state: "kept" as const, where: place },
        ),
      }));
      return { repository, place, workspaces };
    };
    return {
      createProject: async (parent, name) => {
        const root = `${parent.replace(/\/$/, "")}/${name}`;
        const one = { root, records_root: `/records/${name}` };
        const held = fleet.state().holds;
        if (!(held.repositories ?? []).some((repo) => repo.root === root)) {
          fleet.publish({ holds: { ...held, repositories: [...(held.repositories ?? []), one] } });
        }
        return { state: "located", repository: one };
      },
      listImportFolders: async () => ({
        ok: true,
        value: {
          agent: "Claude Code",
          folders: PAST.map((one) => ({
            path: one.path,
            exists: true,
            last_session_at: new Date(Date.now() - one.hours * 3_600_000).toISOString(),
            sessions: one.sessions,
            git: one.git,
          })),
        },
      }),
      getImportRuns: async (path) => {
        imported = true;
        const value: ImportRuns = { path, root: path, facts: facts.filter((fact) => fact.source === "claude_code_sessions") };
        return { ok: true, value };
      },
      installAgentMod: async () => ({ ok: true, value: { result: "installed", said: "Installed armada@armada-local" } }),
      getRunMemory: async (repository) => ({ ok: true, value: memoryOf(repository) }),
      getRunMemoryPlace: async (repository) => ({ ok: true, value: placeOf(repository) }),
      setRunMemoryPlace: async (body) => {
        if (kept.has(body.repository) && body.place !== "this_mac") return refusal("fleet.run_place_kept_here", "This repository is kept on this Mac");
        if (body.place === "cloud" && body.cloud === undefined) return refusal("fleet.run_place_refused", "A cloud place needs the store or a bucket address");
        if (body.place === "synced" && (body.synced_folder ?? "") === "") {
          return refusal("fleet.run_place_refused", "A synced place needs a folder");
        }
        const chosen: RunMemoryPlace = body.place === "synced" || body.place === "cloud" ? body : { repository: body.repository, place: body.place };
        places.set(body.repository, chosen);
        return { ok: true, value: chosen };
      },
      getKeptOnThisMac: async (repository) => ({ ok: true, value: { repository, kept: kept.has(repository) } }),
      setKeptOnThisMac: async (body) => {
        if (body.kept) {
          kept.add(body.repository);
          places.set(body.repository, { repository: body.repository, place: "this_mac" });
        } else kept.delete(body.repository);
        return { ok: true, value: { repository: body.repository, kept: kept.has(body.repository) } };
      },
      dropRunFact: async (body: DropRunFact) => {
        dropped.set(body.repository, (dropped.get(body.repository) ?? new Set<string>()).add(keyOf(body)));
        return { ok: true, value: memoryOf(body.repository) };
      },
    };
  },
};
