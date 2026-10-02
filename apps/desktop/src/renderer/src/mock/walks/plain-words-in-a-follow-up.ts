// An issue drafted from a finding starts with the finding's words for its
// title, with none of the markdown it was written in. The finding's own cell
// draws its markdown; the draft's title field holds plain words.

import { button, dialog, inside, role, row, walk } from "../walk";

export const plainWordsInAFollowUp = walk("job/review", [
  { press: button("For context"), say: "A finding written with bold and code" },
  { press: inside(row("settings_store"), button("More ways to follow up")), say: "Ways to follow it up" },
  { press: role("menuitem", "File an issue"), say: "File an issue from it" },
  {
    look: inside(dialog("File an issue"), role("textbox", "Title")),
    say: "The draft's title: the finding's words, with old plain rather than in asterisks",
  },
]);
