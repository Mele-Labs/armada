// A step that ends on any of several things: the feature workflow's Propose lists its four under Ends when, and a sketch
// put to him in a Job's thread is approved or sent back on its card, after which the Job moves on to Discuss. Over the
// `step-ends-any` scenario.

import { button, deckRow, inside, region, role, text, toSystemPage, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const ARMADA = role("group", "Armada", { exact: true });
const propose = role("group", "Propose", { exact: true });
const ends = inside(propose, role("list", "Ends when"));
const card = role("article", "Sketch: Pause on the row", { exact: true });

const stepEndsAny = walk("step-ends-any", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { press: inside(ARMADA, button("feature", { exact: true })), say: "The feature workflow" },
  { look: ends, say: "Propose ends when any one of four happens" },
  { look: inside(ends, text("you approve a walk")), say: "You approve a walk" },
  { look: inside(ends, text("you approve a sketch")), say: "You approve a sketch" },
  { look: inside(ends, text("you approve a draft")), say: "You approve a draft" },
  { look: inside(ends, text("you pick one of the ideas shown")), say: "You pick one of the ideas shown" },
  toSessions,
  { press: deckRow("Pause a Job from its row"), say: "A Job on Propose with a sketch in its thread" },
  { look: inside(card, role("group", "Answers")), say: "Approve and Send back under the sketch" },
  { press: inside(card, button("Approve", { exact: true })), say: "Approve" },
  { look: inside(card, role("img", "Approved")), say: "The sketch is approved" },
  { look: inside(region("Propose", { exact: true }), role("img", "Done")), say: "Propose is done" },
  { look: region("Discuss", { exact: true }), say: "The Job is on Discuss" },
]);

export { stepEndsAny as "step-ends-any" };
