// Moving a stuck Job to a later step. The Job is on the bug workflow at Fix, with Tests and Deliver to come: Jump here on Tests skips Fix and makes Tests
// the current step. Over the `jump-to-step` scenario.

import { button, deckRow, inside, region, text, walk } from "../walk";

const fix = region("Fix", { exact: true });
const tests = region("Tests", { exact: true });
const deliver = region("Deliver", { exact: true });

export const jumpToStep = walk("jump-to-step", [
  { press: deckRow("Rotate the audit log"), say: "Open a Job on the bug workflow. It is at Fix, with Tests and Deliver to come" },
  { hover: inside(tests, button("Jump here", { exact: true })), say: "Each step to come has a Jump here icon in its band, beside Skip" },
  { look: inside(deliver, button("Jump here", { exact: true })), say: "Deliver has one too, and no Skip" },
  { press: inside(tests, button("Jump here", { exact: true })), say: "Jump to Tests" },
  { look: inside(fix, text("Skipped")), say: "Fix stays on the spine as skipped, and Tests is the current step" },
]);
