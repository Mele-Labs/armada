// The Cockpit and the Dispatch sheet: n opens it ready for the work, the one repository set up is chosen,
// and the words typed are the Session's request. Over `dashboard-quick-dispatch`.

import { inside, place, role, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));

const dashboardQuickDispatch = walk("dashboard-quick-dispatch", [
  { look: role("button", /^Search, or describe work to dispatch/), say: "One bar in the title row, for search" },
  { key: "n", on: place("Cockpit"), say: "Press n" },
  { look: request, say: "The Dispatch sheet opens, ready for the work" },
  { type: "Retire the sleep calls in the store tests", into: request, say: "The first words typed" },
  { look: inside(frame, role("button", "Start", { exact: true })), say: "The act is Start" },
]);

export { dashboardQuickDispatch as "dashboard-quick-dispatch" };
