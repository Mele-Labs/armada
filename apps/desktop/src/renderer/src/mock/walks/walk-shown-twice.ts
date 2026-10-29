// A walk shown again on the same page is one call on him in Now, the newest showing. Over `walk-shown-twice`.

import { button, deckRow, inside, region, walk } from "../walk";

const present = region("Present", { exact: true });

export const walkShownTwice = walk("walk-shown-twice", [
  { press: deckRow("Rotate the audit log"), say: "Open a Job whose agent showed the same walk twice" },
  { look: inside(present, button("Audit files in System, take 2", { exact: true })), say: "Now draws the walk once, as its newest showing, where it used to draw a row for each" },
]);
