// The Bays filter's pull requests outside a bay. Each sits under the Job or Session it belongs to, with the
// owner's live state as its mark; its dot is its checks, and its stops are the stages it has reached. One
// nobody in Armada owns stays a bare branch. Over `bays-outside-prs`.

import { button, inside, role, tab, text, walk } from "../walk";

const outside = role("list", "Pull requests outside a bay");
const tokens = button("Session: Theme token audit");
const migrate = button("Job: Order the store migrations");

const baysOutsidePrs = walk("bays-outside-prs", [
  { press: tab("Bays"), say: "Bays" },
  { look: outside, say: "Pull requests no bay holds, under the work each belongs to" },
  { look: tokens, say: "A Session's two pull requests sit together under it, one found by its number and one by its branch" },
  { hover: inside(outside, role("img", "Idle")), say: "Its mark is the Session's own state, as its tile draws it" },
  { look: migrate, say: "A Job that opened one: the wire names it" },
  { hover: inside(outside, role("img", "Drone working")), say: "A Job working shows its Drone" },
  { look: button("Job: Fold the two notification routes into one"), say: "A Job that is done still owns the pull request on its branch" },
  { look: button("Session: Release script"), say: "A Session that has ended keeps its pull request" },
  { hover: inside(outside, role("img", "Ended")), say: "Ended" },
  { hover: inside(outside, role("img", "ci running")), say: "The dot is the pull request's checks, named on hover" },
  { hover: text("chore/rename-status-tokens"), say: "The number and the forge's title on the branch" },
  { hover: button(/^chore\/rename-status-tokens: Pull request/), say: "A stop is a stage it has reached, with its card" },
  { hover: button(/^armada\/91-notification-routes: Merge queue/), say: "In the merge queue, with its place inside the stop" },
  { hover: button("Pull request: chore/bump-the-lockfile"), say: "One nobody in Armada owns is only its branch" },
  { hover: inside(tokens, text("Theme token audit")), say: "The owner opens on a press" },
  { press: tokens, say: "Pressing the owner opens that Session" },
]);

export { baysOutsidePrs };
