// A picture sent with a message opens larger when pressed, and Esc puts it back. Over the
// `enlarge-a-picture` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

const { thread } = kit(false);
const sessions = region("Jobs", { exact: true });
const picture = button("header-screenshot.png");

const steps: Step[] = [
  { press: inside(sessions, button(/Header looks wrong/)), say: "A Job where a picture was sent" },
  { look: inside(thread, picture), say: "The picture, small, in the message" },
  { press: inside(thread, picture), say: "Pressed, it opens larger over the thread" },
  { key: "Escape", on: role("dialog", "header-screenshot.png"), say: "Esc closes it and focus goes back to the picture" },
];

const enlargeAPicture = walk("enlarge-a-picture", [toSessions, ...steps]);

export { enlargeAPicture as "enlarge-a-picture" };
