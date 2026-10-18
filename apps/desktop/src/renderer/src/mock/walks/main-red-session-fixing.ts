// A Session holds main's red. The Fixing row names it by the hammer, the Session's
// icon and its title, which opens it. The band offers no way to dispatch or send back while it holds the red.

import { button, inside, region, role, viaPalette, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));

const SESSION = "Show who merged on the head";

export const mainRedSessionFixing = walk("main-red-session-fixing", [
  ...viaPalette("Merge line", "The merge line"),
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green" },
  { later: inside(MERGE, role("img", "Main is green")), say: "A Job's pull request merges, and Fleet wakes the Job" },
  { look: RED, say: "Main is red" },
  { look: inside(RED, role("img", "Working on it")), say: "Fixing: the hammer" },
  { look: inside(RED, button(SESSION)), say: "The Job that holds it, by its title: it opens the Job" },
  { hover: inside(RED, role("img", "Job")), say: "A Job, by its icon" },
  { later: inside(RED, role("img", "Working on it")), say: "The Job fixes it" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green again" },
]);
