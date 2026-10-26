// The same walk as `flight-control-tabs`, played on `?take=tabs&tabpos=foot`: the tabs are the Job panel's last row, under the composer. A walk carries no query of
// its own, so the address names the place. The owner's ask of 25 Oct 2026. Goes with the take that is not picked.

import { button, inside, region, role, tab, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const flightControlTabsFoot = walk("flight-control-takes", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { press: inside(controls, role("switch", "Auto land")), say: "Auto land on" },
  { press: button("Context 84%"), say: "Compact is in the context figure" },
  { press: inside(role("region", /^Job: /), button("Compact")), say: "Compact sent" },
  { press: tab("Artifacts"), say: "Five views, as the panel's footer" },
  { press: tab("Thread"), say: "Back to the thread" },
  { hover: inside(controls, button("Judge")), say: "Judge is off. The tooltip says why" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { look: tab("Artifacts"), say: "Only the views with something in them. Off buttons keep their seats" },
]);

export { flightControlTabsFoot as "flight-control-tabs-foot" };
