// A Job's own retro, from its detail: the Record's head carries Retro, and it
// opens the sheet the Lessons page opens. Job 3; `retro.test.tsx` holds the claims.

import { button, dialog, inside, tab, text, walk } from "../walk";

const RETRO = dialog("Retro");

export const retroFromItsRecord = walk("retro/job-3", [
  { press: tab("Record"), say: "Job 3's Record" },
  { press: button("Retro", { exact: true }), say: "Retro, in the Record's head" },
  { look: inside(RETRO, text(/without ever seeing screens_test pass/)), say: "The same sheet Lessons opens" },
  { look: inside(RETRO, text("out_of_bounds")), say: "Each item with the rows it cites" },
]);
