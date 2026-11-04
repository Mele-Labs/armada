// Auto land answers at once. The Job holds five pull requests; the press lights the control with no error, and the forge's refusal of
// #1843 arrives a moment later as a Fleet notice in the thread. Over the `auto-land-answers-at-once` scenario.

import { REFUSED, TITLE } from "../scenarios/auto-land-answers-at-once";
import { toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, region, role, text, walk } from "../walk";

const acts = inside(
  region("Pane: Thread", {
    exact: true,
  }),
  role("toolbar", "Job acts"),
);

const autoLandAnswersAtOnce = walk("auto-land-answers-at-once", [
  toSessions,
  {
    press: deckRow(TITLE),
    say: "A Job holding five open pull requests",
  },
  {
    look: inside(acts, button("Auto land")),
    say: "Auto land is off",
  },
  {
    press: inside(acts, button("Auto land")),
    say: "Auto land, pressed",
  },
  {
    look: inside(acts, button("Auto land")),
    say: "Lit at once, with no error",
  },
  {
    look: text(REFUSED),
    say: "The forge's refusal of #1843 arrives in the thread as a Fleet notice",
  },
]);

export { autoLandAnswersAtOnce as "auto-land-answers-at-once" };
