// A Session's head on one row: the terminal glyph with the address in its tooltip, a one-line title, Retro and End
// session as icons, the context as a tank, and the state mark last. The tank at 5%, climbing to 84% where its tone
// turns, a second Session at 72%, and the composer's Model select reading "Opus 5.5". At 1512 by 949 with the ledger
// open. Over the `session-head` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Session s-e07cfe09");
const other = region("Session s-b4a1d2c7");
const sessions = region("Sessions");
const context = inside(head, role("button", "Context"));

const sessionHead = walk(
  "session-head",
  [
    toSessions,
    { press: inside(sessions, role("button", /How does armada interface/)), say: "The Session, its ledger beside it" },
    { hover: inside(head, role("img", "s-e07cfe09", { exact: true })), say: "The address is the terminal glyph's tooltip" },
    { look: inside(head, role("button", /, rename$/)), say: "The title is one line, the whole of it on hover" },
    { look: inside(head, button("Retro")), say: "Retro" },
    { hover: inside(head, button("End session")), say: "End session" },
    { look: context, say: "The context at 5%" },
    { hover: context, say: "The breakdown card" },
    { later: context, say: "Later" },
    { look: context, say: "84%, in the warning tone" },
    { look: inside(head, role("img", "Working")), say: "The state mark stands last" },
    { look: role("combobox", "Model", { exact: true }), say: "The model is the composer's, read in full" },
    { press: button("Sessions"), say: "Back to the list" },
    { press: inside(sessions, role("button", /Cart totals rounding/)), say: "The other Session" },
    { look: inside(other, role("button", "Context")), say: "72%, with its breakdown" },
  ],
  { width: 1512, height: 949 },
);

export { sessionHead as "session-head" };
