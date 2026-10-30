// The Workspace the Workflows page draws as lanes where a scenario hands one (`draft.workspace`): this keeps it for the window, changed by the acts on it, and its proposals answer
// what an agent is told. Mock-only. A window without one draws the lifecycles and rules Fleet keeps (`lifecycle-editing/`), and a repository with none draws the column of steps.
// A seed with `workspaces` also keeps the Workspaces themselves and the autonomy each level sets (armada-cwj), which the repository picker lists and System > Workspaces draws.

import { createContext, useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { Autonomy, AutonomyKind, AutonomyLayer, Repository, Rule, Workspace, WorkspacesPick } from "@armada/components";

/** A Workspace as the mock keeps it: its name, the repositories in it by id, and where it is kept. A committed one names the repository it lives in. */
export type WorkspaceRecord = { id: string; name: string; repositories: readonly string[]; kept: "mac" | "repository"; home?: string };

/** The layers the mock keeps, by the level that sets them: `everywhere`, `workspace:<id>` and `repository:<id>`. */
export type Layers = Readonly<Record<string, AutonomyLayer>>;

/** What a scenario hands: the Workspace, and the changes an agent comes back with when it is asked for one in these words. */
export type WorkspaceSeed = {
  workspace: Workspace;
  proposals: readonly { asked: string; rule: Rule }[];
  /** Every repository Fleet serves, the Workspaces they are in, and what each level already sets. The Workspace above holds the pieces (lifecycles, rules) all of them share. */
  workspaces?: { served: readonly Repository[]; records: readonly WorkspaceRecord[]; layers?: Layers };
};

/** The Workspaces and autonomy defaults the window holds, which System > Workspaces draws and the repository picker lists. */
export type WorkspaceBook = {
  served: readonly Repository[];
  records: readonly WorkspaceRecord[];
  layers: Layers;
  /** The level the Workspaces page shows. */
  picked: WorkspacesPick;
  pick: (next: WorkspacesPick) => void;
  set: (key: string, kind: AutonomyKind, level: Autonomy | undefined) => void;
  create: (name: string, repositories: readonly string[]) => void;
  save: (id: string, name: string, repositories: readonly string[]) => void;
  remove: (id: string) => void;
};

/** The Workspace as the window holds it. */
export type WorkspaceHeld = {
  workspace: Workspace;
  /** The Workspace after a change by hand. */
  change: (next: Workspace) => void;
  /** The rule an agent comes back with for these words, or none where they are not about rules. */
  ask: (text: string) => Rule | undefined;
  add: (rule: Rule) => void;
  /** The same, as the Workflows page reads it for the repository it is on: the first Workspace the repository is in. Absent where the seed keeps no Workspaces. */
  at?: (repository: string) => WorkspaceHeld;
  book?: WorkspaceBook;
};

const Held = createContext<WorkspaceHeld | undefined>(undefined);

/** The Workspace the window holds, or none. */
export const useWorkspaceHeld = (): WorkspaceHeld | undefined => useContext(Held);

export function WorkspaceFrom({ seed, children }: { seed: WorkspaceSeed | undefined; children: ReactNode }) {
  return seed === undefined ? <>{children}</> : <Kept seed={seed}>{children}</Kept>;
}

function Kept({ seed, children }: { seed: WorkspaceSeed; children: ReactNode }) {
  const [workspace, setWorkspace] = useState(seed.workspace);
  const [records, setRecords] = useState<readonly WorkspaceRecord[]>(seed.workspaces?.records ?? []);
  const [layers, setLayers] = useState<Layers>(seed.workspaces?.layers ?? {});
  const [picked, pick] = useState<WorkspacesPick>({ kind: "everywhere" });
  const held = useMemo<WorkspaceHeld>(() => {
    const base: WorkspaceHeld = {
      workspace,
      change: setWorkspace,
      ask: (text) => seed.proposals.find((one) => text.toLowerCase().includes(one.asked))?.rule,
      add: (rule) => setWorkspace((was) => (was.rules.some((one) => one.id === rule.id) ? was : { ...was, rules: [...was.rules, rule] })),
    };
    const served = seed.workspaces?.served;
    if (served === undefined) return base;
    const nameOf = (id: string) => served.find((one) => one.id === id)?.name ?? id;
    const book: WorkspaceBook = {
      served,
      records,
      layers,
      picked,
      pick,
      set: (key, kind, level) =>
        setLayers((was) => {
          const next = { ...(was[key] ?? {}) };
          if (level === undefined) delete next[kind];
          else next[kind] = level;
          return { ...was, [key]: next };
        }),
      create: (name, ids) => {
        const id = name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
        setRecords((was) => [...was, { id, name, repositories: ids, kept: "mac" }]);
        pick({ kind: "workspace", id });
      },
      save: (id, name, ids) => setRecords((was) => was.map((one) => (one.id === id ? { ...one, name, repositories: ids } : one))),
      remove: (id) => {
        setRecords((was) => was.filter((one) => one.id !== id));
        setLayers((was) => Object.fromEntries(Object.entries(was).filter(([key]) => key !== `workspace:${id}`)));
        pick({ kind: "everywhere" });
      },
    };
    return {
      ...base,
      book,
      at: (repository) => {
        const record = records.find((one) => one.repositories.includes(repository));
        const repositories = (record?.repositories ?? [repository]).map((id): Repository => ({ id, name: nameOf(id) }));
        return { ...base, workspace: { ...workspace, id: record?.id ?? "", name: record?.name ?? "", repositories } };
      },
    };
  }, [workspace, records, layers, picked, seed]);
  return <Held.Provider value={held}>{children}</Held.Provider>;
}
