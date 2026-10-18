// Outposts at the far edge of the Cockpit map, one for each service a Session has called outside the repository. Over the `field-outposts`
// scenario, where three services have been called and each `later` has a Session call one now.

import { inside, role, walk } from "../walk";

const map = role("listbox", "Map");
const outpost = (name: string) => inside(map, role("option", name, { exact: true }));

const fieldOutposts = walk("field-outposts", [
  { look: map, say: "Past the far edge of the field, an outpost for each service a Job has called: GitHub, the issue tracker and the web. None is lit" },
  { later: map, say: "Gate policy on every run calls GitHub with gh pr view" },
  { look: map, say: "A beam runs from its drone to the GitHub outpost, and the outpost is lit. The drone stays where it is" },
  { hover: outpost("GitHub"), say: "Its card lists the recent calls, newest first: the Job, the line and how long ago" },
  { later: map, say: "Pin the store clock calls Figma with get_design_context" },
  { press: outpost("Figma"), say: "Figma has an outpost now, lit. Each outpost is a stop for Tab after the flags, and focus opens its card" },
]);

export { fieldOutposts as "field-outposts" };
