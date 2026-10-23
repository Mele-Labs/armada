// The Job of `artifact-timeline` drawn in lanes, a row to each kind, instead of as a spine. The walk `artifact-timeline-lanes` plays it.

import type { Scenario } from "../moment";
import { timelineChild, timelineJob } from "../sessions/artifact-timeline";
import { s200Sessions } from "./sessions";

export const s321ArtifactTimelineLanes: Scenario = {
  ...s200Sessions,
  name: "artifact-timeline-lanes",
  says: "A Job's Artifacts along time, as a lane for each kind",
  draft: { sessions: (board) => ({ ...s200Sessions.draft!.sessions!(board, [timelineJob(), timelineChild()]), timelineTake: "lanes" as const }) },
};
