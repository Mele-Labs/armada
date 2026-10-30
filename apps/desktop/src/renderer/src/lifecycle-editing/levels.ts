// What Fleet read for a Workspace's repositories, laid out by the level each piece applies at. The editor's tabs are those levels: the default's (every repository), one for each Workspace
// that holds the repository the page is on (`workspace:<id>`), and one repository's. **Fleet does not merge a narrower piece into a wider one: it runs the narrower whole**, so a level that has a
// piece of its own reads that piece, and one that has none reads the nearest above it. A repository in two Workspaces has both above it: the one that sets a piece runs, and where both set it at one
// place Fleet runs neither and the repository reads the default's (`crates/config/src/resolving.rs`).

import { DEFAULT, isWorkspaceLevel, workspaceLevel } from "@armada/components";
import type { AppliedLifecycle, AppliedRule, LifecycleDefinition, LifecycleRule, PieceKept, PieceScope, WorkspaceOfRepositories } from "@armada/protocol";

/** Everything read for the repository the page is on and the other repositories of its Workspaces. */
export type PiecesRead = {
  /** The Workspaces that run for the repository the page is on, in the order Fleet lists them. Empty for a repository that is in none. */
  workspaces: readonly WorkspaceOfRepositories[];
  repositories: readonly { id: string; name: string; lifecycles: readonly AppliedLifecycle[]; rules: readonly AppliedRule[] }[];
};

/** A piece and where it was kept. */
export type Layer<T> = { piece: T; kept: PieceKept };

/** The pieces of one id, by the level each applies at. */
export type Layers<T> = ReadonlyMap<string, Layer<T>>;

/** The level a piece's scope names: a Workspace's by its id, else a repository's. */
export function levelOf(scope: PieceScope, read: Pick<PiecesRead, "workspaces">): string {
  if (scope.kind === "everywhere") return DEFAULT;
  return read.workspaces.some((one) => one.id === scope.id) ? workspaceLevel(scope.id) : scope.id;
}

/** The scope a piece kept at `level` carries. */
export function scopeOf(level: string): PieceScope {
  if (level === DEFAULT) return { kind: "everywhere" };
  return { kind: "within", id: isWorkspaceLevel(level) ? level.slice(workspaceLevel("").length) : level };
}

/** The levels of the Workspaces that hold a repository, in the order Fleet lists them. */
const holdersOf = (read: Pick<PiecesRead, "workspaces">, repository: string): string[] => read.workspaces.filter((one) => one.repositories.includes(repository)).map((one) => workspaceLevel(one.id));

/** Where Fleet prefers a piece of one id and scope from: the person's own over the repository's over Armada's. */
const PLACE: Readonly<Record<string, number>> = { kit: 0, repository: 1, armada: 2 };

function collect<T extends { id: string; scope: PieceScope }>(all: readonly Layer<T>[], read: PiecesRead): Map<string, Map<string, Layer<T>>> {
  const out = new Map<string, Map<string, Layer<T>>>();
  const seen = new Set<string>();
  for (const one of all) {
    // The same file is read once for each repository it applies to.
    const key = `${one.kept.path ?? "armada"}\n${one.piece.id}\n${JSON.stringify(one.piece.scope)}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const level = levelOf(one.piece.scope, read);
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
    read,
  );
}

/** The rules by id, each by level. */
export function ruleLayers(read: PiecesRead): Map<string, Layers<LifecycleRule>> {
  return collect(
    read.repositories.flatMap((one) => one.rules.map((each): Layer<LifecycleRule> => ({ piece: each.rule, kept: each.kept }))),
    read,
  );
}

/** What a level reads where it has no piece of its own: the Workspace level that sets it, or the default's. `clash` is the Workspace levels that set it alike and so are set aside. */
export type Inherited<T> = { found: { level: string; layer: Layer<T> } | undefined; clash: string[] };

/** The preference between two layers of one id: the place, then whether it came from the Workspace's home. */
const better = (a: Layer<unknown>, b: Layer<unknown>): boolean => {
  const place = (PLACE[a.kept.rank.source] ?? 9) - (PLACE[b.kept.rank.source] ?? 9);
  return place !== 0 ? place < 0 : a.kept.rank.home === true && b.kept.rank.home !== true;
};

/**
 * The piece a level would run without one of its own. A Workspace's reads the default's. **A repository reads the Workspace that sets the piece**; where two set it at one place (the same kept place) Fleet
 * keeps neither, so those are set aside, and a repository none else sets reads the default's.
 */
export function inherited<T>(layers: Layers<T>, level: string, read: PiecesRead): Inherited<T> {
  if (level === DEFAULT) return { found: undefined, clash: [] };
  const wider = layers.get(DEFAULT);
  const above = wider === undefined ? undefined : { level: DEFAULT, layer: wider };
  if (isWorkspaceLevel(level)) return { found: above, clash: [] };
  const setting = holdersOf(read, level).flatMap((held) => {
    const layer = layers.get(held);
    return layer === undefined ? [] : [{ level: held, layer }];
  });
  const tied = (one: { layer: Layer<T> }) => setting.filter((each) => each.layer.kept.rank.source === one.layer.kept.rank.source && (each.layer.kept.rank.home === true) === (one.layer.kept.rank.home === true));
  const kept = setting.filter((one) => tied(one).length === 1);
  const winner = kept.reduce<(typeof kept)[number] | undefined>((best, one) => (best === undefined || better(one.layer, best.layer) ? one : best), undefined);
  return { found: winner ?? above, clash: setting.filter((one) => tied(one).length > 1).map((one) => one.level) };
}

/** The piece a level runs: its own, else the one it inherits. */
export function effective<T>(layers: Layers<T>, level: string, read: PiecesRead): { level: string; layer: Layer<T> } | undefined {
  const layer = layers.get(level);
  return layer === undefined ? inherited(layers, level, read).found : { level, layer };
}

/** The Workspace levels a repository's tab sets aside for a piece because two set it alike, as groups; absent where none does. */
export function clashOf<T>(layers: Layers<T>, read: PiecesRead): string[][] {
  const groups = new Map<string, string[]>();
  for (const repository of read.repositories) {
    const { clash } = inherited(layers, repository.id, read);
    if (clash.length > 1) groups.set(clash.join(), clash);
  }
  return [...groups.values()];
}

/** The levels a Workspace's tabs are, widest first. */
export const levelsOfRead = (read: PiecesRead): string[] => [DEFAULT, ...read.workspaces.map((one) => workspaceLevel(one.id)), ...read.repositories.map((one) => one.id)];
