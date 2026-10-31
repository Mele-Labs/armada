// The note card's tools: a note on a Job's row shows what he wants as well as saying it. Size, colour,
// fill, spacing and words are changed on the row from the design tokens, each listed with what it was, one
// undone; a pasted picture rides with the note, and Send takes the changes and the picture to a Job. The
// layer is the mock's own (`../annotating.tsx`).

import { kit } from "../sessions/walk-kit";
import { button, deckRow, dialog, inside, role, text, walk } from "../walk";
import type { Step } from "../walk";

const { thread } = kit(false);
const card = dialog("New note");
const tool = (name: string) => inside(inside(card, role("toolbar", "Tools")), button(name, { exact: true }));
const token = (group: string, name: string) => inside(inside(card, role("group", group)), button(name, { exact: true }));
const changes = inside(card, role("list", "Changes"));

const steps: Step[] = [
  { press: deckRow("Fix the flaky store test"), say: "A press on a Job's row starts a note on it" },
  { type: "This row should stand out from the others", into: inside(card, role("textbox", "Note")), say: "What he wants" },
  { press: tool("Size"), say: "Size offers the type scale" },
  { press: token("Size", "--text-lg"), say: "A step changes the row and all inside it at once" },
  { press: tool("Colour"), say: "Colour offers the text colours" },
  { press: token("Colour", "--accent"), say: "Picked" },
  { press: tool("Fill"), say: "Fill offers the grounds" },
  { press: token("Fill", "--bg-hover"), say: "Picked" },
  { press: tool("Spacing"), say: "Spacing offers padding and the gap between its children" },
  { press: token("Padding", "--space-4"), say: "Padding picked" },
  { press: tool("Text"), say: "Text edits its words" },
  { type: "Flaky store test", into: inside(card, role("textbox", "Text")), say: "Typed on the page as he types" },
  { look: changes, say: "Each change with what it was, what it is, and its token" },
  { press: inside(changes, button("Undo Colour", { exact: true })), say: "Undo takes one change off the page" },
  { paste: inside(card, role("textbox", "Note")), say: "A pasted picture shows what he means" },
  { look: inside(card, role("list", "Pictures")), say: "It sits on the note, with a press to remove it" },
  { press: inside(card, button("Save note")), say: "Saved with the changes and the picture" },
  { press: inside(dialog("Note"), button("Send to a Job")), say: "The caret lists the live Jobs" },
  { press: role("menuitem", "Fix the flaky store test"), say: "A pick sends the note to that Job" },
  { look: inside(thread, text(/Changes he made on the page/)), say: "The changes go with his words" },
];

const tools = walk("annotate-to-session", steps);

export { tools as "annotate-tools" };
