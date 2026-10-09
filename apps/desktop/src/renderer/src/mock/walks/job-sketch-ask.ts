// A press on a part of the sketch opens a small ask about it, and what is asked lands under the Plan question.

import { inside, region, role, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-sketch-plan", [
  { press: role("group", "Writer"), say: "Press a part of the sketch" },
  { type: "Does it still flush on close?\n", into: role("textbox", "Ask about Writer"), say: "Ask about it" },
  { look: inside(now, text("Does it still flush on close?")), say: "The question sits under the Plan question, named for its part" },
]);

export { w as "job-sketch-ask" };
