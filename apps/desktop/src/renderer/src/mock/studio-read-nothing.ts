// A Studio holding one Link to a page whose read-in comes back with nothing to place: the scout
// reads it, answers, and asks for no node, so Fleet lands one Note off the Finding saying so. The
// owner, 2 Oct 2026, over a toast he looked at and did not want.

import type { Studio } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { READS_AS_NOTHING, studying, type StudioFleet } from "./studio-fleet";

const AT = "2026-10-02T09:00:00Z";

export const NOTHING_STUDIO_NAME = "The glossary, read in";

function aLinkToNothing(): Studio {
  return {
    id: "01STUDIONOTHING000000000000",
    manifest_id: repository().manifest!.id,
    name: NOTHING_STUDIO_NAME,
    named_by: "person",
    created_at: AT,
    touched_at: AT,
    nodes: [
      { id: "nothing-link", kind: "link", address: READS_AS_NOTHING, position: { x: 0, y: 0 }, created_at: AT, added_by: "person" },
    ],
    edges: [],
  };
}

/** The `studio-read-nothing` scenario: one Link, whose read-in finds nothing. */
export function readingNothing(): StudioFleet {
  const fleet = studying([aLinkToNothing()]);
  return {
    ...fleet,
    scenario: {
      ...fleet.scenario,
      name: "studio-read-nothing",
      says: "A Link whose read-in comes back with nothing to place",
    },
  };
}
