// Hovering an option draws its sketch against the current one: added green, removed red, changed amber, the rest dimmed.

import { inside, region, role, walk } from "../walk";

const now = region("Now");

const w = walk("job-sketch-plan", [
  { hover: inside(now, role("radio", "Split it out")), say: "The option's sketch, read against how it is now" },
  { look: role("group", "Added, Clock"), say: "What it adds is green" },
  { look: role("group", "Changed, Writer"), say: "What it alters is amber" },
  { hover: inside(now, role("radio", "Wrap it in place")), say: "Another option" },
  { look: role("button", "Removed, append"), say: "What it drops is red, and the rest stands back" },
]);

export { w as "job-sketch-diff" };
