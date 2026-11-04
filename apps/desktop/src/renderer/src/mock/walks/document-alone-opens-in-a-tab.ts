// A page that is all the Job waits on opens as a tab of the Job, not over it: Open on its thread card brings the page forward as
// a Document tab, with Approve and Send back under it. Over `document-waits-in-needs-you`.

import { button, deckRow, inside, role, tab, walk } from "../walk";

const card = role("article", "Document: Video script, Revision 1", {
  exact: true,
});
const page = role("tabpanel", /Video script/);

const documentAloneOpensInATab = walk("document-waits-in-needs-you", [
  {
    press: deckRow("Demo marketing video"),
    say: "The script is all the Job waits on, and it is in Needs you while you are in the Job",
  },
  {
    press: inside(
      card,
      button("Open Video script, Revision 1", {
        exact: true,
      }),
    ),
    say: "Open reads the page in a tab of the Job",
  },
  {
    look: tab(/Video script/),
    say: "The page is its own tab beside the thread",
  },
  {
    press: inside(
      page,
      button("Approve", {
        exact: true,
      }),
    ),
    say: "Approve and Send back are under the page, as on its card",
  },
]);

export { documentAloneOpensInATab as "document-alone-opens-in-a-tab" };
