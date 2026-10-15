// Jobs, a place in the title bar beside Cockpit and Sessions: every Job there is, in the Sessions page's own
// layout, with Active, Done and All as tabs. Over `fleet-timeline`, which holds Jobs in both states in two repositories.

import { button, inside, place, region, role, tab, text, walk } from "../walk";

const jobs = region("Jobs");

export const jobsPage = walk("fleet-timeline", [
  { look: place("Jobs"), say: "Jobs, the second place in the title bar, after Cockpit" },
  { press: place("Jobs"), say: "It opens the list of Jobs" },
  { look: jobs, say: "The Sessions page's layout: a band, the tabs under it, a column for each heading. A Job's tile is a Session's: the title on top, the time at the top right, its facts on one line below" },
  { hover: inside(jobs, role("img", "Running")), say: "State is the tile's edge and its mark, which turns only while the Job runs" },
  { look: text(/Edit apps\/desktop\/src\/renderer\/src\/resize-handler\.ts/), say: "A running Job's Now: what its Drone is doing, how long ago, one mono line" },
  { hover: inside(jobs, role("img", /· 3 of 5 tasks done/)), say: "How far along it is in its step: its plan's tasks as a bar, done first. The branch is on the Job's detail" },
  { look: text("Write apps/pocket/src/service-worker.ts"), say: "A Drone silent for eleven minutes reads it on the line" },
  { press: inside(jobs, button(/More for Debounce/)), say: "The less used acts are behind the ellipsis at the tile's top right, shown when the tile is pointed at" },
  { look: text("Kill"), say: "Pause, Kill and the rest, with the key" },
  { key: "Escape", on: inside(jobs, button(/More for Debounce/)), say: "Esc closes it" },
  { press: tab("Done"), say: "Done: what is over" },
  { look: jobs, say: "Failed and landed Jobs, dashed and receded; Redispatch and Clear are behind the ellipsis" },
  { press: tab("All"), say: "All: both" },
  { press: inside(jobs, button("Order the store migrations", { exact: true })), say: "A press anywhere on a tile opens the Job, as it does from the Cockpit" },
  { look: place("Jobs"), say: "Jobs stays marked in the title bar while its Job is open" },
]);
