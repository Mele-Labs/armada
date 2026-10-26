// The Job's views as a panel under the Jobs list, played on `?take=tabs&views=left`. A walk carries no query of its own, so the address names the place. The owner's ask of 25 Oct 2026.
// Goes with the take that is not picked.

import { button, inside, region, walk } from "../walk";

const list = region("Jobs", { exact: true });
const views = region("Views", { exact: true });
const flightControlLeft = walk("flight-control-takes", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { look: inside(views, button("Artifacts")), say: "Its views are a panel under the list, one row each" },
  { press: inside(views, button("Artifacts")), say: "Artifacts over the thread. The row stays lit" },
  { press: inside(views, button("Artifacts")), say: "The lit row again goes back to the thread" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { hover: inside(views, button("Checks")), say: "Checks is off. The tooltip says why, and the row keeps its seat" },
]);

export { flightControlLeft as "flight-control-left" };
