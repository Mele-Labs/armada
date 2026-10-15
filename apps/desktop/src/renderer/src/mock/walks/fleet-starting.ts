// Fleet starting: a live pid that has not answered yet, told apart from no Fleet
// at all. The mark in the dot's place, and the word on hover.
//
// The dot on the title row is the Fleet button: the mark takes the dot's place there, and the
// Board's card holds the same mark.

import { button, role, walk } from "../walk";

const MARK = role("img", "Starting", { exact: true });
const TITLE_ROW = button("Fleet — Starting", { exact: true });

export const fleetStarting = walk("fleet/starting", [
  { look: TITLE_ROW, say: "The mark pulses where the dot sits" },
  { hover: TITLE_ROW, say: "Its name is the tooltip" },
  { look: MARK, say: "The board holds the mark and a phrase, nothing to run" },
]);
