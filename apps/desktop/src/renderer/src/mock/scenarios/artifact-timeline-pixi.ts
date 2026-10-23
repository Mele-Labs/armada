// The Job of `artifact-timeline` drawn on a canvas, zoomed and panned, instead of as the spine. The walk `artifact-timeline-pixi` plays it.

import type { Scenario } from "../moment";
import { timelineChild, timelineJob } from "../sessions/artifact-timeline";
import { s200Sessions } from "./sessions";

export const s322ArtifactTimelinePixi: Scenario = {
  ...s200Sessions,
  name: "artifact-timeline-pixi",
  says: "A Job's Artifacts along time, drawn on a canvas with zoom and pan",
  draft: { sessions: (board) => ({ ...s200Sessions.draft!.sessions!(board, [timelineJob(), timelineChild()]), timelineTake: "pixi" as const }) },
};
