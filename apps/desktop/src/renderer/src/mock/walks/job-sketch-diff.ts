// Picking an option draws its sketch against the current one: added green, removed red, changed amber, the rest dimmed.

import { inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-sketch-plan", [
  { press: inside(now, role("radio", "Split it out")), say: "Pick an option and its sketch is read against how it is now" },
  { look: role("group", "Added, Clock"), say: "What it adds is green" },
  { look: role("group", "Changed, Writer"), say: "What it alters is amber, and the rest stands back" },
  { press: inside(now, role("radio", "Wrap it in place")), say: "Another option" },
  { look: role("button", "Removed, append"), say: "What it drops is red" },
]);

export { w as "job-sketch-diff" };
