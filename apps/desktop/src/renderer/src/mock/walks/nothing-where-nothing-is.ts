// An empty slot stays empty — the owner's standing rule, swept on 2 Oct 2026. A Job still being
// proposed has nothing yet almost everywhere, and every place that once filled the gap with a
// sentence ("No brief was written.", "The proposer has not chosen a workflow yet.", "No plan has
// been recorded.", "No Drone has submitted evidence on this Job.", "Reading the machine.",
// "Ask Helm about this repository.") now draws nothing. Facts and failures still speak.

import { button, region, role, tab, walk } from "../walk";

export const nothingWhereNothingIs = walk("arc/proposing-reading", [
  { look: region("Brief"), say: "Brief: nothing written yet, and no sentence saying so" },
  { look: region("Workflow"), say: "Workflow: none chosen yet, so the head alone" },
  { look: region("Plan"), say: "Plan: none recorded yet, the head alone again" },
  { press: tab("Workflow"), say: "The Workflow tab" },
  { look: role("tabpanel", "Workflow"), say: "No steps to draw, and nothing standing in for them" },
  { press: tab("Record"), say: "The Record" },
  { press: button("All", { exact: true }), say: "Its filter" },
  { press: role("menuitem", "Evidence"), say: "Evidence, which holds nothing yet" },
  { look: region("What the Record holds"), say: "The filter stays, and nothing is drawn under it" },
  { press: tab("Pulse"), say: "Pulse" },
  { look: region("Stats"), say: "No machine reading yet: the Job's own figures, and no sentence" },
  { press: button("Helm"), say: "Helm" },
  { look: role("complementary", "Helm"), say: "An empty thread draws nothing above the composer" },
]);
