// Flight control as one row, two switches and buttons, with the Job's views as tabs on its head and Compact in the context figure: the take `tabs`, played on `?take=tabs` in its
// look `nav`, the default. Thread, Artifacts, Checks, Subagents and Bays are the tabs, and a tab with nothing behind it is not drawn. The owner's ask of 25 Oct 2026,
// one of two takes. Goes with the take that is not picked.

import { button, inside, region, role, tab, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const flightControlTabs = walk("flight-control-takes", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { press: inside(controls, role("switch", "Auto land")), say: "Auto land on" },
  { press: button("Context 84%"), say: "Compact is in the context figure" },
  { press: inside(role("region", /^Job: /), button("Compact")), say: "Compact sent" },
  { press: tab("Artifacts"), say: "Five views over the thread, as tabs" },
  { press: tab("Thread"), say: "Back to the thread" },
  { hover: inside(controls, button("Judge")), say: "Judge is off. The tooltip says why" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { look: tab("Artifacts"), say: "Only the views with something in them. Off buttons keep their seats" },
]);

export { flightControlTabs as "flight-control-tabs" };
