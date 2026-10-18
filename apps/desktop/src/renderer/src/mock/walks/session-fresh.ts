// What a Session runs on, and a fresh start once it is big or old. The model is read in the composer's Model
// select, the version its process resolved. A Session over 300k of context, or past a day
// old, holds two acts in its More menu: New Job, which opens on a short handoff, and Fork.
// Over the `session-fresh` scenario.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, place, region, role, text, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const more = inside(region(/^Job s-01FRESH/), button("More"));
const menu = role("menu");
const model = role("combobox", "Model", { exact: true });

const fresh = walk("session-fresh", [
  toSessions,
  { press: inside(sessions, button(/Release notes script/)), say: "Small and new" },
  { look: model, say: "The model its process resolved, in the composer" },
  { look: role("img", "Context 5%"), say: "53.8k of the window, and no fresh start in More" },
  { press: place("Cockpit"), say: "Back to the list" },
  { press: inside(sessions, button(/Invoice PDF layout/)), say: "Past a day old, at 88k" },
  { look: model, say: "A picked alias, resolved to its version" },
  { press: more, say: "Past a day old, so More holds a fresh start" },
  { look: menu, say: "New Job and Fork" },
  { hover: inside(menu, role("menuitem", "Fork")), say: "Fork starts a new Job from this conversation" },
  { press: inside(menu, role("menuitem", "Fork")), say: "Fork" },
  { look: region("Job s-01FORK00"), say: "It opens at once" },
  { press: place("Cockpit"), say: "Back to the list" },
  { press: inside(sessions, button(/Cart totals rounding/)), say: "612k, started hours ago" },
  { look: model, say: "Opus again" },
  { look: role("img", "Context 61%"), say: "The context figure" },
  { press: more, say: "Context alone puts it in More" },
  { hover: inside(menu, role("menuitem", "New Job")), say: "A new Job that opens on a short handoff" },
  { press: inside(menu, role("menuitem", "New Job")), say: "New Job asks the old one for its handoff" },
  { look: region("Job s-01SESSIO"), say: "The new Job opens once the handoff is written" },
  { look: inside(region("Thread"), text("Handoff from Cart totals rounding")), say: "Its first message, sent for you" },
  { look: inside(region("Thread"), text("the same rounding in the refund path")), say: "The old Job's own words" },
]);

export { fresh as "session-fresh" };
