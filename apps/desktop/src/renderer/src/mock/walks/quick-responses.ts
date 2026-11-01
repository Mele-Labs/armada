// The quick responses on a line of the agent's message: 👍 sends, a right press edits one, ❓ fills the box. Over the
// `real-thread-header-sizes` scenario.

import { button, deckRow, dialog, inside, region, role, text, walk as walkOver } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const barOf = (line: string) => inside(thread, role("group", new RegExp(`^Quick responses: ${line}`)));
const editor = dialog("Edit No to this");

const walk = walkOver("real-thread-header-sizes", [
  toSessions,
  { press: deckRow("Header Font Sizes"), say: "A Job's thread, with the agent's options in a list" },
  { hover: inside(thread, text(/C: 18/)), say: "Hover an option and the quick responses float over its right end" },
  { press: inside(barOf("C: 18"), button("Yes to this")), say: "👍 sends the line back as a quote with the emoji, and the line keeps a 👍" },
  { context: inside(barOf("B: 15"), button("No to this")), say: "A right press on a response offers Edit, Add response and Remove" },
  { press: role("menuitem", "Edit", { exact: true }), say: "Edit opens the grid, the emoji field and the name" },
  { press: inside(editor, button("Fire", { exact: true })), say: "Any common reaction is one press, and the field takes any emoji typed or pasted" },
  { press: inside(editor, button("Save", { exact: true })), say: "Save puts it in the bar" },
  { hover: inside(thread, text(/B: 15/)), say: "The bar shows 🔥 where 👎 was" },
  { hover: inside(thread, text(/A: 14/)), say: "Hover the first option" },
  { press: inside(barOf("A: 14"), button("Ask about this")), say: "❓ sends nothing: the quote waits in the message box, ready to type or talk" },
]);

export { walk as "quick-responses" };
