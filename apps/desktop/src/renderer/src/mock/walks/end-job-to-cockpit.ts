// Ending a Job leaves at once. The Job's pane plays out and the stage is the timeline with nothing selected, from the head's More
// menu and from the done band's End job; Fleet parks the Job a moment later and its row reads ended. Three exits, picked by
// `?take=fold`, `?take=slide` or `?take=settle` (fold is the one without it). Over the `session-all-done` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

// Whichever the stage was showing before the Job opened, the map or the timeline, is what it shows with nothing selected.
const timeline = region(/^(Fleet timeline|Flight map)$/);
const callout = region("Nothing left to do");

const endJobToCockpit = walk("session-all-done", [
  toSessions,
  { press: deckRow("Release notes script"), say: "A Job open on the stage" },
  { press: inside(role("region", /^Job: /), button("More")), say: "End job is behind the three dots" },
  { press: role("menuitem", "End job"), say: "The pane leaves and the stage is the timeline, with no Job selected. The row reads ended a moment after" },
  { look: timeline, say: "Nothing is open" },
  { press: deckRow("Pin the store clock"), say: "A Job whose agent has nothing left to do" },
  { press: inside(callout, button("End job")), say: "The same exit from the band" },
  { look: timeline, say: "Nothing is open" },
]);

export { endJobToCockpit as "end-job-to-cockpit" };
