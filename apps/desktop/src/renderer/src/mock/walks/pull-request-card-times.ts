// The merge queue chip's hover card gains three ages: how long the pull request has been open, when it last changed,
// and how long it has waited in the queue. A stamp reads as an age, and its tooltip gives the clock time.

import { button, inside, region, walk } from "../walk";

const queue = region("Merge queue");

const pullRequestCardTimes = walk("cockpit-ship-flightdeck", [
  {
    hover: inside(queue, button(/^#1890/)),
    say: "The queued pull request's card lists Opened, Updated and In queue, each as an age",
  },
  {
    hover: inside(queue, button(/^#1896/)),
    say: "One outside the queue has Opened and Updated, and no In queue row",
  },
]);

export { pullRequestCardTimes };
