// A document the Job wrote asks for his Approve as a sketch does: its row needs him, the page is a card in the thread with Approve
// and Send back, it stands in Now until approved, the page read over the Job carries the same keys, and Approve moves it under
// Done. Over `document-waits-in-needs-you`; `document-asks-again` holds revision 2 asking after revision 1 was approved.

import { button, deckRow, inside, region, role, walk } from "../walk";

const card = role("article", "Document: Video script, Revision 1", { exact: true });
const now = region("Now", { exact: true });
const layer = region("Document", { exact: true });

const documentWaitsInNeedsYou = walk("document-waits-in-needs-you", [
  { look: deckRow("Demo marketing video"), say: "The script waits on you, so the row needs you" },
  { press: deckRow("Demo marketing video"), say: "The Job" },
  { look: inside(now, button("Video script", { exact: true })), say: "It stands in Now until you approve it" },
  { press: inside(card, button("Open Video script, Revision 1", { exact: true })), say: "Its card in the thread names the revision, with Approve and Send back. Open reads the page over the Job" },
  { key: "Escape", on: inside(layer, button("Approve", { exact: true })), say: "The page carries the same Approve and Send back. Esc gives the Job back" },
  { press: inside(card, button("Approve", { exact: true })), say: "Approve: the script leaves Now and stands under Done" },
]);

export { documentWaitsInNeedsYou as "document-waits-in-needs-you" };
