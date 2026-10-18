// How a repository runs and where it is kept, for the mock stores that offer it: what Fleet saw, and what a Remember has kept and where.
// A repository's first Remember names a place and the Remembers after it reuse that place, so a place is held by repository. Mock only.

import type { RunsPlace, SyncedFolder } from "@armada/components";
import type { SessionsStore } from "./sessions/script";

/** The synced folders the mock detects on this Mac. */
export const SYNCED: readonly SyncedFolder[] = [
  { label: "iCloud Drive", path: "~/Library/Mobile Documents/com~apple~CloudDocs" },
  { label: "Google Drive", path: "~/Library/CloudStorage/GoogleDrive-user@gmail.com/My Drive" },
];

/** The profiles signed in to Armada's store. */
export const PROFILES: readonly string[] = ["nickmele", "mele-work"];

/** What Fleet saw of a repository, by job. A job with no command did not run. */
export type RunsRead = {
  repository: string;
  source: string;
  jobs: readonly { id: string; job: string; command?: string; port?: number }[];
};

/** `read` answers what was seen of a repository now, and `changed` tells whatever draws that something was kept. */
export function keptRuns(read: (root: string) => RunsRead | undefined, changed: () => void): NonNullable<SessionsStore["runs"]> {
  const kept = new Map<string, ReadonlySet<string>>();
  const places = new Map<string, RunsPlace>();
  return {
    synced: SYNCED,
    profiles: PROFILES,
    of: (root) => {
      const one = read(root);
      if (one === undefined) return undefined;
      const place = places.get(root);
      return {
        repository: one.repository,
        source: one.source,
        rows: one.jobs.map((job) => ({ ...job, state: kept.get(root)?.has(job.id) ? "remembered" : "seen" })),
        ...(place === undefined ? {} : { place }),
      };
    },
    remember: (root, place) => {
      const one = read(root);
      const where = place ?? places.get(root);
      if (one === undefined || where === undefined) return;
      places.set(root, where);
      kept.set(root, new Set(one.jobs.map((job) => job.id)));
      changed();
    },
  };
}
