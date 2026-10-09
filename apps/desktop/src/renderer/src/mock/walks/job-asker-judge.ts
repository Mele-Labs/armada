// A Judge question with no sketch: its own read, then the work product beside the checks it is reading.

import { role, walk } from "../walk";

const w = walk("job-asker-judge", [
  { look: role("group", "Asker", { exact: true }), say: "The Judge's read, where the canvas was" },
  { look: role("region", "Work product"), say: "The work product" },
  { look: role("region", "Checks it is reading"), say: "beside the checks it reads" },
]);

export { w as "job-asker-judge" };
