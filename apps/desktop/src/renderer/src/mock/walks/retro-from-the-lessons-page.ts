// The Lessons page (23.12, `docs/concepts/retro.md`): what got in the way
// across Jobs, newest first, whose way and where the fix lands as marks their
// tooltips name, tabs by where the fix lands, and a row opening its Job's retro
// — Job 3's, with the record rows each item cites and the owner's notes.
// `retro.test.tsx` holds the claims.

import { button, dialog, inside, region, role, row, tab, text, walk } from "../walk";

const LESSONS = region("Lessons");
const RETRO = dialog("Retro");
const FIRST = row(/out_of_bounds on armada\.yml/);

export const retroFromTheLessonsPage = walk("retro/lessons", [
  { press: button("Lessons", { exact: true }), say: "Lessons, under Work" },
  { look: LESSONS, say: "Newest first, across Jobs" },
  { hover: inside(FIRST, role("img", "Fleet")), say: "Whose way: hover names it" },
  { hover: inside(FIRST, role("img", "Lands in Armada")), say: "Where the fix lands: hover names it" },
  { press: tab("Kit"), say: "Kit: the tool set" },
  { press: tab("Manifest"), say: "Manifest: the repository's Checks, tests and code" },
  { press: tab("All"), say: "All, with the items from before" },
  { press: FIRST, say: "A row opens its Job's retro" },
  { look: inside(RETRO, text(/4e1c2a9 on main/)), say: "The record row it cites" },
  { look: inside(RETRO, text("helm via http")), say: "Helm's acts, through curl" },
  { look: inside(RETRO, text(/Allow pnpm/)), say: "The command you were asked to allow" },
  { look: inside(RETRO, region("Notes")), say: "Notes left with Job 3 open" },
]);
