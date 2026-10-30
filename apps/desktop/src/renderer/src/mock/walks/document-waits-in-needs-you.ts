// A document the Job wrote asks for his Approve as a sketch does: its row needs him, the page is a card in the thread with Approve
// and Send back, it stands in Now until approved, and the page read over the Job carries the same keys. Approve there takes the
// keys away and leaves the page open, as a sketch's panel does, and the script stands under Done. Over
// `document-waits-in-needs-you`; `document-asks-again` holds revision 2 asking after revision 1 was approved.

import { button, deckRow, inside, region, role, walk } from "../walk";

const card = role("article", "Document: Video script, Revision 1", { exact: true });
const now = region("Now", { exact: true });
const layer = region("Document", { exact: true });
const done = region("Done", { exact: true });

const documentWaitsInNeedsYou = walk("document-waits-in-needs-you", [
  { press: deckRow("Demo marketing video"), say: "The script waits on you, so the Job's row needs you" },
  { look: inside(now, button("Video script", { exact: true })), say: "It stands in Now until you approve it" },
  { press: inside(card, button("Open Video script, Revision 1", { exact: true })), say: "Its card in the thread names the revision, with Approve and Send back. Open reads the page over the Job" },
  { press: inside(layer, button("Approve", { exact: true })), say: "The page carries the same Approve and Send back" },
  { key: "Escape", on: layer, say: "Esc gives the Job back" },
  { press: inside(done, button(/^Done/)), say: "The script has left Now and stands under Done" },
]);

export { documentWaitsInNeedsYou as "document-waits-in-needs-you" };
