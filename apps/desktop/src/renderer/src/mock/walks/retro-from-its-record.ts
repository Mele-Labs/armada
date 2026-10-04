// A Job's own retro, from its detail: the Record's head carries Retro, and it
// opens the sheet the Lessons page opens, each item with where its fix lands.
// Then on to Lessons, narrowed by where fixes land. Job 3; `retro.test.tsx`
// holds the claims.

import { button, dialog, inside, role, tab, text, walk } from "../walk";

const RETRO = dialog("Retro");

export const retroFromItsRecord = walk("retro/job-3", [
  { press: tab("Record"), say: "Job 3's Record" },
  { press: button("Retro", { exact: true }), say: "Retro, in the Record's head" },
  { look: inside(RETRO, text(/fixed 15 s timeouts/)), say: "The same sheet Lessons opens" },
  { hover: inside(RETRO, role("img", "Lands in Kit")), say: "Where each fix lands, beside whose way" },
  { look: inside(RETRO, text("out_of_bounds")), say: "Each item with the rows it cites" },
  { press: inside(RETRO, button("Close")), say: "Back to the Record" },
  { press: button("Lessons", { exact: true }), say: "Every Job's items, on Lessons" },
  { press: tab("Kit"), say: "Kit: the tool set" },
  { press: tab("Manifest"), say: "Manifest: the repository's Checks, tests and code" },
]);
