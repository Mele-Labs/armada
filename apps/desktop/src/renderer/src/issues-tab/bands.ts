// Which repositories the list draws and the query each runs. The repository picked in the title row narrows it to one; All
// draws every repository Fleet serves a tracker for, each under its heading, as the Cockpit's list does (`cockpit/deck/bands.ts`).

import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf, repositoryLabel } from "@armada/shell";

import { ASSIGNED, BUILT_IN_FIELDS, OPEN, savedName } from "./model";
import type { ReadTracker, SavedQuery, Tracker } from "./model";
import type { Describing } from "./store";

export type Band = { manifestId: string; root: string; label: string; tracker: Tracker };

export function bandsIn(picked: RepositorySummary | null, repositories: readonly RepositorySummary[], trackers: readonly Tracker[]): Band[] {
  return (picked === null ? repositories : [picked]).flatMap((one) => {
    const id = jobIdOf(one);
    const tracker = trackers.find((it) => it.manifestId === id);
    return id === undefined || tracker === undefined ? [] : [{ manifestId: id, root: one.root, label: repositoryLabel(one, repositories), tracker }];
  });
}

export const readable = (band: Band): band is Band & { tracker: ReadTracker } => band.tracker.state === "read";

/** The names of the queries saved in these repositories, each once, in the order first met. */
export const savedNames = (bands: readonly Band[]): string[] => [...new Set(bands.filter(readable).flatMap((band) => band.tracker.saved.map((one) => one.name)))];

/**
 * What one repository's list runs: the query as text and the fields its rows show, or the model still writing it, or Fleet's
 * words for why it could not. `undefined` leaves the repository out of this choice: it keeps no query of that name.
 */
export type Asked = { state: "query"; query: string; fields: string[]; saved?: SavedQuery; written?: true } | { state: "writing" } | { state: "failed"; said: string };

export function askedOf(band: Band & { tracker: ReadTracker }, choice: string, describing: Describing | undefined): Asked | undefined {
  if (describing?.phase === "written") {
    const one = describing.written[band.manifestId];
    if (one === undefined) return undefined;
    if (one.state === "running") return { state: "writing" };
    if (one.state === "failed") return one;
    return { state: "query", query: one.query.query, fields: one.query.row_fields, written: true };
  }
  if (choice === OPEN) return { state: "query", query: band.tracker.open, fields: BUILT_IN_FIELDS };
  if (choice === ASSIGNED) return { state: "query", query: band.tracker.assigned, fields: BUILT_IN_FIELDS };
  const saved = band.tracker.saved.find((one) => one.name === savedName(choice));
  return saved === undefined ? undefined : { state: "query", query: saved.query, fields: saved.row_fields, saved };
}
