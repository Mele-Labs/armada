// A subagent stands on the plan group the agent named in its brief. Over the `subagents-on-plan-groups` scenario.

import { button, deckRow, walk } from "../walk";

const subagentsOnPlanGroups = walk("subagents-on-plan-groups", [
  {
    press: deckRow("Rotate the audit log"),
    say: "A Job mid-build. Its agent started three subagents and named a plan group in two of their briefs",
  },
  {
    look: button("Report the rotation"),
    say: "The protocol engineer is on Report the rotation, the group its brief named",
  },
  {
    look: button("Show the rotation"),
    say: "The Bridge engineer is on Show the rotation. The explorer named no group, so it is on none",
  },
]);

export { subagentsOnPlanGroups as "subagents-on-plan-groups" };
