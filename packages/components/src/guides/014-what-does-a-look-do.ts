import type { Guide } from "./guide";

/**
 * What the `Refresh` beside it does, and what a reading's age means. Pulse's
 * headline used to carry *Looking costs no model call* and its footer *a
 * process can exit between the reading and this screen*. The button read
 * `Look now` until 28 Sep, when the owner renamed it; the act is still a look
 * and the word is still this guide's to explain.
 *
 * **No `docs/concepts/` page holds this.** The looks are `crates/ipc`'s
 * `Look`, and nothing under `docs/concepts/` writes them up; saying so here is
 * better than naming a page that does not carry it.
 */
export const GUIDE_LOOK: Guide = {
  number: 14,
  group: "machine",
  title: "What does a look do?",
  piece: "pulse.look",
  steps: [
    "A look asks Fleet to go and check this job on this machine.",
    "It reads the process, the checkout, what is being written and where the job has got to.",
    "It also says whether anything has gone quiet or is repeating itself.",
    "It costs no model call.",
    "Nothing about the job moves and nothing is spent, so there is no reason not to press it.",
    "Every figure here is one instant.",
    "A process can exit between the reading and this screen, which is why the age is drawn under " +
      "the tables.",
    "A look answers working, not working, or could not tell.",
    "Could not tell is its own answer. It means the checks came back short.",
  ],
};
