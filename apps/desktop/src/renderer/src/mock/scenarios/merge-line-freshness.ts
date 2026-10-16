// How old the merge queue's reading of the forge is, and the forge going quiet: main's chip on the flight deck, one repository.
// Fleet's last visit answered 40 seconds before the walk starts, then at the next moment the forge stops answering and the visit that
// last answered is six minutes old. The walk `merge-line-freshness` plays it.

import type { MergeLine, MergeLines } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const ago = (seconds: number) => new Date(Date.now() - seconds * 1000).toISOString();

/** The ship's first repository alone, so main's chip speaks for one forge, with Fleet's last answered visit `seconds` ago. */
function lines(seconds: number, quiet: boolean): MergeLines {
  const [armada] = ship.state.mergeLines!.lines as [MergeLine];
  const read = ago(seconds);
  return { lines: [{ ...armada, hub: { ...armada.hub!, forge_read_at: read, ...(quiet ? { quiet_since: read } : {}) } }] };
}

export const mergeLineFreshness: Scenario = {
  ...ship,
  name: "merge-line-freshness",
  says: "Main's chip on the merge queue with the time Fleet last read the forge, then with the forge not answering",
  state: { ...ship.state, mergeLines: lines(40, false) },
  later: [{ mergeLines: lines(360, true) }],
};
