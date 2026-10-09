// A Drone question with no sketch: the left shows the asker's live output and what it changed, the file it asks about marked.

import { inside, region, role, walk } from "../walk";

const w = walk("job-asker-drone", [
  { look: role("group", "Asker", { exact: true }), say: "The asking Drone, where the canvas was" },
  { look: role("img", "Asking about this"), say: "The file it asks about is marked" },
  { press: role("button", "crates/store/tests/fixtures.rs"), say: "Press a file to open it" },
  { look: inside(region("Now"), role("button", "Canvas")), say: "The panel's switch offers the canvas back" },
]);

export { w as "job-asker-drone" };
