// Workflows' members: the creator's own Fleet, serving the list, a definition and a save from the fixture.

import type { WorkflowsApi, WorkflowsState } from "../../../../shared/api/workflows";
import { WORKFLOWS_NOTHING_YET } from "../../../../shared/api/workflows";
import type { Slice } from "../fake-context";
import { doctoring, workflowing, workflowingOnAll, workflowsServed } from "../workflows-fleet";

export const workflows: Slice<WorkflowsApi, WorkflowsState> = {
  name: "workflows",
  state: WORKFLOWS_NOTHING_YET,
  scenarios: () => [workflowing(), workflowingOnAll(), doctoring()],
  api: (_scenario, { state, publish }) =>
    workflowsServed(() => state().holds.repositories?.find((one) => one.root === state().repository)?.manifest?.id ?? null, {
      // A job started to build what a change lacks is listed as Fleet would list it.
      started: (record) => {
        const was = state().sessions;
        publish({ sessions: { state: "read", sessions: [...(was.state === "read" ? was.sessions : []), record] } });
      },
    }),
};
