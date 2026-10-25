// A Job dispatched from the Dispatch sheet opens held. The title fills in, Kind is changed in the composer's bar and Lands follows,
// and Start begins the agent with nothing in Needs you. Over `job-start-card`.

import { inside, region, role, text, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));

const jobStartCard = walk("job-start-card", [
  { key: "n", on: jobs, say: "n opens Dispatch on the repository the rail is on" },
  { type: "Add a retry cap to the store client", into: request, say: "Words for a Job" },
  { press: inside(frame, role("radio", "Repository armada", { exact: true })), say: "The repository it runs in" },
  { press: inside(frame, role("button", "Start", { exact: true })), say: "Send. The Job opens and no agent moves" },
  { look: role("button", /rename$/), say: "The title fills in from the request and renames in place" },
  { hover: role("img", "Your usual for features in armada", { exact: true }), say: "Lands is set to his usual for features" },
  { type: "bug", into: role("combobox", "Kind"), say: "Kind changes to Bug and Lands follows to his usual for bugs" },
  { press: role("button", "Start", { exact: true }), say: "Start" },
  { look: text(/Read crates\/store\/src\/client\.rs/), say: "The agent is at work. Needs you is empty" },
]);

export { jobStartCard as "job-start-card" };
