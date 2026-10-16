// A form edit of `armada.yml` went out as pull request #2405 and the forge would not turn auto-merge on, so it waits for a person.
// Fleet records that on the edit (`Pending::merge_by_hand`, #2396) and the hub's row for the pull request carries `merge_by_hand`.
// Bridge draws it three ways over the flight deck's ship: a call in Incoming calls, a person's glyph on the pull request in the merge
// queue, and a line under Save on the Manifest's form. The walk `mergeByHand` plays it.

import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

export const s261MergeByHand: Scenario = (() => {
  const lines = ship.state.mergeLines!;
  const edit = {
    number: 2405,
    title: "armada.yml: auto_merge checks-pass",
    branch: "armada-yml/auto-merge-checks-pass",
    url: `${FORGE}2405`,
    ci: "passed",
    opened_at: ago(6),
    merge_by_hand: true,
  };
  return {
    ...ship,
    name: "merge-by-hand",
    says: "The flight deck with a Manifest form edit's pull request the forge would not auto-merge, in Incoming calls, on the merge queue and under the form's Save",
    state: {
      ...ship.state,
      mergeLines: {
        lines: lines.lines.map((line, at) => (at !== 0 ? line : { ...line, hub: { ...line.hub, pull_requests: [edit, ...(line.hub?.pull_requests ?? [])] } })),
      },
    },
    behaves: (handle) => ({ ...ship.behaves?.(handle), readManifestFile: manifesting().behaves?.(handle).readManifestFile! }),
  };
})();
