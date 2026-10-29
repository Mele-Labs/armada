// A follow-up the agent kept draws Remind and two thumbs after its words: thumbs down takes it off, thumbs up keeps it without the thumbs, and a right press
// offers the same two. A filed issue has none. Over the `deferred-thumbs` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const landing = region("Open", { exact: true });
const made = region("Done", { exact: true });
const rowOf = (words: RegExp) => inside(landing, role("listitem", words));

const deferredThumbs = walk("deferred-thumbs", [
  { press: deckRow("Read the Studio registry on start"), say: "A Job with follow-ups kept" },
  { press: inside(landing, button("Deferred")), say: "Three kept follow-ups and a filed issue" },
  { hover: inside(landing, button(/^Deferred: The Studio icon/)), say: "Deferred" },
  { press: inside(landing, button(/^Not deferred: The Studio icon/)), say: "Thumbs down takes it off the list" },
  { context: rowOf(/Rename the registry loader/), say: "A right press on another one opens its menu, with the same two acts" },
  { press: role("menuitem", "Deferred", { exact: true }), say: "Deferred keeps it" },
  { key: "Alt+1", on: made, say: "Done is folded until pressed: ⌥1 opens it" },
  { look: inside(made, button(/Rename the registry loader/)), say: "It moves to Done as a deferred item, with no Remind and no thumbs" },
]);

export { deferredThumbs as "deferred-thumbs" };
