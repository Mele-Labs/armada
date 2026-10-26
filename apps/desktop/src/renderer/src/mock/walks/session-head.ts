// A Session's head on one row: the state mark with the address in its tooltip, a one-line title, the More menu holding Retro and End
// session, and the context as a tank. The tank at 5%, climbing to 84% where its tone
// turns, a second Session at 72%, and the composer's Model select reading "Opus 5.5". At 1512 by 949 with a Job
// open, its Blueprint and Field beside it. Over the `session-head` scenario.

import { button, inside, place, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Job s-e07cfe09");
const other = region("Job s-b4a1d2c7");
const sessions = region("Jobs", { exact: true });
const context = inside(head, role("button", "Context"));

const sessionHead = walk(
  "session-head",
  [
    toSessions,
    { press: inside(sessions, role("button", /How does armada interface/)), say: "The Job, its Blueprint and Field beside it" },
    { hover: inside(head, role("img", /s-e07cfe09/)), say: "The address is the state mark's tooltip" },
    { look: inside(head, role("button", /, rename$/)), say: "The title is one line, the whole of it on hover" },
    { hover: inside(head, button("More")), say: "Retro and End job are in the More menu" },
    { look: context, say: "The context at 5%" },
    { hover: context, say: "The breakdown card" },
    { later: context, say: "Later" },
    { look: context, say: "84%, in the warning tone" },
    { look: inside(head, role("img", /^Working/)), say: "The state mark stands first" },
    { look: role("combobox", "Model", { exact: true }), say: "The model is the composer's, read in full" },
    { press: place("Cockpit"), say: "Back to the list" },
    { press: inside(sessions, role("button", /Cart totals rounding/)), say: "The other Job" },
    { look: inside(other, role("button", "Context")), say: "72%, with its breakdown" },
  ],
  { width: 1512, height: 949 },
);

export { sessionHead as "session-head" };
