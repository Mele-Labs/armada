// Starting out, answered from memory for the life of the window: one folder of past sessions, a run history for any
// repository that asks, a place that can be chosen and a line that can be dropped. A `cloud` with no address and a `synced` with no folder are
// refused as Fleet refuses them. A scenario that wants the mod's install to fail, or no folders, supplies its own through
// `Scenario.behaves`.

import type { DropRunFact, ImportRuns, RunFact, RunMemory, RunMemoryPlace, WireError } from "@armada/protocol";

import type { OnboardingAnswer, OnboardingApi } from "../../../../shared/api/onboarding";
import type { Slice } from "../fake-context";

const FACTS: RunFact[] = [
  { workspace: ".", kind: "tests", command: "pnpm test", succeeded: true, last_ran: "2026-10-18T16:20:00Z", source: "claude_code_sessions" },
  { workspace: ".", kind: "typecheck", command: "pnpm tsc -b", succeeded: true, last_ran: "2026-10-18T16:22:00Z", source: "jobs" },
  { workspace: "web", kind: "dev_server", command: "pnpm dev", port: 5173, succeeded: true, last_ran: "2026-10-17T09:05:00Z", source: "claude_code_sessions" },
];

const refusal = (code: string, message: string): OnboardingAnswer<never> => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false, outcome: { ok: false, why: "refused", error } };
};

const keyOf = (fact: Pick<RunFact, "workspace" | "kind" | "command">): string => `${fact.workspace}\u0000${fact.kind}\u0000${fact.command}`;

export const onboarding: Slice<OnboardingApi, Record<string, never>> = {
  name: "onboarding",
  state: {},
  api: (_scenario, fleet) => {
    const places = new Map<string, RunMemoryPlace>();
    const dropped = new Map<string, Set<string>>();
    const placeOf = (repository: string): RunMemoryPlace => places.get(repository) ?? { repository, place: "this_mac" };
    const memoryOf = (repository: string): RunMemory => {
      const { place } = placeOf(repository);
      const gone = dropped.get(repository) ?? new Set<string>();
      const workspaces = [...new Set(FACTS.map((fact) => fact.workspace))].map((workspace) => ({
        workspace,
        runs: FACTS.filter((fact) => fact.workspace === workspace && !gone.has(keyOf(fact))).map((fact) =>
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
          folders: [
            { path: "/Users/user/work/storefront", exists: true, last_session_at: "2026-10-18T16:30:00Z", sessions: 14, git: { state: "root" } },
            { path: "/Users/user/work/storefront/web", exists: true, last_session_at: "2026-10-17T09:10:00Z", sessions: 3, git: { state: "inside", root: "/Users/user/work/storefront" } },
            { path: "/Users/user/scratch/notes", exists: true, last_session_at: "2026-10-12T20:00:00Z", sessions: 2, git: { state: "not_a_repository" } },
            { path: "/Users/user/old/removed", exists: false, last_session_at: "2026-09-01T08:00:00Z", sessions: 1, git: { state: "unknown" } },
          ],
        },
      }),
      getImportRuns: async (path) => {
        const value: ImportRuns = { path, root: path, facts: FACTS.filter((fact) => fact.source === "claude_code_sessions") };
        return { ok: true, value };
      },
      installAgentMod: async () => ({ ok: true, value: { result: "installed", said: "Installed armada@armada-local" } }),
      getRunMemory: async (repository) => ({ ok: true, value: memoryOf(repository) }),
      getRunMemoryPlace: async (repository) => ({ ok: true, value: placeOf(repository) }),
      setRunMemoryPlace: async (body) => {
        if (body.place === "cloud" && body.cloud === undefined) return refusal("fleet.run_place_refused", "A cloud place needs the store or a bucket address");
        if (body.place === "synced" && (body.synced_folder ?? "") === "") {
          return refusal("fleet.run_place_refused", "A synced place needs a folder");
        }
        const chosen: RunMemoryPlace = body.place === "synced" || body.place === "cloud" ? body : { repository: body.repository, place: body.place };
        places.set(body.repository, chosen);
        return { ok: true, value: chosen };
      },
      dropRunFact: async (body: DropRunFact) => {
        dropped.set(body.repository, (dropped.get(body.repository) ?? new Set<string>()).add(keyOf(body)));
        return { ok: true, value: memoryOf(body.repository) };
      },
    };
  },
};
