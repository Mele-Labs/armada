// The directories a repository is drawn in on the flight deck's battlefield, and the one rule that puts a path in one.
//
// **An area is two directories deep.** `crates/fleet`, `apps/desktop`, `docs/concepts`: the grain the battlefield draws
// plots at and the grain a touch or a write clash is rolled up to. A file at the top of a repository, or a path under no
// checkout, is in none.
//
// **Nothing here knows what a repository holds.** The areas come from Fleet's `list_repository_areas`, the base head's
// directories counted by git. Where Fleet has not answered, has no such route or names none for a repository, its
// territory shows pads and the main flag with no plots. A walk may stand a source in. #2301.

import type { AreaKid, RepositoryArea, RepositoryAreas } from "@armada/protocol";

/**
 * A directory inside a top one: its name, its file count and, where Fleet named them, its first files as git lists them,
 * relative to the kid. Fewer names than the count means the rest were cut, not that they are not there.
 */
export type Kid = readonly [name: string, files: number, names?: readonly string[]];

/**
 * One top directory with the directories inside it, each with a file count. A top directory with no children has none
 * listed, and names its own first files instead (relative to `dir`) with its exact count in `files`; one with children
 * names none.
 */
export type Area = { dir: string; kids: readonly Kid[]; names?: readonly string[]; files?: number };

export type Areas = readonly Area[];

/** Where a repository's areas come from: its root, to the directories drawn for it. */
export type AreaSource = (root: string) => Areas;

/** No areas for any repository: before Fleet has answered, and from a Fleet with no such route. */
export const noAreas: AreaSource = () => [];

const kidOf = (kid: AreaKid): Kid => (kid.names === undefined ? [kid.name, kid.files] : [kid.name, kid.files, kid.names]);
const areaFromWire = (area: RepositoryArea): Area => ({ dir: area.dir, kids: (area.kids ?? []).map(kidOf), ...(area.names === undefined ? {} : { names: area.names, files: area.files }) });

/** The areas Fleet read, for the root each was read in. A root Fleet named none for has none. */
export function areasFromFleet(read: RepositoryAreas): AreaSource {
  const bare = (root: string) => root.replace(/\/+$/, "");
  const byRoot = new Map(
    read.repositories.map((one) => [bare(one.root), one.areas.map(areaFromWire)] as const),
  );
  return (root) => byRoot.get(bare(root)) ?? [];
}

/** The area a repository-relative path is in, or none for a file at the top of the repository and for a path that is not relative. */
export function areaOf(path: string): string | undefined {
  if (path === "" || path.startsWith("/") || path.startsWith("~")) return undefined;
  const parts = path.split("/").filter((part) => part !== "");
  // The last part is the file, so a path of one part has no directory and a path of two has one.
  const dirs = parts.slice(0, -1).slice(0, 2);
  return dirs.length === 0 ? undefined : dirs.join("/");
}

/** The home directory off an absolute path, so `~/x` and `/Users/user/x` can be set side by side. */
const HOME = /^\/(?:Users|home)\/[^/]+\//;

/**
 * A path as the repository spells it, where it lies under one of these checkouts, in either spelling of the home directory.
 * A path under none is somebody else's file and comes back as it came, which no area claims.
 */
export function relativeTo(path: string, roots: readonly string[]): string {
  for (const root of roots) {
    const bare = root.replace(/\/+$/, "");
    if (path.startsWith(`${bare}/`)) return path.slice(bare.length + 1);
    const homed = bare.replace(HOME, "");
    if (homed !== bare && path.startsWith(`~/${homed}/`)) return path.slice(homed.length + 3);
  }
  return path;
}

/** A path as the repository spells it, or none where it lies outside these checkouts: a plugin's notes, a scratch file. */
export function insideOf(path: string, roots: readonly string[]): string | undefined {
  const named = relativeTo(path, roots);
  return named === "" || named.startsWith("/") || named.startsWith("~/") ? undefined : named;
}
