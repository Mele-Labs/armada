// Fleet starting: a live pid that has not answered yet, told apart from no Fleet
// at all. A glyph in the dot's place, and the word on hover.

import { inside, role, text, walk } from "../walk";

const MARK = role("img", "Starting", { exact: true });
const HEAD = inside(role("button", /^Fleet/), MARK);

export const fleetStarting = walk("fleet/starting", [
  { look: HEAD, say: "Fleet's head: the glyph breathes where the dot sits" },
  { hover: HEAD, say: "Its name is the tooltip" },
  { look: text("61372"), say: "The pid the runtime file names" },
  { look: MARK, say: "The board holds the glyph and nothing to run" },
]);
