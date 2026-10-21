// A right press on Fleet's behind-base notice in a Job's thread: Disable for job, Disable for repo, Disable for 1 hour.
// The POST each choice sends is asserted in `../mute-fleet-notice.test.tsx`.

import { deckRow, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const notice = inside(region("Message from Fleet"), text(/23 commits behind master/));

const open: Step[] = [
  { press: deckRow("Fix the flaky store test"), say: "Open the Job" },
  { look: notice, say: "Fleet's notice is in the thread" },
];

const choose = (item: string): Step[] => [
  { context: notice, say: "A right press on the notice" },
  { look: role("menuitem", "Disable for job"), say: "Disable for job" },
  { look: role("menuitem", "Disable for repo"), say: "Disable for repo" },
  { look: role("menuitem", "Disable for 1 hour"), say: "Disable for 1 hour" },
  { press: role("menuitem", item), say: item },
  { look: notice, say: "The thread stays as it was" },
];

const forJob = walk("mute-fleet-notice", [...open, ...choose("Disable for job")]);
const forRepo = walk("mute-fleet-notice", [...open, ...choose("Disable for repo")]);
const forHour = walk("mute-fleet-notice", [...open, ...choose("Disable for 1 hour")]);
export { forJob as "mute-fleet-notice", forRepo as "mute-fleet-notice-repo", forHour as "mute-fleet-notice-hour" };
