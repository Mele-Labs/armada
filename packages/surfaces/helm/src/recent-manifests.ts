// The last-picked repositories as Helm's switch names them. The title row's picker keeps its
// recents by root, and the switch's rows are Manifest ids, so this is the one place they meet.

import type { RepositorySummary } from "@armada/protocol";
import { RECENT_FROM } from "@armada/shell";

/**
 * Roots picked last, newest first, as the Manifest ids of the repositories that are still served
 * and set up. **Led with only past as many switch rows as the picker leads with**, where it adds
 * something to a list too long to read at a glance: a short one gets none.
 */
export function recentManifestIds(
  repositories: readonly RepositorySummary[],
  recentRoots: readonly string[],
): string[] {
  const setUp = repositories.filter((one) => one.manifest !== undefined);
  if (setUp.length <= RECENT_FROM) return [];
  return recentRoots.flatMap((root) => {
    const found = setUp.find((one) => one.root === root);
    return found?.manifest === undefined ? [] : [found.manifest.id];
  });
}
