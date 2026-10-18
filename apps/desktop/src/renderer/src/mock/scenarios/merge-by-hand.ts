// A form edit of `armada.yml` went out as pull request #2405 and the forge would not turn auto-merge on, so it waits for a person.
// Fleet records that on the edit (`Pending::merge_by_hand`, #2396) and the hub's row for the pull request carries `merge_by_hand`.
// Bridge draws it three ways over the flight deck's ship: a call, a person's glyph on the pull request in the merge
// queue, and a line under Save on the Manifest's form. The walk `mergeByHand` plays it.

import type { MergeLines } from "@armada/protocol";

import { manifesting } from "../manifest-fake";
import type { FleetHandle, Scenario } from "../moment";
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
  let fleet: FleetHandle | undefined;
  // Fleet merges it and the forge lists it as merged: the pull request leaves the open ones, and every mark of it goes.
  const merged = (now: MergeLines): MergeLines => ({
    lines: now.lines.map((line) => {
      const { merge_by_hand: _held, ...gone } = edit;
      const open = (line.hub?.pull_requests ?? []).filter((one) => one.number !== edit.number);
      return open.length === (line.hub?.pull_requests ?? []).length
        ? line
        : { ...line, hub: { ...line.hub, pull_requests: open, merged: [{ ...gone, merged_at: new Date().toISOString() }, ...(line.hub?.merged ?? [])] } };
    }),
  });
  return {
    ...ship,
    name: "merge-by-hand",
    says: "The flight deck with a Manifest form edit's pull request the forge would not auto-merge, a call, on the merge queue and under the form's Save",
    state: {
      ...ship.state,
      mergeLines: {
        lines: lines.lines.map((line, at) => (at !== 0 ? line : { ...line, hub: { ...line.hub, pull_requests: [edit, ...(line.hub?.pull_requests ?? [])] } })),
      },
    },
    draft: {
      ...ship.draft,
      sessions: (control) => {
        const store = ship.draft!.sessions!(control);
        return {
          ...store,
          // No Session holds this pull request, so the act is pressed on the repository; Merge is the press the walk makes.
          actInRepository: async (_root, number, what) => {
            if (number !== edit.number) return false;
            if (what === "merge") fleet?.publish({ mergeLines: merged(fleet.state().mergeLines!) });
            return true;
          },
        };
      },
    },
    behaves: (handle) => {
      fleet = handle;
      return { ...ship.behaves?.(handle), readManifestFile: manifesting().behaves?.(handle).readManifestFile! };
    },
  };
})();
