// The walk over a `real-thread-*` mock. Apart from `real-thread.ts`, since a scenario the app loads must not pull in the
// walk engine and its `dom-accessibility-api`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

/**
 * The walk over it: open the Job, a stretch with his bubbles, a stretch of commands, the quick bar and the Aa panel.
 * `commands` are words the thread's calls line draws.
 */
export function realThreadWalk(name: string, title: string, commands: RegExp) {
  const thread = region("Thread");
  const steps: readonly Step[] = [
    toSessions,
    { press: deckRow(title), say: "A real Job's thread in the new look" },
    { look: inside(thread, region("Your message")), say: "Your own words, as a bubble on the right" },
    { look: inside(thread, text(commands)), say: "Commands, with their glyph and time" },
    { look: role("group", "Quick replies"), say: "The quick bar sits above the box, with replies to the last message" },
    { press: button("Thread look"), say: "Aa: how this thread reads" },
  ];
  return walk(name, steps);
}
