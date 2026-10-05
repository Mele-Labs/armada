// Overview over 150 Jobs on a timeline: a lane to each family of Jobs, the playhead dragged back
// through the day, and the card of a Job read by pointing at it.

import { button, role, walk } from "../walk";

export const overviewOnATimeline = walk("overview/timeline", [
  { press: button("Timeline", { exact: true }), say: "A lane to each family: a root Job and everything it dispatched" },
  { look: role("img", /dispatched from/), say: "A child branches off its dispatcher's row at the instant it was minted" },
  { drag: role("slider", "Playhead"), by: { x: -900, y: 0 }, say: "Scrub back through the day: Jobs not yet minted are rings, the rest show the state they had" },
  { press: button("More time", { exact: true }), say: "Widen the window to more of the day" },
  { press: button("More time", { exact: true }), say: "Wider again: the whole day, every family" },
  { hover: button(/^Move the poke loop onto the new queue, /), say: "Point at a Job and its card is shown below the lanes" },
  { look: button("Kill", { exact: true }), say: "The list's acts, on the card" },
]);
