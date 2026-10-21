// The root of the repository the Reviews page is picked on, for the acts on a review tool of the person's own: a commit lands on a
// Job's branch in it, and a package names it. Absent with none picked, so the acts are not drawn.

import { createContext, useContext } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf } from "@armada/shell";

export const PickedRepository = createContext<RepositorySummary | null>(null);

/** The picked repository's root, where it is the one the pull request is in. */
export function useRootFor(manifestId: string): string | undefined {
  const picked = useContext(PickedRepository);
  return picked !== null && jobIdOf(picked) === manifestId ? picked.root : undefined;
}
