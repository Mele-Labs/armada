// A check pressed in a Session open on the Cockpit's stage opens its panel there, over the Cockpit,
// and closing it leaves the Session open. Over `session-checks`.

import { button, dialog, inside, region, walk } from "../walk";

const list = region("Work", { exact: true });
const pane = region("Session: Session check rows");

const cockpitSessionCheckPanel = walk("session-checks", [
  { press: inside(list, button("Session check rows")), say: "A Session on the Cockpit's stage, its Checks in its thread" },
  { look: inside(pane, button("Check app_smoke, running")), say: "One still out" },
  { press: inside(pane, button("Check desktop:typecheck, passed")), say: "Pressing a Check opens its panel here, on the Cockpit" },
  { look: dialog("Check log"), say: "That run, open on its log" },
  { look: list, say: "The Cockpit is still behind it" },
  { press: inside(dialog("Check log"), button("Close")), say: "Closing it leaves the Session open" },
  { look: pane, say: "Still on the stage" },
]);

export { cockpitSessionCheckPanel as "cockpit-session-check-panel" };
