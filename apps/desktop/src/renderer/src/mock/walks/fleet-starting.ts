// Fleet starting: a live pid that has not answered yet, told apart from no Fleet
// at all. The mark in the dot's place, and the word on hover; the Cockpit powers on beneath (`cockpit/boot.ts`).
//
// The dot on the title row is the Fleet button: the mark takes the dot's place there.

import { button, region, walk } from "../walk";

const TITLE_ROW = button("Fleet — Starting", { exact: true });

export const fleetStarting = walk("fleet/starting", [
  { look: TITLE_ROW, say: "The mark pulses where the dot sits" },
  { hover: TITLE_ROW, say: "Its name is the tooltip" },
  { look: region("Jobs", { exact: true }), say: "The Cockpit stands by: each panel drawn, waiting on Fleet, with no banner and nothing to run" },
]);
