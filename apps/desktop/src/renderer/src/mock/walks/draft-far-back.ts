// A draft far back in a long thread, behind Show earlier, is listed in Needs you; the sheet the band opens draws its card, answered there, and the item
// leaves the list. Over `draft-far-back`.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import { openNeeds } from "../sessions/walk-kit";

const sheet = dialog("Tidy the settings page");
const list = inside(sheet, role("group", "Waiting on you"));

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
  openNeeds,
  {
    look: inside(list, button(/Description for #57/)),
    say: "The draft is listed there",
  },
  {
    press: inside(list, button(/Description for #57/)),
    say: "A press draws its card in the sheet",
  },
  {
    look: inside(sheet, role("group", "Answers")),
    say: "The whole draft, with Approve, Edit, Note and Withdraw under it",
  },
  {
    press: inside(sheet, button(/^Approve/)),
    say: "Approve",
  },
  {
    look: text("Updated #57 with the description."),
    say: "The agent's reply ends the thread",
  },
]);

export { draftFarBack as "draft-far-back" };
