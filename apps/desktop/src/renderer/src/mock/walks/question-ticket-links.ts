// A Jira key in a Needs-you question draws as a link: a press on it opens the ticket in the side panel and does not pick the option
// it sits in. Over `question-ticket-links`, which has the key taught.

import { deckRow, dialog, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const key = inside(needs, role("link", "STORE-1234"));

const questionTicketLinks = walk("question-ticket-links", [
  { press: deckRow("Fix the store totals"), say: "The Job is open, and its question is in Needs you" },
  { look: key, say: "STORE-1234 in the question and in an option's description draws as a link" },
  { press: key, say: "A press on the key opens the ticket in the side panel" },
  { look: dialog("Issue STORE-1234"), say: "The ticket, read from its tracker, beside the question" },
  { look: inside(needs, role("radio", "Round each line", { exact: true })), say: "The option the key sits in is not picked" },
]);

export { questionTicketLinks as "question-ticket-links" };
