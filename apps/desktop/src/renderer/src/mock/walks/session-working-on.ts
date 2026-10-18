// What a Session is doing. A Session whose agent keeps a todo list also has Working on over the box, folded
// to one glyph a todo with the one being done first, beside Needs you where the Session waits on the
// person. The Cockpit tile reads the todo being done, or the line where there is no list. Nothing is drawn for a Session that is done. Over `session-working-on`.

import { button, inside, place, region, role, tab, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", { exact: true });
const working = region("Working on");
const glass = role("listbox", "Tiles");
const open = (title: RegExp) => inside(sessions, button(title));

const sessionWorkingOn = walk("session-working-on", [
  toSessions,
  { press: open(/Gate policy for every run/), say: "Opened" },
  { look: working, say: "Working on, folded over the box. The todo being done leads" },
  { hover: inside(working, role("img", /^Working: Run the policy on every pull request/)), say: "Each todo is a glyph, named on hover" },
  { press: inside(working, button(/^Working on/)), say: "A press opens the rows" },
  { look: inside(working, text("Cover a second run in the gate tests")), say: "The todo being done first, then those still to do, then those done" },

  toSessions,
  { press: open(/Pin the store clock/), say: "A Job that also waits on the person" },
  { look: working, say: "Working on" },
  { look: region("Needs you"), say: "Needs you beside it, each folded to its own glyphs" },

  toSessions,
  { press: open(/Order the store migrations/), say: "A Job that is done" },
  { look: region("Nothing left to do"), say: "The Job end band, and no Working on band" },

  { press: place("Cockpit"), say: "Back to the Cockpit" },
  { press: tab("Active"), say: "Active" },
  { press: button("Grid"), say: "Grid" },
  { look: inside(glass, text("Run the policy on every pull request")), say: "A tile reads the todo being done" },
  { look: inside(glass, text("Running cargo check")), say: "With no todo list, the line Fleet detected" },
]);

export { sessionWorkingOn as "session-working-on" };
