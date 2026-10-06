// Workflows' members: the creator's own Fleet, serving the list, a definition and a save from the fixture.

import type { WorkflowsApi, WorkflowsState } from "../../../../shared/api/workflows";
import { WORKFLOWS_NOTHING_YET } from "../../../../shared/api/workflows";
import type { Slice } from "../fake-context";
import { workflowing, workflowsServed } from "../workflows-fleet";

export const workflows: Slice<WorkflowsApi, WorkflowsState> = {
  name: "workflows",
  state: WORKFLOWS_NOTHING_YET,
  scenarios: () => [workflowing()],
  api: (_scenario, { state }) =>
    workflowsServed(() => state().holds.repositories?.find((one) => one.root === state().repository)?.manifest?.id ?? null),
};
