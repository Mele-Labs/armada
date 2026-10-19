// How a repository runs and where it is kept, for the mock stores that offer it: what Fleet saw pass, and the place the repository was added with. Mock only.

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
  jobs: readonly { id: string; job: string; workspace?: string; command?: string; port?: number }[];
};

/**
 * `read` answers what was seen to pass of a repository now, and `changed` tells whatever draws that something moved. Armada keeps what passed on its own,
 * under the workspace it ran in, in the place the repository was added with (On this Mac until one is named). A line dropped is never kept again.
 * A repository can start with a place already named.
 */
export function keptRuns(
  read: (root: string) => RunsRead | undefined,
  changed: () => void,
  before: { root: string; place: RunsPlace } | undefined = undefined,
): NonNullable<SessionsStore["runs"]> {
  const places = new Map<string, RunsPlace>(before === undefined ? [] : [[before.root, before.place]]);
  const dropped = new Map<string, ReadonlySet<string>>();
  return {
    synced: SYNCED,
    profiles: PROFILES,
    of: (root) => {
      const one = read(root);
      if (one === undefined) return undefined;
      return {
        repository: one.repository,
        source: one.source,
        rows: one.jobs.filter((job) => !dropped.get(root)?.has(job.id)),
        place: places.get(root) ?? { id: "mac" },
      };
    },
    place: (root, place) => {
      places.set(root, place);
      changed();
    },
    drop: (root, id) => {
      dropped.set(root, new Set([...(dropped.get(root) ?? []), id]));
      changed();
    },
  };
}
