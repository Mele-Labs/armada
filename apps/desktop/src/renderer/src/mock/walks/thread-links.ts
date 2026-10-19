// A Job's thread links what it names. A Job's id opens that Job, with its title on hover; the Job's own pull
// request opens its panel. Over the `thread-links` scenario.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, place, region, role, text, walk } from "../walk";

const thread = region("Thread");
const jobs = region("Jobs", { exact: true });
const follow = inside(jobs, button(/Store retry follow-up/));

const links = walk("thread-links", [
  toSessions,
  { press: follow, say: "A Job whose thread names another Job, an issue and a pull request" },
  { look: inside(thread, role("link", "#2209", { exact: true })), say: "The pull request and the issue read as links in the thread" },
  { hover: inside(thread, role("link", "s-9e2e41a1", { exact: true })), say: "A Job id names the Job on hover" },
  { press: inside(thread, role("link", "s-9e2e41a1", { exact: true })), say: "It opens that Job" },
  { look: inside(thread, text("Traced and written up")), say: "Its thread" },
  { press: place("Cockpit"), say: "Back to the list" },
  { press: follow, say: "The first Job again" },
  { press: inside(thread, role("link", "#2209", { exact: true })), say: "Its own pull request opens the panel" },
  { look: role("dialog", "Pull request #2209"), say: "The panel, over the thread" },
]);

export { links as threadLinks };
