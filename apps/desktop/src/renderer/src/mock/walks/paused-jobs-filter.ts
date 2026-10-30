// Paused Jobs have their own Filter entry: they leave the Active list and read under Paused. Over `pausing-a-session`.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const filter = inside(list, button(/^Filter/));
const scope = (name: string) => role("menuitem", name, { exact: true });

const pausedJobsFilter = walk("pausing-a-session", [
  { look: inside(list, button("Pin the store clock")), say: "The list is on Active: the Jobs working or waiting on you" },
  { press: filter, say: "Open the Filter" },
  { look: scope("Paused"), say: "Paused sits between Active and Quiet" },
  { press: scope("Paused"), say: "Pick it" },
  { look: inside(list, button("Move the dock handle")), say: "Only the paused Jobs are here, each with the paused mark" },
  { press: filter, say: "Back to the Filter" },
  { press: scope("All"), say: "All lists them too" },
  { look: inside(list, button("Move the dock handle")), say: "The paused Job is among the rest" },
]);

export { pausedJobsFilter as "paused-jobs-filter" };
