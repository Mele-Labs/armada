// One Job's Artifacts drawn as a graph of how it grew, each thing hanging off what produced it. Over the `sessions` scenario with the Job and
// its child Job open beside the usual ones. The walk `artifact-graph` plays it. Built in `../sessions/artifact-graph.ts`.

import type { Scenario } from "../moment";
import { graphChild, graphHistory, graphJob } from "../sessions/artifact-graph";
import { s200Sessions } from "./sessions";

export const s320ArtifactGraph: Scenario = {
  ...s200Sessions,
  name: "artifact-graph",
  says: "A Job's Artifacts as a graph, each thing hanging off what produced it",
  behaves: () => ({ readAttachmentHistory: async () => ({ ok: true, value: graphHistory() }) }),
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [graphJob(), graphChild()]) },
};
