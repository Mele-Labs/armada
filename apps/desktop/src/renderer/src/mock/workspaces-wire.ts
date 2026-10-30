// The Workspaces and autonomy layers a scenario's `draft.workspace.workspaces` names (armada-cwj), as Fleet keeps them: Workspaces by Manifest id, and each level's layer scoped by the id of
// what it is for. The seed names repositories by name, which the Workflows page draws; the wire names them by Manifest.

import type { Autonomy } from "@armada/components";
import type { PieceKept, PieceScope, RepositorySummary, WorkspaceOfRepositories } from "@armada/protocol";

import type { WorkspaceSeed } from "../workspace-draft";

const KIT = "/Users/user/.armada";

/** A layer as the mock keeps it: the kinds it sets by the wire's names, where it is kept. */
export type KeptAutonomy = { layer: { id: string; scope: PieceScope; defaults: Record<string, string> }; kept: PieceKept };

export function workspacesWire(book: NonNullable<WorkspaceSeed["workspaces"]>, repositories: readonly RepositorySummary[]): { workspaces: WorkspaceOfRepositories[]; autonomy: KeptAutonomy[] } {
  const idOf = (name: string) => repositories.find((one) => one.manifest?.repository === name)?.manifest?.id ?? name;
  const workspaces = book.records.map(
    (one): WorkspaceOfRepositories => ({
      id: one.id,
      name: one.name,
      repositories: one.repositories.map(idOf),
      ...(one.home === undefined ? {} : { home: idOf(one.home) }),
      source: one.kept === "mac" ? "kit" : "repository",
      path: one.kept === "mac" ? `${KIT}/workspaces/${one.id}.toml` : `/Users/user/${one.home}/.armada/workspaces/${one.id}.toml`,
    }),
  );
  const autonomy = Object.entries(book.layers ?? {}).map(([level, layer]): KeptAutonomy => {
    const [tier, name = ""] = level.split(":");
    const scope: PieceScope = tier === "everywhere" ? { kind: "everywhere" } : { kind: "within", id: tier === "repository" ? idOf(name) : name };
    return {
      layer: { id: "defaults", scope, defaults: Object.fromEntries(Object.entries(layer).filter((one): one is [string, Autonomy] => one[1] !== undefined)) },
      kept: { rank: { reach: tier!, source: "kit" }, path: `${KIT}/autonomy/defaults.toml` },
    };
  });
  return { workspaces, autonomy };
}
