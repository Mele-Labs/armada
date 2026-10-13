// The Bays filter's pull requests outside a bay. Each sits under the Job or Session it belongs to, with the
// owner's live state as its mark, and its line carries the journey that work made: the stations it passed
// (waited, held a bay, built), then its pull request and the queue. One nobody in Armada owns stays a bare
// branch. Over `bays-outside-prs`.

import { button, inside, role, tab, text, walk } from "../walk";

const outside = role("list", "Pull requests outside a bay");
const tokens = button("Session: Theme token audit");
const notes = button("Session: Migration notes");
const pause = button("Job: Add the pause marker column");

const baysOutsidePrs = walk("bays-outside-prs", [
  { press: tab("Bays"), say: "Bays" },
  { look: outside, say: "Pull requests no bay holds, under the work each belongs to" },
  { look: tokens, say: "A Session's pull requests sit together under it, each with the line that work made" },
  { hover: inside(outside, role("img", "Idle")), say: "Its mark is the Session's own state, as its tile draws it" },
  { hover: button(/^chore\/rename-status-tokens: Waiting/), say: "Waiting for a bay: it did, before it had one" },
  { hover: button(/^chore\/rename-status-tokens: In a bay/), say: "The bay it was in, and for how long, though it gave it back" },
  { hover: button(/^chore\/rename-status-tokens: Building/), say: "Building, on its branch" },
  { hover: button(/^chore\/drop-unused-tokens: Pull request/), say: "Its pull request, and when it was opened" },
  { hover: inside(outside, role("img", "Working")), say: "A Session still working has already given its bay back" },
  { hover: button(/^docs\/migration-order: In a bay/), say: "Bay 10, from and to" },
  { look: pause, say: "A Job that finished and waits on him" },
  { hover: inside(pause, role("img", "Waiting on you")), say: "Its mark says so" },
  { hover: button(/^armada\/93-pause-column: In a bay/), say: "A Job's record names no bay, so it says a bay" },
  { look: button("Job: Fold the two notification routes into one"), say: "A Job that is done still owns the pull request on its branch" },
  { look: button("Session: Release script"), say: "A Session that has ended keeps its pull request" },
  { hover: inside(outside, role("img", "Ended")), say: "Ended" },
  { hover: button(/^release\/script-bump: In a bay/), say: "And the bay it held while it lived" },
  { hover: inside(outside, role("img", "ci failed")), say: "The dot is the pull request's checks, named on hover" },
  { hover: text("chore/rename-status-tokens"), say: "The number and the forge's title on the branch" },
  { hover: button(/^armada\/91-notification-routes: Merge queue/), say: "In the merge queue, with its place inside the stop" },
  { hover: button("Pull request: chore/bump-the-lockfile"), say: "One nobody in Armada owns is only its branch, from Pull request on" },
  { hover: inside(notes, text("Migration notes")), say: "The owner opens on a press" },
  { press: notes, say: "Pressing the owner opens that Session" },
]);

export { baysOutsidePrs };
