// A page he approved does not ask again on the Job that took over from the one that wrote it. Over `document-handed-on`; `document-asks-again`
// holds what a real edit does.

import { button, deckRow, inside, region, role, walk } from "../walk";

const card = role("article", "Document: Video script, Revision 2", { exact: true });
const layer = region("Document", { exact: true });
const done = region("Done", { exact: true });

const documentHandedOn = walk("document-handed-on", [
  { press: deckRow("Demo marketing video"), say: "This Job took another's place and only read the page, so its row does not need you" },
  { look: card, say: "The page came with it at the revision you approved, first in the thread, with no Approve or Send back" },
  { press: inside(card, button("Open Video script, Revision 2", { exact: true })), say: "Open reads the page over the Job" },
  { key: "Escape", on: layer, say: "Esc gives the Job back" },
  { press: inside(done, button(/^Done/)), say: "The page stands under Done" },
]);

export { documentHandedOn as "document-handed-on" };
