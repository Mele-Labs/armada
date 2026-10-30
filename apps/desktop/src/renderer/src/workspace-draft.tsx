// The Workspace the Workflows page draws as lanes where a scenario hands one (`draft.workspace`): this keeps it for the window, changed by the acts on it, and its proposals answer
// what an agent is told. Mock-only. A window without one draws the lifecycles and rules Fleet keeps (`lifecycle-editing/`), and a repository with none draws the column of steps.
// A seed with `workspaces` also keeps the Workspaces themselves and the autonomy each level sets (armada-cwj, mock takes), which the page reads for the repository it is on.

import { createContext, useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { Autonomy, AutonomyKind, AutonomyLayer, AutonomyTake, Repository, Rule, WorkspaceEditing, WorkspaceTake, Workspace } from "@armada/components";

/** A Workspace as the mock keeps it: its name, the repositories in it by id, and where it is kept. */
export type WorkspaceRecord = { id: string; name: string; repositories: readonly string[]; kept: "mac" | "repository" };

/** What a scenario hands: the Workspace, and the changes an agent comes back with when it is asked for one in these words. */
export type WorkspaceSeed = {
  workspace: Workspace;
  proposals: readonly { asked: string; rule: Rule }[];
  /** Every repository Fleet serves, and the Workspaces they are in. The Workspace above holds the pieces (lifecycles, rules) all of them share. */
  workspaces?: { served: readonly Repository[]; records: readonly WorkspaceRecord[] };
};

/** The Workspace as the window holds it. */
export type WorkspaceHeld = {
  workspace: Workspace;
  /** The Workspace after a change by hand. */
  change: (next: Workspace) => void;
  /** The rule an agent comes back with for these words, or none where they are not about rules. */
  ask: (text: string) => Rule | undefined;
  add: (rule: Rule) => void;
  /** The same, as the page reads it for the repository it is on, with what the Workspace and autonomy controls need. Absent where the seed keeps no Workspaces. */
  at?: (repository: string) => WorkspaceHeld;
  editing?: Pick<WorkspaceEditing, "workspaces" | "autonomy">;
};

const Held = createContext<WorkspaceHeld | undefined>(undefined);

/** The Workspace the window holds, or none. */
export const useWorkspaceHeld = (): WorkspaceHeld | undefined => useContext(Held);

/** `?take=` names where the Workspace form is drawn, and with it how the autonomy rows are. */
const TAKES: Readonly<Record<string, { workspace: WorkspaceTake; autonomy: AutonomyTake }>> = {
  sheet: { workspace: "sheet", autonomy: "table" },
  panel: { workspace: "panel", autonomy: "segmented" },
  popover: { workspace: "popover", autonomy: "cycle" },
};

const takeOf = () => TAKES[(typeof window === "undefined" ? new URLSearchParams() : new URLSearchParams(window.location.search)).get("take") ?? ""] ?? TAKES.sheet!;

export function WorkspaceFrom({ seed, children }: { seed: WorkspaceSeed | undefined; children: ReactNode }) {
  return seed === undefined ? <>{children}</> : <Kept seed={seed}>{children}</Kept>;
}

/** The layers the mock keeps, by the level that sets them: `everywhere`, `workspace:<id>` and `repository:<id>`. */
type Layers = Readonly<Record<string, AutonomyLayer>>;

function Kept({ seed, children }: { seed: WorkspaceSeed; children: ReactNode }) {
  const [workspace, setWorkspace] = useState(seed.workspace);
  const [records, setRecords] = useState<readonly WorkspaceRecord[]>(seed.workspaces?.records ?? []);
  const [layers, setLayers] = useState<Layers>({});
  const held = useMemo<WorkspaceHeld>(() => {
    const base: WorkspaceHeld = {
      workspace,
      change: setWorkspace,
      ask: (text) => seed.proposals.find((one) => text.toLowerCase().includes(one.asked))?.rule,
      add: (rule) => setWorkspace((was) => (was.rules.some((one) => one.id === rule.id) ? was : { ...was, rules: [...was.rules, rule] })),
    };
    const served = seed.workspaces?.served;
    if (served === undefined) return base;
    const take = takeOf();
    const nameOf = (id: string) => served.find((one) => one.id === id)?.name ?? id;
    /** The repositories leave any Workspace they were in, and a Workspace left with none goes. */
    const without = (ids: readonly string[], from: readonly WorkspaceRecord[]): WorkspaceRecord[] => from.map((one) => ({ ...one, repositories: one.repositories.filter((each) => !ids.includes(each)) })).filter((one) => one.repositories.length > 0);
    return {
      ...base,
      at: (repository) => {
        const record = records.find((one) => one.repositories.includes(repository));
        const repositories = (record?.repositories ?? [repository]).map((id): Repository => ({ id, name: nameOf(id) }));
        const set = (key: string, kind: AutonomyKind, level: Autonomy | undefined) =>
          setLayers((was) => {
            const next = { ...(was[key] ?? {}) };
            if (level === undefined) delete next[kind];
            else next[kind] = level;
            return { ...was, [key]: next };
          });
        const keyOf = (tab: string) => (tab === "default" ? "everywhere" : tab === "workspace" ? `workspace:${record?.id ?? ""}` : `repository:${tab}`);
        return {
          ...base,
          workspace: { ...workspace, id: record?.id ?? "", name: record?.name ?? "", repositories },
          editing: {
            workspaces: {
              take: take.workspace,
              repositories: served,
              current: repository,
              ...(record === undefined ? {} : { kept: record.kept }),
              onCreate: (name, ids) => {
                const id = name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
                setRecords((was) => [...without(ids, was), { id, name, repositories: ids, kept: "mac" as const }]);
              },
              onSave: (name, ids) => setRecords((was) => (record === undefined ? was : [...without(ids, was.filter((one) => one.id !== record.id)), { ...record, name, repositories: ids }])),
              onRemove: () => {
                setRecords((was) => was.filter((one) => one.id !== record?.id));
                setLayers((was) => Object.fromEntries(Object.entries(was).filter(([key]) => key !== keyOf("workspace"))));
              },
            },
            autonomy: {
              take: take.autonomy,
              layers: Object.fromEntries(["default", "workspace", ...repositories.map((one) => one.id)].map((tab) => [tab, layers[keyOf(tab)] ?? {}])),
              onSet: (tab, kind, level) => set(keyOf(tab), kind, level),
            },
          },
        };
      },
    };
  }, [workspace, records, layers, seed]);
  return <Held.Provider value={held}>{children}</Held.Provider>;
}
