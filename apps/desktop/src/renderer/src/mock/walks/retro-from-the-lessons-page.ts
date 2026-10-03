// The Lessons page (23.12, `docs/concepts/retro.md`): what got in the way
// across Jobs, newest first, whose way a mark its tooltip names, and a row
// opening its Job's retro — Job 3's, with the record rows each item cites and
// the owner's notes. `retro.test.tsx` holds the claims.

import { button, dialog, inside, region, role, row, text, walk } from "../walk";

const LESSONS = region("Lessons");
const RETRO = dialog("Retro");

export const retroFromTheLessonsPage = walk("retro/lessons", [
  { press: button("Lessons", { exact: true }), say: "Lessons, under Work" },
  { look: LESSONS, say: "Newest first, across Jobs" },
  { hover: inside(row(/out_of_bounds on armada\.yml/), role("img", "Fleet")), say: "Whose way: hover names it" },
  { press: row(/out_of_bounds on armada\.yml/), say: "A row opens its Job's retro" },
  { look: inside(RETRO, text(/4e1c2a9 on main/)), say: "The record row it cites" },
  { look: inside(RETRO, text("helm via http")), say: "Helm's acts, through curl" },
  { look: inside(RETRO, text(/Allow pnpm/)), say: "The command you were asked to allow" },
  { look: inside(RETRO, region("Notes")), say: "Notes left with Job 3 open" },
]);
