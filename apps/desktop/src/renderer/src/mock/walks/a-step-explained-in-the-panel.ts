// A step he has never met, opened at the gate (5 Oct 2026): its panel is the
// one every other tab opens, over the work area, and it opens on what the step
// does for the Job before the settings. The Frame step of `prototype`; the
// workflow files carry the line as `about`.

import { button, card, dialog, inside, region, role, text, walk } from "../walk";

const FRAME = dialog("Frame");

export const aStepExplainedInThePanel = walk("proto/feature-at-approval", [
  { look: region("What you are approving"), say: "A Job at its gate, on a workflow he has not used" },
  { type: "prototype", into: role("combobox", "Workflow", { exact: true }), say: "The prototype workflow" },
  { press: card("Frame"), say: "Its first step, Frame" },
  { look: FRAME, say: "Its panel, docked on the right like every other" },
  {
    look: inside(FRAME, text("Writes three lines on what will be tried and how, before any code. Build works from them.")),
    say: "What the step does, above its settings",
  },
  { type: "opus", into: inside(FRAME, role("combobox", "Model on Frame")), say: "Frame runs on opus" },
  { press: card("Build"), say: "Another node, with the panel still open" },
  { look: dialog("Build"), say: "The panel is Build's now, without closing first" },
  { press: inside(dialog("Build"), button("Close")), say: "Closed, and the canvas is as it was" },
  { look: inside(card("Frame"), text("opus")), say: "The node carries the model" },
]);
