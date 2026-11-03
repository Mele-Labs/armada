// A draft far back in a long thread, behind Show earlier, is listed in Needs you; its press opens the card as a tab of its own beside Thread, answered
// there, and the item leaves Needs you. Over `draft-far-back`.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";

const needs = region("Needs you");
const card = role("tabpanel", "Description for #57");

const draftFarBack = walk("draft-far-back", [
  {
    press: deckRow("Tidy the settings page"),
    say: "A Job that drafted a description for #57, then worked on for hundreds of rows",
  },
  {
    look: inside(
      region("Thread"),
      button("Show earlier", {
        exact: true,
      }),
    ),
    say: "The draft is far back in the thread, behind Show earlier",
  },
  {
    press: inside(needs, button(/^Needs you/)),
    say: "Needs you opens",
  },
  {
    look: inside(needs, button(/Open Description for #57/)),
    say: "The draft is listed there",
  },
  {
    press: inside(needs, button(/Open Description for #57/)),
    say: "A press opens its card as a tab on the Job",
  },
  {
    look: tab("Description for #57", {
      exact: true,
    }),
    say: "The tab is named for the draft, beside Thread",
  },
  {
    look: inside(card, role("group", "Answers")),
    say: "The tab is in front: the whole draft, with Approve, Edit, Note and Withdraw under it",
  },
  {
    press: inside(
      card,
      button("Approve", {
        exact: true,
      }),
    ),
    say: "Approve",
  },
  {
    look: inside(card, role("img", "Approved")),
    say: "The card is settled as approved in its tab, and the draft leaves Needs you",
  },
  {
    press: tab("Thread", {
      exact: true,
    }),
    say: "Back to Thread; the draft's tab stays until it is closed",
  },
  {
    look: text("Updated #57 with the description."),
    say: "The agent's reply ends the thread",
  },
]);

export { draftFarBack as "draft-far-back" };
