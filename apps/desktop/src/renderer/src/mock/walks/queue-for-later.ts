// An idea saved to start later. A Session's dispatcher has Model and Effort beside the act and Queue for later next to Start,
// ⌘⇧↵ on the request does the same, and the idea waits in its repository's group of the Cockpit's list, a dashed row with Start and Remove.
// Over `queue-for-later`, where the 5-hour limit is nearly spent.

import { button, inside, region, role, tab, walk } from "../walk";

const cockpit = region("Jobs", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const repository = (name: string) => inside(frame, role("radio", `Repository ${name}`));
const idea = (title: string) => inside(cockpit, button(title, { exact: true }));

const first = "Make the Cleanup grid sortable by size";
const second = "Draft the billing export notes";

const queueForLater = walk("queue-for-later", [
  { look: role("button", "5-hour limit"), say: "The 5-hour limit is nearly spent" },
  { key: "n", on: cockpit, say: "N opens the dispatcher" },
  { press: inside(frame, tab("Job")), say: "Job" },
  { look: inside(frame, role("combobox", "Model")), say: "Model and Effort sit beside the act" },
  { type: first, into: request, say: "An idea" },
  { press: repository("armada"), say: "For armada" },
  { type: "opus", into: inside(frame, role("combobox", "Model")), say: "On opus" },
  { look: inside(frame, button("Queue for later")), say: "Queue for later is next to Start" },
  { press: inside(frame, button("Queue for later")), say: "Queue it" },
  { key: "n", on: cockpit, say: "N opens the dispatcher again" },
  { type: second, into: request, say: "A second idea" },
  { press: repository("billing"), say: "For billing" },
  { key: "Meta+Shift+Enter", on: request, say: "⌘⇧↵ on the request queues it" },
  { look: idea(first), say: "The first sits in armada's group, dashed" },
  { look: idea(second), say: "The second sits in billing's" },
  { hover: inside(cockpit, role("img", "Model opus")), say: "The model is on the row" },
  { hover: inside(cockpit, button(new RegExp(`^Start ${first}`))), say: "Start is a press beside the row, and ↵ on the row does the same" },
  { press: inside(cockpit, button(new RegExp(`^Start ${first}`))), say: "Start the first" },
  { look: role("textbox", "Message"), say: "It is a Job now, holding the idea's words" },
  { look: inside(cockpit, role("heading", "armada")), say: "In armada's group, where the idea was" },
]);

export { queueForLater as "queue-for-later" };
