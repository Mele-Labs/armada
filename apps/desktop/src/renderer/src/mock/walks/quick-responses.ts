// The quick responses on the bar under the agent's message: 👍 sends, a right press edits one, ❓ fills the box. Over the
// `real-thread-header-sizes` scenario.

import { button, deckRow, dialog, inside, region, role, text, walk as walkOver } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
// The last message's bar: the engine takes the last of several, and it is the one the hover raises.
const bar = inside(thread, role("toolbar", "Message"));
const asked = inside(thread, text(/Do you want the plan recorded/));
const editor = dialog("Edit No to this");

const walk = walkOver("real-thread-header-sizes", [
  toSessions,
  {
    press: deckRow("Header Font Sizes"),
    say: "A Job's thread, its last message asking a question",
  },
  {
    hover: asked,
    say: "Hover the message and a bar comes up under it: Copy, Reply, the quick responses and the time",
  },
  {
    press: inside(bar, button("Yes to this")),
    say: "👍 sends the message back as a quote with the emoji, and the bar keeps the 👍 pressed",
  },
  {
    hover: asked,
    say: "Back on the message",
  },
  {
    context: inside(bar, button("No to this")),
    say: "A right press on a response offers Edit, Add response and Remove",
  },
  {
    press: role("menuitem", "Edit", {
      exact: true,
    }),
    say: "Edit opens the grid, the emoji field and the name",
  },
  {
    press: inside(
      editor,
      button("Fire", {
        exact: true,
      }),
    ),
    say: "Any common reaction is one press, and the field takes any emoji typed or pasted",
  },
  {
    press: inside(
      editor,
      button("Save", {
        exact: true,
      }),
    ),
    say: "Save puts it in the bar",
  },
  {
    hover: asked,
    say: "The bar shows 🔥 where 👎 was",
  },
  {
    press: inside(bar, button("Ask about this")),
    say: "❓ sends nothing: the quote waits in the message box, ready to type or talk",
  },
]);

export { walk as "quick-responses" };
