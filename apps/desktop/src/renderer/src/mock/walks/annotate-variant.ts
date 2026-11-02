// The note card's Variant tool: a note on a setting field's input names the component and its file, and Variant
// lists the stories of the nearest component in the chain that has any. A story is picked and recorded with the file
// it is in, pressed again it is undone, and Send takes the line to a Job. Disabled sets no args on an Input, so that pick is
// recorded and the input draws as before; Secondary on Reset's Button changes its ground, so the swap is seen once. The stories
// are read by the mock's dev server (`annotations-stories.ts`); the layer is the mock's own (`../annotating.tsx`).

import { kit } from "../sessions/walk-kit";
import { button, dialog, inside, role, text, viaPalette, walk } from "../walk";
import type { Step } from "../walk";

const { thread } = kit(false);
const card = dialog("New note");
const variants = inside(card, role("group", "Variant"));
const changes = inside(card, role("list", "Changes"));
const reset = role("button", /^Reset: Drones at once/);

const steps: Step[] = [
  ...viaPalette("Limits", "Limits draws each setting as a field"),
  { press: role("textbox", "Drones at once"), say: "A press on a field's input starts a note on it" },
  { look: inside(card, text("Input · Input.tsx")), say: "Under the chain, each component with stories and its file, the whole path on hover" },
  { type: "What does the disabled input look like?", into: inside(card, role("textbox", "Note")), say: "What he wants" },
  { press: inside(inside(card, role("toolbar", "Tools")), button("Variant", { exact: true })), say: "Variant lists the stories of the nearest component that has any" },
  { look: inside(variants, button("Disabled", { exact: true })), say: "Each story is drawn as it is, over its name" },
  { press: inside(variants, button("Disabled", { exact: true })), say: "A story is picked on the page" },
  { look: role("textbox", "Drones at once"), say: "Disabled sets no args, so the input draws as before" },
  { look: changes, say: "The pick is listed with what it was" },
  { press: inside(variants, button("Disabled", { exact: true })), say: "Pressed again, it is undone" },
  { press: inside(variants, button("Disabled", { exact: true })), say: "Picked once more" },
  { press: inside(card, button("Save note")), say: "Saved with the variant" },
  { press: inside(dialog("Note"), button("Send to a Job")), say: "The caret lists the live Jobs" },
  { press: role("menuitem", "Fix the flaky store test"), say: "A pick sends the note to that Job" },
  { look: inside(thread, text(/variant: as drawn → Disabled \(story Disabled in packages\/components\/src\/primitives\/Input\/Input\.stories\.tsx\)/)), say: "The variant line goes with his words" },
  { look: inside(thread, text(/Component: Input in packages\/components\/src\/primitives\/Input\/Input\.tsx/)), say: "And the file the component is in" },
  ...viaPalette("Limits", "Back to Limits, for a Button"),
  { press: reset, say: "A note on Reset, a Button" },
  { look: inside(card, text("Button · Button.tsx")), say: "Its component and file" },
  { type: "Should Reset be a secondary button?", into: inside(card, role("textbox", "Note")), say: "What he wants" },
  { press: inside(inside(card, role("toolbar", "Tools")), button("Variant", { exact: true })), say: "Variant lists Button's stories" },
  { look: inside(variants, button("Secondary", { exact: true })), say: "Secondary, drawn before it is picked" },
  { press: inside(variants, button("Secondary", { exact: true })), say: "Secondary is picked on the page" },
  { look: reset, say: "The ground changes on the page" },
  { press: inside(variants, button("Secondary", { exact: true })), say: "Pressed again, it is undone" },
  { press: inside(variants, button("Secondary", { exact: true })), say: "Picked once more" },
  { press: inside(card, button("Save note")), say: "Saved with the variant" },
  { press: inside(dialog("Note"), button("Send to a Job")), say: "The caret lists the live Jobs" },
  { press: role("menuitem", "Fix the flaky store test"), say: "Sent to the same Job" },
  { look: inside(thread, text(/variant: ghost → Secondary \(story Secondary in packages\/components\/src\/primitives\/Button\/Button\.stories\.tsx\)/)), say: "Both variant lines are in the thread" },
  { look: inside(thread, text(/variant: as drawn → Disabled/)), say: "The Input's, from before" },
];

const variant = walk("annotate-to-session", steps);

export { variant as "annotate-variant" };
