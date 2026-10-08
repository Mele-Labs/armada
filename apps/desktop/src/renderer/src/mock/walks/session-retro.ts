// A Session's retro (`docs/concepts/retro.md`), written from the Sessions list. Each row has the
// Retro press, an icon with a tooltip. Pressing it writes the retro of what happened in that
// Session, the mark moving meanwhile, and opens it on the Retros page, headed by the Session's
// title and address. Its items read like a Job's, and agreeing with one proposes a Job.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const ROW = inside(region("Sessions"), role("listitem", "Fix the flaky store test"));
const RETRO = dialog("Retro");
const item = (words: string) => inside(RETRO, role("listitem", words));
const RESET = item("A reset waited 40 minutes for an answer");

const retro = walk("session-retro", [
  { hover: inside(ROW, button("Retro")), say: "Retro, an icon on each row" },
  { press: inside(ROW, button("Retro")), say: "It writes the retro of what happened in this Session" },
  { look: inside(ROW, button("Writing the retro")), say: "The mark moves while it is written" },
  { look: RETRO, say: "Then the retro opens on the Retros page" },
  { look: inside(RETRO, text(/Fix the flaky store test · s-01IDLECC/)), say: "Headed by the Session's title and address" },
  { look: inside(RESET, text(/^Agent$/)), say: "An agent's way, where a Job's says Drone" },
  { look: inside(RESET, text(/^Armada$/)), say: "And where the fix lands, as on a Job's" },
  { look: item("Walks opened in your browser"), say: "Five items, from today's pains" },
  { press: inside(RESET, button("Evidence")), say: "The record rows it cites" },
  { look: inside(RESET, text("Allow git reset --hard origin/main?")), say: "The ask, and how long it waited" },
  { press: inside(RESET, button("Create Job", { exact: true })), say: "Agree proposes a Job, as on a Job's retro" },
  { look: inside(RESET, button("Proposed Job")), say: "The item keeps a link to it" },
]);

export { retro as "session-retro" };
