// Flight control in three groups, no pages: the take `grouped`, played on `?take=grouped`. Job holds Auto land and Pause as switches and Release; Tell the
// agent holds Update, Compact, Judge and Report; Open holds Artifacts, Checks, Subagents, Bays, Terminal and Requests. An off tile ends its group as a glyph.
// The owner's ask of 25 Oct 2026, one of two takes. Goes with the take that is not picked.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const group = (name: string) => inside(controls, role("group", name, { exact: true }));

const flightControlGrouped = walk("flight-control-takes", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { look: group("Job"), say: "Three groups: Job, Tell the agent, Open" },
  { press: inside(controls, role("switch", "Auto land")), say: "Auto land on" },
  { press: inside(group("Open"), button("Artifacts")), say: "Artifacts over the thread" },
  { press: inside(region("Artifacts", { exact: true }), button("Back")), say: "Back to the thread" },
  { hover: inside(group("Tell the agent"), button("Judge")), say: "Judge is off. The tooltip says why" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { look: group("Open"), say: "Off ones end the group as glyphs" },
]);

export { flightControlGrouped as "flight-control-grouped" };
