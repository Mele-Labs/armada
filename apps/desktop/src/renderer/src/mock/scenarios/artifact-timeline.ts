// One Job's Artifacts drawn along its two hours, the Job's own line through the middle with every artifact branching off it. Over the
// `sessions` scenario with the Job and its child Job open beside the usual ones. The walk `artifact-timeline` plays it; `artifact-timeline-lanes`
// is the same Job drawn in lanes. Built in `../sessions/artifact-timeline.ts`.

import type { Scenario } from "../moment";
import { timelineChild, timelineJob } from "../sessions/artifact-timeline";
import { s200Sessions } from "./sessions";

export const s320ArtifactTimeline: Scenario = {
  ...s200Sessions,
  name: "artifact-timeline",
  says: "A Job's Artifacts along time, as a spine with the artifacts branching off it",
  draft: { sessions: (board) => ({ ...s200Sessions.draft!.sessions!(board, [timelineJob(), timelineChild()]), timelineTake: "spine" as const }) },
};
