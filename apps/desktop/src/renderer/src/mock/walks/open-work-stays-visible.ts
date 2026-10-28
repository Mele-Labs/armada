// Work still open stays in view. A pull request that is a draft or open, and a follow-up not yet settled, stand in the Job's current step and in an Open group after the
// last step, and a finished step holds only what finished. A right press on the Open group hides it. Over `open-work-stays-visible`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const propose = region("Propose", { exact: true });
const approvePlan = region("Approve plan", { exact: true });
const current = region("Walk", { exact: true });
const open = region("Open", { exact: true });

export const openWorkStaysVisible = walk("open-work-stays-visible", [
  { press: deckRow("Issues tab beside Reviews"), say: "A Job at its fourth step, with a draft pull request and an open one made in earlier steps" },
  { press: inside(approvePlan, button(/Approve plan/)), say: "A finished step holds only what finished: the merged pull request stays here" },
  { look: propose, say: "Nothing still open is filed under it" },
  { look: inside(current, role("list", "Open")), say: "The open pull requests stand in the current step, with their state" },
  { look: open, say: "The Open group after the last step lists them again with the follow-up, so a folded step never hides them" },
  { look: inside(open, text(/Tracker mods over a CLI/)), say: "It is open to start with, and folds like any step" },
  { context: open, say: "A right press on the group offers Hide Open group, for every Job. Settings, Bridge has it back" },
]);
