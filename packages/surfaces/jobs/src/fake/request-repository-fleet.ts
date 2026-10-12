// The mock Fleet's reading of which repository a request is about. Fleet's own is deterministic
// (`read_request_repository`: a link, a name, a path, the only repository), and this is a stand-in for
// the names a mock request uses: a word naming the repository, or one of the words it is known by
// here, picks it; a request naming none, or two, picks none, which is what the panel's inline ask is
// for. It answers after `READ_MS` so the chip's finding state is drawn long enough to be seen.

import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import type { RequestRepositoryRead } from "../api";

/** How long the stand-in takes to answer, on top of the panel's pause in typing. */
export const READ_MS = 350;

/** What each repository is also called in a request, by its label. A repository named nothing here is known by its own name. */
const KNOWN_AS: Record<string, RegExp> = {
  armada: /\b(armada|bridge|cockpit|fleet|drones?)\b/i,
  storefront: /\b(storefront|cart|checkout|catalogue)\b/i,
  billing: /\b(billing|invoices?|ledger)\b/i,
};

const escaped = (word: string) => word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** The set-up repositories ranked as Fleet ranks them: those the words name first, then those with the most Jobs. */
export function requestRepositoryRead(
  words: string,
  repositories: readonly RepositorySummary[],
  jobs: readonly JobSummary[],
): RequestRepositoryRead {
  const ranked = repositories
    .flatMap((one) => {
      if (one.manifest === undefined) return [];
      const name = repositoryLabel(one, repositories);
      const known = KNOWN_AS[name.toLowerCase()] ?? new RegExp(`\\b${escaped(name)}\\b`, "i");
      const busy = jobs.filter((job) => job.owner_manifest_id === one.manifest?.id).length;
      return [{ id: one.manifest.id, name, named: known.test(words), busy }];
    })
    .sort((a, b) => Number(b.named) - Number(a.named) || b.busy - a.busy || a.name.localeCompare(b.name));
  const named = ranked.filter((one) => one.named);
  return { manifest_id: named.length === 1 ? named[0]!.id : null, likely: ranked.map((one) => one.id) };
}
