// Fleet's write-clash notice where the other Job is paused: it names that Job's branch and says to resolve the conflict
// on merge, with no message to send. Over the `write-clash-paused` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { thread } = kit(false);
const jobs = region("Jobs", { exact: true });

const writeClashPaused = walk("write-clash-paused", [
  toSessions,
  { look: role("option", "Demo marketing video, Job, Paused"), say: "The other Job, paused" },
  { press: inside(jobs, button("Thread, Browser and Terminal Split")), say: "The Job whose changes clash with it" },
  { press: inside(thread, button(/conflict with branch mock\/video-multitasking-boards/)), say: "Fleet's notice names the paused Job's branch" },
  { look: inside(thread, text(/resolve the conflict on merge/)), say: "And says to resolve the conflict on merge, not to message a Job that cannot answer" },
]);

export { writeClashPaused as "write-clash-paused" };
