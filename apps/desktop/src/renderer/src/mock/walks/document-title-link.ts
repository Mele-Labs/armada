// A page with the same title as its Job. The agent's message names the page, and its title is a link to the page, not to the Job. While the page
// is not approved no plan card stands in the thread, and his Approve of the page settles the plan. Over `document-title-link`.

import { ROLLOUT_DOC_TITLE } from "../document-pages";
import { button, deckRow, inside, region, role, walk } from "../walk";

const thread = region("Thread");
const layer = region("Document", { exact: true });
const present = region("Present", { exact: true });
const title = inside(thread, button(ROLLOUT_DOC_TITLE, { exact: true }));
const window = role("dialog", `Bridge's window on ${ROLLOUT_DOC_TITLE}`);

export const documentTitleLink = walk("document-title-link", [
  { press: deckRow(ROLLOUT_DOC_TITLE), say: "A design plan Job that wrote a page with its own title and recorded a plan" },
  { look: title, say: "The title in the agent's message is a link" },
  { press: title, say: "A press opens the page, not the Job" },
  { look: layer, say: "The Document layer opens on the page" },
  { key: "Escape", on: layer, say: "Esc" },
  { look: present, say: "No Plan to approve stands in the thread: the page is the one approval asked" },
  { press: inside(present, button(ROLLOUT_DOC_TITLE, { exact: true })), say: "The page's own approval opens its window" },
  { press: inside(window, button("Approve")), say: "Approve the page" },
  { look: inside(thread, button(/Plan approved|Approved/)), say: "The plan reads approved, with no card at any time" },
]);
