// A right press on Fleet's behind-base notice in a Job's thread: a "Disable slot behind notice" header over For job, For repo, For 1 hour.
// The POST each choice sends is asserted in `../mute-fleet-notice.test.tsx`.

import { deckRow, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const notice = inside(region("Fleet turn"), text(/23 commits behind master/));

const open: Step[] = [
  { press: deckRow("Fix the flaky store test"), say: "Open the Job" },
  { look: notice, say: "Fleet's notice is in the thread" },
];

const choose = (item: string): Step[] => [
  { context: notice, say: "A right press on the notice" },
  { look: text("Disable slot behind notice"), say: "The header names what is being turned off" },
  { look: role("menuitem", "For job"), say: "For job" },
  { look: role("menuitem", "For repo"), say: "For repo" },
  { look: role("menuitem", "For 1 hour"), say: "For 1 hour" },
  { press: role("menuitem", item), say: item },
  { look: notice, say: "The thread stays as it was" },
];

const forJob = walk("mute-fleet-notice", [...open, ...choose("For job")]);
const forRepo = walk("mute-fleet-notice", [...open, ...choose("For repo")]);
const forHour = walk("mute-fleet-notice", [...open, ...choose("For 1 hour")]);
export { forJob as "mute-fleet-notice", forRepo as "mute-fleet-notice-repo", forHour as "mute-fleet-notice-hour" };
