// What Fleet read for a Workspace's repositories, laid out by the level each piece applies at. The editor's tabs are those levels: the default's (every repository),
// the Workspace's, and one repository's. **Fleet does not merge a narrower piece into a wider one: it runs the narrower whole**, so a level that has a piece of its own
// reads that piece, and one that has none reads the nearest above it.

import { DEFAULT } from "@armada/components";
import type { AppliedLifecycle, AppliedRule, LifecycleDefinition, LifecycleRule, PieceKept, PieceScope, WorkspaceOfRepositories } from "@armada/protocol";

/** The Workspace's own tab. The default's is `DEFAULT`, and a repository's is its Manifest id. */
export const WORKSPACE = "workspace";

/** Everything read for the repository the page is on and the other repositories of its Workspace. */
export type PiecesRead = {
  /** The Workspace that runs. Absent for a repository that is in none. */
  workspace?: WorkspaceOfRepositories;
  repositories: readonly { id: string; name: string; lifecycles: readonly AppliedLifecycle[]; rules: readonly AppliedRule[] }[];
};

/** A piece and where it was kept. */
export type Layer<T> = { piece: T; kept: PieceKept };

/** The pieces of one id, by the level each applies at. */
export type Layers<T> = ReadonlyMap<string, Layer<T>>;

/** The level a piece's scope names: the Workspace's by its id, else a repository's. */
export function levelOf(scope: PieceScope, workspace: string | undefined): string {
  if (scope.kind === "everywhere") return DEFAULT;
  return scope.id === workspace ? WORKSPACE : scope.id;
}

/** The scope a piece kept at `level` carries. */
export function scopeOf(level: string, workspace: string | undefined): PieceScope {
  if (level === DEFAULT) return { kind: "everywhere" };
  return { kind: "within", id: level === WORKSPACE ? (workspace ?? level) : level };
}

/** The level above, whose piece a level without its own reads. The default has none. */
export const parentOf = (level: string): string | undefined => (level === DEFAULT ? undefined : level === WORKSPACE ? DEFAULT : WORKSPACE);

/** Where Fleet prefers a piece of one id and scope from: the person's own over the repository's over Armada's. */
const PLACE: Readonly<Record<string, number>> = { kit: 0, repository: 1, armada: 2 };

function collect<T extends { id: string; scope: PieceScope }>(all: readonly Layer<T>[], workspace: string | undefined): Map<string, Map<string, Layer<T>>> {
  const out = new Map<string, Map<string, Layer<T>>>();
  const seen = new Set<string>();
  for (const one of all) {
    // The same file is read once for each repository it applies to.
    const key = `${one.kept.path ?? "armada"}\n${one.piece.id}\n${JSON.stringify(one.piece.scope)}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const level = levelOf(one.piece.scope, workspace);
    const levels = out.get(one.piece.id) ?? new Map<string, Layer<T>>();
    const held = levels.get(level);
    if (held === undefined || (PLACE[one.kept.rank.source] ?? 9) < (PLACE[held.kept.rank.source] ?? 9)) levels.set(level, one);
    out.set(one.piece.id, levels);
  }
  return out;
}

/** The lifecycles by id, each by level. Armada's four are read without a path and are the default's. */
export function lifecycleLayers(read: PiecesRead): Map<string, Layers<LifecycleDefinition>> {
  return collect(
    read.repositories.flatMap((one) => one.lifecycles.map((each): Layer<LifecycleDefinition> => ({ piece: each.lifecycle, kept: each.kept }))),
    read.workspace?.id,
  );
}

/** The rules by id, each by level. */
export function ruleLayers(read: PiecesRead): Map<string, Layers<LifecycleRule>> {
  return collect(
    read.repositories.flatMap((one) => one.rules.map((each): Layer<LifecycleRule> => ({ piece: each.rule, kept: each.kept }))),
    read.workspace?.id,
  );
}

/** The piece a level runs: its own, else the nearest above it. */
export function effective<T>(layers: Layers<T>, level: string): { level: string; layer: Layer<T> } | undefined {
  for (let at: string | undefined = level; at !== undefined; at = parentOf(at)) {
    const layer = layers.get(at);
    if (layer !== undefined) return { level: at, layer };
  }
  return undefined;
}

/** The levels a Workspace's tabs are, widest first. */
export const levelsOfRead = (read: PiecesRead): string[] => [DEFAULT, ...(read.workspace === undefined ? [] : [WORKSPACE]), ...read.repositories.map((one) => one.id)];
