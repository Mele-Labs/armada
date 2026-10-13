// Jobs, a row of the rail's Work panel beside Cockpit and Sessions: every Job there is, in the Sessions page's frame
// with Active, Done and All as tabs. Over `fleet-timeline`, which holds Jobs in both states in two repositories.

import { button, inside, region, role, tab, walk } from "../walk";

const work = role("navigation", "Work");
const jobs = region("Jobs");

export const jobsPage = walk("fleet-timeline", [
  { look: inside(work, button("Jobs", { exact: true })), say: "Jobs, second in Work, under Cockpit" },
  { press: inside(work, button("Jobs", { exact: true })), say: "It opens the list of Jobs" },
  { look: jobs, say: "The Sessions page's frame: a band, the tabs, the glass. Active holds what needs him, what is running and what is queued" },
  { key: "j", on: jobs, say: "j and k move down and up the list, Enter opens the Job" },
  { press: tab("Done"), say: "Done: what is over" },
  { look: jobs, say: "Landed and failed Jobs, with the acts a finished Job has" },
  { press: tab("All"), say: "All: both" },
  { look: jobs, say: "Every Job in both repositories" },
  { press: inside(jobs, role("option", /Order the store migrations/)), say: "A Job opens as it does from the Cockpit" },
  { look: inside(work, button("Jobs", { exact: true })), say: "Jobs stays marked in the rail while its Job is open" },
]);
