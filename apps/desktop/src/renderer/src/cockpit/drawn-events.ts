// Fleet's events named from what the window holds, for what draws them in the Cockpit.

import { useMemo } from "react";

import type { MergeLineView } from "@armada/screens";

import { useSessions } from "../sessions-draft";
import { fromWire, useFleetEvents, type FleetEvent } from "./fleet-events";
import { timelineOwner } from "./owner";

export function useDrawnEvents(views: readonly MergeLineView[]): readonly FleetEvent[] {
  const wire = useFleetEvents();
  const sessions = useSessions();
  return useMemo(() => {
    // Only a pull request's opening and its merge carry its branch, so the other entries read it from those.
    const branches = new Map(wire.flatMap((one) => (one.subject.kind === "pull_request" && one.detail !== undefined && (one.kind === "pr.opened" || one.kind === "pr.merged") ? [[one.subject.number, one.detail] as const] : [])));
    return wire.flatMap((one) => {
      const drawn = fromWire(one, {
        session: (id) => sessions.find((row) => row.id === id)?.title,
        pull: (number) => timelineOwner(views, sessions, number, branches.get(number)),
      });
      return drawn === undefined ? [] : [drawn];
    });
  }, [wire, views, sessions]);
}
