// The folders every session may reach past its slots with no card, and the ones it may not: the
// value of the `features.reachFolders` setting. `crates/ipc/src/reach_folders.rs`.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and
// they drift the day a field moves.

/** The settings key the list is kept under, in settings.json. */
export const REACH_FOLDERS_KEY = "features.reachFolders";

/** Both lists, an absolute path per entry, in the order they were added. A list left out is empty. */
export type ReachFolders = {
  allow?: string[];
  block?: string[];
};

/**
 * The lists a setting's value holds. Read the way fleet reads them: anything not the shape is an
 * empty list, and an entry that is not an absolute path is dropped.
 */
export function reachFoldersOf(value: unknown): { allow: string[]; block: string[] } {
  const listed = (raw: unknown): string[] =>
    Array.isArray(raw) ? raw.filter((one): one is string => typeof one === "string" && one.startsWith("/")) : [];
  if (typeof value !== "object" || value === null) return { allow: [], block: [] };
  const read = value as ReachFolders;
  return { allow: listed(read.allow), block: listed(read.block) };
}

/** `folder` taken off both lists: what Settings sends to remove an entry. */
export function reachFoldersWithout(value: unknown, folder: string): ReachFolders {
  const { allow, block } = reachFoldersOf(value);
  const kept: ReachFolders = {};
  const left = { allow: allow.filter((one) => one !== folder), block: block.filter((one) => one !== folder) };
  if (left.allow.length > 0) kept.allow = left.allow;
  if (left.block.length > 0) kept.block = left.block;
  return kept;
}
