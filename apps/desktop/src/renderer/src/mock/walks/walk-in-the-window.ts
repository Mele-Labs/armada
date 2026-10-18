// A walk played in Bridge's window: the page is a walk, so its controls are on the window's bar
// under the takes, and the keys act on it from there. Over the `walk-in-the-window` scenario.
// The page's own card steps aside; its ring stays in the page.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const window = dialog("Bridge's window on Walks in a window");
const takes = inside(window, role("radiogroup", "Variations"));
const row = inside(window, role("group", /^Walk /));
const next = inside(row, button("Next", { exact: true }));
const step = (n: number) => inside(row, text(new RegExp(`^Step ${n} of \\d+`)));

const inTheWindow = walk("walk-in-the-window", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Walks in a window")), say: "A Job with three walks to show" },
  { type: "Show me the walks\n", into: role("textbox", "Message"), say: "Asked to show them" },
  { look: takes, say: "One window, three takes on its bar" },
  { look: row, say: "The walk's controls are a row of the bar's own, under the takes; the page has no card" },
  { look: step(1), say: "Step 1, and the caption beside it" },
  { press: next, say: "Next acts on the page, as the card's did" },
  { look: step(2), say: "Step 2" },
  { key: "ArrowRight", on: next, say: "→ is Next" },
  { look: step(3), say: "Step 3" },
  { press: inside(row, button("Back", { exact: true })), say: "Back replays the page to the step before" },
  { look: step(2), say: "Step 2 again" },
  { key: "ArrowLeft", on: next, say: "← is Back" },
  { look: step(1), say: "Step 1; Back is off here" },
  { key: "ArrowRight", on: next, say: "Forward again" },
  { key: "r", on: next, say: "R replays from step 1" },
  { look: step(1), say: "Step 1" },
  { key: "a", on: next, say: "A turns Autoplay on" },
  { look: inside(row, role("switch", "Autoplay")), say: "On, and the page goes on by itself" },
  { key: "a", on: next, say: "And off" },
  { press: inside(takes, role("radio", "Task marks")), say: "Another take is another walk, with its own controls" },
  { look: step(1), say: "Step 1 of that one" },
]);

export { inTheWindow as "walk-in-the-window" };
