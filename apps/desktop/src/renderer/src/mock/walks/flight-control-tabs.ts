// Flight control as one row, with the Job's views as tabs over the thread: the take `tabs`, played on `?take=tabs`. Thread, Artifacts, Checks, Subagents and
// Bays are the tabs, and a tab with nothing behind it is not drawn. The owner's ask of 25 Oct 2026, one of two takes. Goes with the take that is not picked.

import { button, inside, region, role, tab, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");

const flightControlTabs = walk("flight-control-takes", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { look: inside(controls, role("switch", "Auto land")), say: "Flight control is one row" },
  { press: inside(controls, role("switch", "Auto land")), say: "Auto land on" },
  { press: tab("Artifacts"), say: "Five views over the thread, as tabs" },
  { press: tab("Thread"), say: "Back to the thread" },
  { hover: inside(controls, button("Judge")), say: "Judge is off. The tooltip says why" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { look: tab("Artifacts"), say: "Only the views with something in them. Off buttons end the row as glyphs" },
]);

export { flightControlTabs as "flight-control-tabs" };
