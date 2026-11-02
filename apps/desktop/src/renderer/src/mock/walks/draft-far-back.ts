// A draft far back in a long thread, behind Show earlier, is listed in Needs you; its press opens the card beside the thread, answered there, and
// the item leaves Needs you. Over `draft-far-back`.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const needs = region("Needs you");
const panel = dialog("Draft: Description for #57");

const draftFarBack = walk("draft-far-back", [
  { press: deckRow("Tidy the settings page"), say: "A Job that drafted a description for #57, then worked on for hundreds of rows" },
  { look: inside(region("Thread"), button("Show earlier", { exact: true })), say: "The draft is far back in the thread, behind Show earlier" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you opens" },
  { look: inside(needs, button(/Open Description for #57/)), say: "The draft is listed there" },
  { press: inside(needs, button(/Open Description for #57/)), say: "A press opens its card beside the thread" },
  { look: inside(panel, role("group", "Answers")), say: "The whole draft, with Approve, Edit, Note and Withdraw under it" },
  { press: inside(panel, button("Approve", { exact: true })), say: "Approve" },
  { look: inside(panel, role("img", "Approved")), say: "The card is settled as approved, and the draft leaves Needs you" },
  { look: text("Updated #57 with the description."), say: "The agent's reply ends the thread" },
]);

export { draftFarBack as "draft-far-back" };
