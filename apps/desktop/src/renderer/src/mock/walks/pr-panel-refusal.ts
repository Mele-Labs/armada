// A press Fleet refuses is said in the pull request's own panel, where it was pressed: Enable auto-merge on #1847, whose checks
// are still running, comes back "Auto merge is not allowed". Over the `pr-panel-refusal` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, text, walk } from "../walk";

const { sheet, close, row } = kit(false);
const card = sheet("Pull request #1847");

const refusal = walk("pr-panel-refusal", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Fix the flaky store test")), say: "A Job holding a pull request whose checks are running" },
  ...row(/^#1847 /, "The pull request opens in the side panel"),
  { look: inside(card, button("Enable auto-merge")), say: "Auto-merge is offered" },
  { press: inside(card, button("Enable auto-merge")), say: "Enable auto-merge, pressed" },
  { look: inside(card, text("Auto merge is not allowed")), say: "Fleet's refusal, in the panel it was pressed in" },
  { press: close(card), say: "Back to the Job" },
]);

export { refusal as "pr-panel-refusal" };
