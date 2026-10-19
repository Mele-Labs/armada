// The Settings row for large context set to carry on automatically, and a Job passing the line: it hands off with no card. The new Job takes the row and the earlier one sits behind it. Over `job-handoff-auto`.

import { button, deckRow, inside, place, region, role, toSystemPage, walk } from "../walk";

const list = region("Jobs", { exact: true });
const setting = role("combobox", "Large context");

export const jobHandoffAuto = walk("job-handoff-auto", [
  ...toSystemPage("Settings", "Settings"),
  { press: role("tab", "Agents"), say: "Agents, the group Large context is in" },
  { hover: setting, say: "The row names itself in a tooltip" },
  { look: setting, say: "Ask is what ships" },
  { type: "carry on automatically", into: setting, say: "Carry on automatically" },
  { press: place("Cockpit"), say: "Back to the Cockpit" },
  { look: deckRow("Retry the slot lease"), say: "A Job under the line, nothing asked" },
  { later: deckRow("Retry the slot lease"), say: "The Job passes the line" },
  { look: inside(list, role("img", "Handing off")), say: "It hands off with no card" },
  { look: inside(list, button("Earlier Job")), say: "One row, the same title and place, the earlier Job behind it" },
]);
