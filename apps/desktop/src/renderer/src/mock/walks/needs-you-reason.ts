// Why a Job needs him, drawn where he already looks. Three Jobs wait on him, for a command, a question and a walk, and one runs. `&take=`
// is the take: `row` a second line under the Job's name, `pillar` a label over its amber pillar on the map, `both` the two (the default).
// The drone's card says the reason in every take. Over `needs-you-reason`.

import { deckRow, inside, region, role, walk } from "../walk";

const map = role("listbox", "Map");

const needsYouReason = walk("needs-you-reason", [
  { look: region("Jobs", { exact: true }), say: "Three Jobs need him and pulse. In the row and both takes, a quieter line under each name says what for. The running Job has none" },
  { hover: deckRow("Migrate the payments store"), say: "A long reason is cut with an ellipsis. Its full words are in the tooltip on the line" },
  { look: map, say: "The map. In the pillar and both takes, each amber pillar carries a label with the same words. In the row take it stays bare" },
  { hover: inside(map, role("option", "Rebuild the dock panel")), say: "The drone's card. Its call line says the reason in all three takes" },
]);

export { needsYouReason as "needs-you-reason" };
