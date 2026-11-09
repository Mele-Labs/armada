// A subagent is a drone on the plan group the agent named in its brief: pulsing while it runs, still once it has finished, and a press opens its thread. Over the
// `subagents-on-plan-groups` scenario.

import { button, deckRow, walk } from "../walk";

const subagentsOnPlanGroups = walk("subagents-on-plan-groups", [
  {
    press: deckRow("Rotate the audit log"),
    say: "A Job mid-build. Its agent named a plan group in the brief of each subagent it started",
  },
  {
    look: button("Report the rotation"),
    say: "Each drone at the end of a group's title is a subagent on it. The pulsing one is at work; the still one has finished",
  },
  {
    hover: button("Subagent at work on Mirror it in the TypeScript types"),
    say: "Its tooltip says what it is doing",
  },
  {
    press: button("Subagent at work on Mirror it in the TypeScript types"),
    say: "A press opens the subagent's thread in a tab",
  },
]);

export { subagentsOnPlanGroups as "subagents-on-plan-groups" };
