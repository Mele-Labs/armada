// What happened across the Fleet: the wire is answered by Fleet alone, so the mock has none for it. A
// scenario hands its events to the Cockpit's timeline through `draft.events` instead (`mount.tsx`).

import type { TimelineApi } from "../../../../shared/api/timeline";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const timeline: Slice<TimelineApi, Record<string, never>> = {
  name: "timeline",
  state: {},
  api: () => ({
    getFleetTimeline: async () => ({ ok: false, outcome: unanswered("/timeline") }),
    onTimelineAppended: () => () => {},
  }),
};
