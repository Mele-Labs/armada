// Fleet starting: a live pid that has not answered yet, told apart from no Fleet
// at all. The mark in the dot's place, and the word on hover.
//
// **Every target is one the window draws at every width.** Under
// `--layout-breakpoint` the Fleet panel is a bare head with no button and no
// figures, so a target inside it, or the pid, is there only when the window is
// wide. The title row's mark and the Board's card are drawn at every width.

import { role, walk } from "../walk";

const MARK = role("img", "Starting", { exact: true });
const TITLE_ROW = role("img", "Fleet — Starting", { exact: true });

export const fleetStarting = walk("fleet/starting", [
  { look: TITLE_ROW, say: "The mark pulses where the dot sits" },
  { hover: TITLE_ROW, say: "Its name is the tooltip" },
  { look: MARK, say: "The board holds the mark and a phrase, nothing to run" },
]);
