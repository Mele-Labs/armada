// The repositories of the Workspace the window is scoped to, for the Sessions store, which reads Main's state and not the app's. Main keeps the pick as one
// repository or All, so a Workspace pick leaves it on All and is held here instead.

import type { RepositorySummary } from "@armada/protocol";

let scope: readonly RepositorySummary[] | null = null;
const listeners = new Set<() => void>();

export const workspaceScope = (): readonly RepositorySummary[] | null => scope;

/** `null` is no Workspace. A list with the same repositories as the one held is not a change. */
export function scopeToWorkspace(members: readonly RepositorySummary[] | null): void {
  if (members === null ? scope === null : scope !== null && scope.length === members.length && scope.every((one, at) => one.root === members[at]!.root)) return;
  scope = members;
  listeners.forEach((listener) => listener());
}

export function onWorkspaceScope(listener: () => void): () => void {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}
