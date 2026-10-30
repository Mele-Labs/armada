// A page with the same title as its Job. The agent's message names the page, and its title is a link to the page, not to the Job. While the page
// is not approved no plan card stands in the thread, and his Approve of the page settles the plan. Over `document-title-link`.

import { ROLLOUT_DOC_TITLE } from "../document-pages";
import { button, deckRow, inside, region, role, text, walk } from "../walk";

const thread = region("Thread");
const layer = region("Document", { exact: true });
const needs = region("Needs you");
const page = inside(needs, button(new RegExp(ROLLOUT_DOC_TITLE)));
const title = inside(thread, role("link", ROLLOUT_DOC_TITLE, { exact: true }));

const linking = walk("document-title-link", [
  { press: deckRow(ROLLOUT_DOC_TITLE), say: "A design plan Job that wrote a page with its own title and recorded a plan" },
  { look: title, say: "The title in the agent's message is a link" },
  { press: title, say: "A press opens the page, not the Job" },
  { look: layer, say: "The Document layer opens on the page" },
  { key: "Escape", on: layer, say: "Esc" },
  { press: inside(needs, button("Needs you")), say: "Needs you opens: the page waits here, and no plan card stands in the thread" },
  { look: page, say: "The page's approval is the one thing asked" },
  { press: page, say: "A press on it approves the page" },
  { look: text("Roll the retry budget out in four phases"), say: "The plan stands in the Blueprint, settled with the page, and no card was asked on the way" },
]);

export { linking as "document-title-link" };
