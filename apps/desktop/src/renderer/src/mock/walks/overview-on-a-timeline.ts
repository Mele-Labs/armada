// Overview over 150 Jobs on a timeline: the playhead dragged back through the day, cards laid out by
// start time, and an edge from each Job to what it dispatched.

import { button, role, walk } from "../walk";

export const overviewOnATimeline = walk("overview/timeline", [
  { press: button("Timeline", { exact: true }), say: "The same cards, left to right by when each started" },
  { drag: role("slider", "Playhead"), by: { x: -400, y: 0 }, say: "Scrub back through the day: cards not yet started fall faint, the rest show the state they had" },
  { press: button("More time", { exact: true }), say: "Widen the window to more of the day" },
  { press: button("More time", { exact: true }), say: "Wider again: the whole day, every Job" },
  { look: button("Kill", { exact: true }), say: "The list's acts, on the card" },
  { look: role("group", /dispatched from/), say: "An edge from the parent card to what it dispatched" },
]);
