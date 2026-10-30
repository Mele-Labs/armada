// The Workspace the Workflows page draws as lanes where a scenario hands one (`draft.workspace`): this keeps it for the window, changed by the acts on it, and its proposals answer
// what an agent is told. Mock-only. A window without one draws the lifecycles and rules Fleet keeps (`lifecycle-editing/`), and a repository with none draws the column of steps.
// A seed with `workspaces` also names the Workspaces themselves and the autonomy each level sets (armada-cwj): the mock Fleet answers the Workspaces calls from them (`slices/lifecycles.ts`),
// and the Workflows page reads the first Workspace a repository is in from them.

import { createContext, useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { AutonomyLayer, Repository, Rule, Workspace } from "@armada/components";

/** A Workspace as the mock keeps it: its name, the repositories in it by name, and where it is kept. A committed one names the repository it lives in. */
export type WorkspaceRecord = { id: string; name: string; repositories: readonly string[]; kept: "mac" | "repository"; home?: string };

/** The layers the mock keeps, by the level that sets them: `everywhere`, `workspace:<id>` and `repository:<name>`. */
export type Layers = Readonly<Record<string, AutonomyLayer>>;

/** What a scenario hands: the Workspace, and the changes an agent comes back with when it is asked for one in these words. */
export type WorkspaceSeed = {
  workspace: Workspace;
  proposals: readonly { asked: string; rule: Rule }[];
  /** Every repository Fleet serves, the Workspaces they are in, and what each level already sets. The Workspace above holds the pieces (lifecycles, rules) all of them share. */
  workspaces?: { served: readonly Repository[]; records: readonly WorkspaceRecord[]; layers?: Layers };
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
};

const Held = createContext<WorkspaceHeld | undefined>(undefined);

/** The Workspace the window holds, or none. */
export const useWorkspaceHeld = (): WorkspaceHeld | undefined => useContext(Held);

export function WorkspaceFrom({ seed, children }: { seed: WorkspaceSeed | undefined; children: ReactNode }) {
  return seed === undefined ? <>{children}</> : <Kept seed={seed}>{children}</Kept>;
}

function Kept({ seed, children }: { seed: WorkspaceSeed; children: ReactNode }) {
  const [workspace, setWorkspace] = useState(seed.workspace);
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
    const records = seed.workspaces?.records ?? [];
    return {
      ...base,
      at: (repository) => {
        const record = records.find((one) => one.repositories.includes(repository));
        const repositories = (record?.repositories ?? [repository]).map((id): Repository => ({ id, name: nameOf(id) }));
        return { ...base, workspace: { ...workspace, id: record?.id ?? "", name: record?.name ?? "", repositories } };
      },
    };
  }, [workspace, seed]);
  return <Held.Provider value={held}>{children}</Held.Provider>;
}
