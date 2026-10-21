// A new Job dispatched, and its first turn asks how the work is delivered: Needs you is already open on the question when the Job opens, with no
// click. Over `dispatch-asks-delivery`.

import { inside, region, role, text, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));

const dispatchAsksDelivery = walk("dispatch-asks-delivery", [
  { key: "n", on: jobs, say: "n opens Dispatch on the repository the rail is on" },
  { type: "Add a retry cap to the store client", into: request, say: "Words for a Job" },
  { press: inside(frame, role("radio", "Repository armada", { exact: true })), say: "The repository it runs in" },
  { press: inside(frame, role("button", "Start", { exact: true })), say: "Start. The Job opens itself" },
  { look: text(/Add a retry cap to the store client/), say: "The Job opens on its words" },
  { later: jobs, say: "Its first turn ends on a question: how should this work be delivered" },
  { look: text(/How should this work be delivered/), say: "Needs you is already open on it. Nothing was pressed" },
]);

export { dispatchAsksDelivery as "dispatch-asks-delivery" };
