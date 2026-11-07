// A long thread opens on its newest rows, and the older ones are drawn as he scrolls up to them, the page staying on what he was reading.
// Over `draft-far-back`, whose Job worked on for hundreds of rows.

import { deckRow, inside, region, text, walk } from "../walk";

const thread = region("Thread");

const threadLoadsOnScroll = walk("draft-far-back", [
  {
    press: deckRow("Tidy the settings page"),
    say: "A Job that worked on for hundreds of rows",
  },
  {
    look: inside(thread, text("Section 200: Checked the group folds with the keyboard.")),
    say: "It opens on its newest rows",
  },
  {
    key: "Home",
    on: thread,
    say: "Home, to the top of what is drawn",
  },
  {
    look: inside(thread, text("Section 161: Moved the theme picker under Appearance.")),
    say: "The rows before it were drawn as the top came near, and the view stayed where it was",
  },
  {
    key: "Home",
    on: thread,
    say: "Home again",
  },
  {
    look: inside(thread, text("Section 141: Moved the theme picker under Appearance.")),
    say: "The next rows before those came in the same way",
  },
]);

export { threadLoadsOnScroll as "thread-loads-on-scroll" };
