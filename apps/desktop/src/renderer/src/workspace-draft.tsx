// The Workspace the Workflows page draws as lanes. Draft, mock-only: no Fleet serves a Workspace, a lifecycle or a rule yet, so a scenario hands
// its Workspace (`draft.workspace`) and this keeps it for the window, changed by the acts on it. A window with none draws the column of steps.

import { createContext, useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { Rule, Workspace } from "@armada/components";

/** What a scenario hands: the Workspace, and the changes an agent comes back with when it is asked for one in these words. */
export type WorkspaceSeed = { workspace: Workspace; proposals: readonly { asked: string; rule: Rule }[] };

/** The Workspace as the window holds it. */
export type WorkspaceHeld = {
  workspace: Workspace;
  /** The Workspace after a change by hand. */
  change: (next: Workspace) => void;
  /** The rule an agent comes back with for these words, or none where they are not about rules. */
  ask: (text: string) => Rule | undefined;
  add: (rule: Rule) => void;
};

const Held = createContext<WorkspaceHeld | undefined>(undefined);

/** The Workspace the window holds, or none. */
export const useWorkspaceHeld = (): WorkspaceHeld | undefined => useContext(Held);

export function WorkspaceFrom({ seed, children }: { seed: WorkspaceSeed | undefined; children: ReactNode }) {
  return seed === undefined ? <>{children}</> : <Kept seed={seed}>{children}</Kept>;
}

function Kept({ seed, children }: { seed: WorkspaceSeed; children: ReactNode }) {
  const [workspace, setWorkspace] = useState(seed.workspace);
  const held = useMemo<WorkspaceHeld>(
    () => ({
      workspace,
      change: setWorkspace,
      ask: (text) => seed.proposals.find((one) => text.toLowerCase().includes(one.asked))?.rule,
      add: (rule) => setWorkspace((was) => (was.rules.some((one) => one.id === rule.id) ? was : { ...was, rules: [...was.rules, rule] })),
    }),
    [workspace, seed],
  );
  return <Held.Provider value={held}>{children}</Held.Provider>;
}
