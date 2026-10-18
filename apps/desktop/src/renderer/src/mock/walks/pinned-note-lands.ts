// A note pinned on the Usage mock, in the Job it went to: his words alone in the thread, and the reply
// naming the line Fleet found. Over the `pinned-note-lands` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, text, walk } from "../walk";
import type { Step } from "../walk";

const { thread } = kit(false);
const work = region("Jobs", { exact: true });

const steps: Step[] = [
  { press: inside(work, button(/Usage page/)), say: "The Job that showed the Usage page" },
  { look: inside(thread, text("this bar is too faint")), say: "His note, the words he typed" },
  { look: inside(thread, text(/UsagePage\.tsx:171/)), say: "The reply opens at the line Fleet found" },
];

const pinnedNoteLands = walk("pinned-note-lands", [toSessions, ...steps]);

export { pinnedNoteLands };
