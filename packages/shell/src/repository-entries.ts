// The rows of a repository picker, as a `DropdownMenu` draws them. One builder, so the title row's
// picker and the ask a surface puts on All cannot list the same repositories differently.

import type { DropdownMenuEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { jobIdOf, repositoryLabel } from "./repository-label";

/** The rail's value for All repositories. No root is empty, so it cannot collide with one. */
export const ALL = "";

/** No root is ever this, so it cannot collide with one. */
export const ADD = "add-repository";

/**
 * Past this many repositories the picker leads with the last few picked. Below it the list is short
 * enough to read and a Recent group would only repeat it. The filter field has no such floor: the
 * owner, 11 Oct 2026, wanted it always.
 */
export const RECENT_FROM = 6;

/** What the filter field is called, to a screen reader and in the walk that types into it. */
export const FIND_A_REPOSITORY = "Find a repository";

/** A recent row's id: a repository listed twice must not repeat an id, and the prefix says which row. */
const RECENT = "recent:";

/** The root a picked row names, whichever of its two places it was picked from. */
export function rootOf(id: string): string {
  return id.startsWith(RECENT) ? id.slice(RECENT.length) : id;
}

export function repositoryEntries(
  repositories: readonly RepositorySummary[],
  options: {
    /** The pick the checkmark rests on. `undefined` marks none, as the ask does. */
    scope?: string | null;
    /** The row for All repositories, with this said where nothing is served. */
    all?: string;
    /** Add a repository, last, below a rule. */
    add?: boolean;
    /** A repository a Job cannot start in (no Manifest and no `implicit_id`) is listed and cannot be picked — New job's, #959. */
    onlySetUp?: boolean;
    /** Roots picked last, newest first. Led with only where the picker is searchable. */
    recent?: readonly string[];
  },
): DropdownMenuEntry[] {
  const { scope, all, add = false, onlySetUp = false, recent = [] } = options;
  const setUp = repositories.filter((one) => one.manifest !== undefined);
  const loose = repositories.filter((one) => one.manifest === undefined);
  const item = (repo: RepositorySummary, id = repo.root): DropdownMenuEntry => ({
    kind: "item",
    id,
    label: onlySetUp && jobIdOf(repo) === undefined ? `${repositoryLabel(repo, repositories)} — Set it up first` : repositoryLabel(repo, repositories),
    selected: scope !== undefined && repo.root === scope,
    ...(onlySetUp && jobIdOf(repo) === undefined ? { disabled: true } : {}),
  });
  const entries: DropdownMenuEntry[] = [];
  const led = repositories.length > RECENT_FROM
    ? recent.flatMap((root) => repositories.filter((one) => one.root === root && (!onlySetUp || jobIdOf(one) !== undefined)))
    : [];
  if (led.length > 0) {
    entries.push({ kind: "label", id: "recent", label: "Recent" });
    entries.push(...led.map((repo) => item(repo, `${RECENT}${repo.root}`)));
    entries.push({ kind: "separator", id: "recent-rule" });
  }
  if (all !== undefined) {
    entries.push({ kind: "item", id: ALL, label: all, selected: scope === null });
  }
  if (setUp.length > 0) {
    if (all !== undefined) entries.push({ kind: "separator", id: "all-repositories-rule" });
    entries.push(...setUp.map((repo) => item(repo)));
  }
  if (loose.length > 0) {
    entries.push({ kind: "label", id: "not-set-up", label: "Not set up" });
    entries.push(...loose.map((repo) => item(repo)));
  }
  if (add) {
    entries.push({ kind: "separator", id: "add-repository-rule" });
    entries.push({ kind: "item", id: ADD, label: "Add a repository" });
  }
  return entries;
}
