// Fleet starting: a live pid that has not answered yet, told apart from no Fleet
// at all. A glyph in the dot's place, and the word on hover.

import { role, text, walk } from "../walk";

const MARK = role("img", "Starting", { exact: true });

export const fleetStarting = walk("fleet/starting", [
  { look: MARK, say: "Fleet's head: the glyph breathes where the dot sits" },
  { hover: MARK, say: "Its name is the tooltip" },
  { look: text("61372"), say: "The pid the runtime file names" },
]);
