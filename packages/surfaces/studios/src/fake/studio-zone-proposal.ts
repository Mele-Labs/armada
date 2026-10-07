// A Zone with room in it, and a relation you proposed whose line runs straight across a Note — the
// case the owner's screenshot of 2 Oct 2026 showed, where "You proposed · blocks · Continue to
// accept or reject" sat on a Note and hid its text. `walks/four-studio-fixes.ts` plays it.

import type { Studio, StudioNode } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { studying, type StudioFleet, type StudyingApi, type StudyingState } from "./studio-fleet";

const AT = "2026-10-02T09:00:00Z";

/** Fleet's frame inset and head, and a read-in's column and row pitch. */
const INSET = 24;
const HEAD = 48;
const ACROSS = 340;
const DOWN = 180;

export const ZONE_PROPOSAL_NAME = "The legend, ringed off";

const ZONE = "legend-zone";

const note = (id: string, said: string, column: number, row: number): StudioNode => ({
  id,
  kind: "note",
  said,
  within: ZONE,
  position: { x: INSET + column * ACROSS, y: HEAD + row * DOWN },
  created_at: AT,
  added_by: "person",
});

/** Three Notes in a row, the outer two joined across the middle one, and a fourth two rows down. */
export function zoneWithAProposal(): Studio {
  return {
    id: "01STUDIOZONEPROPOSAL00000000",
    manifest_id: repository().manifest!.id,
    name: ZONE_PROPOSAL_NAME,
    created_at: AT,
    touched_at: AT,
    nodes: [
      { id: ZONE, kind: "zone", position: { x: 0, y: 0 }, created_at: AT, added_by: "person" },
      note("legend-wraps", "The legend wraps at 720 wide", 0, 0),
      note("legend-unreadable", "The step bar's legend is unreadable in View", 1, 0),
      note("legend-ship", "Ship the step bar's new legend", 2, 0),
      note("legend-belongs", "Ask whether the legend belongs in the step bar at all", 0, 2),
    ],
    edges: [
      {
        id: "legend-blocks",
        from: "legend-wraps",
        to: "legend-ship",
        kind: "blocks",
        standing: "proposed",
        created_at: AT,
        added_by: "person",
      },
    ],
  };
}

/** The `studio-zone-proposal` scenario: that one Studio, and a checkout declaring things to run. */
export function zoneProposing<S extends StudyingState, A extends StudyingApi>(nothingYet: S): StudioFleet<S, A> {
  const fleet = studying<S, A>(nothingYet, [zoneWithAProposal()]);
  return {
    ...fleet,
    scenario: {
      ...fleet.scenario,
      name: "studio-zone-proposal",
      says: "A Zone with room in it, and a relation proposed across a Note",
    },
  };
}
