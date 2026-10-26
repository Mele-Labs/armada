// The same take as `flight-control-tabs`, played on `?take=tabs&views=composer`: the Job's views are icon buttons in the composer's row, and there is no tab row. A walk
// carries no query of its own, so the address names the place. The owner's ask of 25 Oct 2026. Goes with the take that is not picked.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const flightControlComposer = walk("flight-control-takes", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { look: button("Artifacts"), say: "Its views are buttons in the composer's row" },
  { press: button("Artifacts"), say: "Artifacts over the thread. The composer stays" },
  { press: button("Artifacts"), say: "The lit button again goes back to the thread" },
  { press: button("Context 84%"), say: "Compact is in the context figure" },
  { press: inside(role("region", /^Job: /), button("Compact")), say: "Compact sent" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { hover: button("Checks"), say: "Checks is off. The tooltip says why, and the button keeps its seat" },
]);

export { flightControlComposer as "flight-control-composer" };
