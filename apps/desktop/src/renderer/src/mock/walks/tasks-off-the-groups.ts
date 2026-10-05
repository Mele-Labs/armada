// The running canvas with each group's tasks hung below it (prototype, 5 Oct
// 2026): a short chain under every group, rejoining above the gate, and a
// task opening Plan's own panel with the way back to the run.

import { button, card, region, role, text, walk } from "../walk";

const RUN = region("This Job's run");
const T6 = role("button", /^Draw tasks under their group, /);

export const tasksOffTheGroups = walk("proto/feature-running", [
  { look: RUN, say: "The groups stay a row; each one's tasks hang below it in plan order" },
  { look: role("button", /^Answer restart and move in the mock, /), say: "T4 is at work under Group 3" },
  { look: role("button", /^Name the group on a Check run, /), say: "Group 1's chain is two done tasks" },
  { look: card("Checks"), say: "The chains rejoin above the gate" },
  { press: T6, say: "A task opens Plan's panel" },
  { look: text("Draw tasks under their group"), say: "T6, its own panel" },
  { look: button("Back to Overview"), say: "with the way back to the run" },
  { press: button("Back to Overview"), say: "Back" },
]);
