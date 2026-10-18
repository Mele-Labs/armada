// A Needs you panel holding one question opens on it, with nothing drawn above it, and Back folds the panel. Over `session-many-questions`.

import { kit } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const needs = region("Needs you");
const ask = inside(needs, role("article", "Waiting on you"));

const needsYouOneQuestion = walk("session-many-questions", [
  { press: rail("Sessions"), say: "The Jobs page" },
  { press: inside(region("Jobs", { exact: true }), button(/Plan the night/)), say: "A Job with one question waiting" },
  { look: inside(ask, text("Should a destructive best answer be decided overnight?")), say: "The question is open, with nothing stacked above it" },
  { press: inside(needs, button("Back", { exact: true })), say: "Back folds the panel" },
  { press: inside(needs, button("Needs you", { exact: true })), say: "The trigger opens the question again" },
  { look: ask, say: "Straight to it" },
]);

export { needsYouOneQuestion as "needs-you-one-question" };
