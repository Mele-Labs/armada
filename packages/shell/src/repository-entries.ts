// The rows of a repository picker, as a `DropdownMenu` draws them. One builder, so the title row's
// picker and the ask a surface puts on All cannot list the same repositories differently.

import type { DropdownMenuEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { jobIdOf, repositoryLabel } from "./repository-label";

/** The rail's value for All repositories. No root is empty, so it cannot collide with one. */
export const ALL = "";

/** No root is ever this, so it cannot collide with one. */
export const ADD = "add-repository";

/** No root is ever this either. */
export const NEW = "new-project";

/**
 * Past this many repositories the picker leads with the last few picked. Below it the list is short
 * enough to read and a Recent group would only repeat it. The filter field has no such floor: the
 * owner, 11 Oct 2026, wanted it always.
 */
export const RECENT_FROM = 6;

/** No root is ever this, so it cannot collide with one: New Workspace, beside Add a repository. */
export const NEW_WORKSPACE = "new-workspace";

/** A Workspace heading's id is this and then the Workspace's id. */
export const WORKSPACE = "workspace:";

/** What the filter field is called, to a screen reader and in the walk that types into it. */
export const FIND_A_REPOSITORY = "Find a repository";

/** A recent row's id: a repository listed twice must not repeat an id, and the prefix says which row. */
const RECENT = "recent:";

/** A repository under a Workspace: its id is this, the Workspace's id and a bar, then the root. A repository in two Workspaces is listed under each. */
const IN = "in|";

/** A Workspace as the picker lists it: the repositories in it by root. */
export type PickerWorkspace = { id: string; name: string; repositories: readonly string[] };

/** The root a picked row names, whichever of its places it was picked from. */
export function rootOf(id: string): string {
  if (id.startsWith(IN)) return id.slice(id.indexOf("|", IN.length) + 1);
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
    /** New project, above Add a repository. */
    create?: boolean;
    /** A repository a Job cannot start in (no Manifest and no `implicit_id`) is listed and cannot be picked — New job's, #959. */
    onlySetUp?: boolean;
    /** Roots picked last, newest first. Led with only where the picker is searchable. */
    recent?: readonly string[];
    /** The Workspaces, each a heading with its repositories under it, after All. A repository in none follows them. */
    workspaces?: readonly PickerWorkspace[];
    /** New Workspace, beside Add a repository. */
    newWorkspace?: boolean;
  },
): DropdownMenuEntry[] {
  const { scope, all, add = false, create = false, onlySetUp = false, recent = [], workspaces = [], newWorkspace = false } = options;
  const setUp = repositories.filter((one) => one.manifest !== undefined);
  const loose = repositories.filter((one) => one.manifest === undefined);
  const item = (repo: RepositorySummary, id = repo.root, member = false): DropdownMenuEntry => ({
    kind: "item",
    id,
    ...(member ? { member: true } : {}),
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
  // A Workspace with none of its repositories served is not listed: a heading over nothing.
  const heldBy = workspaces.map((one) => ({ one, repos: one.repositories.flatMap((root) => repositories.filter((repo) => repo.root === root)) })).filter((held) => held.repos.length > 0);
  const inNone = setUp.filter((repo) => !workspaces.some((one) => one.repositories.includes(repo.root)));
  if (setUp.length > 0) {
    if (all !== undefined) entries.push({ kind: "separator", id: "all-repositories-rule" });
    for (const { one, repos } of heldBy) {
      entries.push({ kind: "item", id: `${WORKSPACE}${one.id}`, label: one.name });
      entries.push(...repos.map((repo) => item(repo, `${IN}${one.id}|${repo.root}`, true)));
    }
    entries.push(...inNone.map((repo) => item(repo)));
  }
  if (loose.length > 0) {
    entries.push({ kind: "label", id: "not-set-up", label: "Not set up" });
    entries.push(...loose.map((repo) => item(repo)));
  }
  if (add || create || newWorkspace) entries.push({ kind: "separator", id: "add-repository-rule" });
  if (create) entries.push({ kind: "item", id: NEW, label: "New project" });
  if (add) entries.push({ kind: "item", id: ADD, label: "Add a repository" });
  if (newWorkspace) entries.push({ kind: "item", id: NEW_WORKSPACE, label: "New Workspace" });
  return entries;
}
