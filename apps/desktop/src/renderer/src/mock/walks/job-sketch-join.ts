// Pick a box, press Join, then press another: his arrow joins them.

import { role, walk } from "../walk";

const w = walk("job-sketch-plan", [
  { press: role("group", "Caller"), say: "Pick a box" },
  { press: role("button", "Join", { exact: true }), say: "Join" },
  { press: role("group", "Record file"), say: "Then the one it goes to" },
  { look: role("button", "Yours, Arrow"), say: "His arrow, in his colour" },
]);

export { w as "job-sketch-join" };
