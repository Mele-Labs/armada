// The Job's views as icon buttons in its title row, played on `?take=tabs&views=head`. A walk carries no query of its own, so the address names the place. The owner's ask of 25 Oct 2026.
// Goes with the take that is not picked.

import { button, inside, region, walk } from "../walk";

const list = region("Jobs", { exact: true });
const flightControlHead = walk("flight-control-takes", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { look: button("Artifacts"), say: "Its views are buttons in the title row, before the Job's own" },
  { press: button("Artifacts"), say: "Artifacts over the thread. The head and the composer stay" },
  { press: button("Artifacts"), say: "The lit button again goes back to the thread" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { hover: button("Checks"), say: "Checks is off. The tooltip says why, and the button keeps its seat" },
]);

export { flightControlHead as "flight-control-head" };
