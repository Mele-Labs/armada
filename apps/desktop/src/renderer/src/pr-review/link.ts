// Which served repository a link to a pull request belongs to. **Never a guess**: a link to a repository Fleet does not serve is
// refused, where `repositoryOf` (`shared-panel/fleet-reads.ts`) would give it the only one there is.

import type { RepositorySummary } from "@armada/protocol";

import type { PullLink } from "../../../shared/pull-link";

/** The manifest id of the repository a link names: one a pull request already named by its forge name, else a Manifest whose directory is that name. */
export function manifestOfLink(repositories: readonly RepositorySummary[], known: readonly { repository: string; manifestId: string }[], link: PullLink): string | undefined {
  const full = `${link.owner}/${link.name}`.toLowerCase();
  const listed = known.find((one) => one.repository.toLowerCase() === full);
  if (listed !== undefined) return listed.manifestId;
  return repositories.find((one) => [full, link.name.toLowerCase()].includes(one.manifest?.repository.toLowerCase() ?? ""))?.manifest?.id;
}
