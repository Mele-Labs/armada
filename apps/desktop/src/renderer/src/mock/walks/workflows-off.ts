// Workflows switched off (29 Oct 2026): Kit, Workflows holds the switch for Jobs, the same setting as Settings, Features, and a switch on each row. On All a row's
// switch is the machine's, with one picked it is that repository's, and a workflow off on the machine says so in its place in every repository. A held Job's
// Workflow picker leaves off workflows out, and a Job with no workflow reads Brief, Done, Now and Open. Fleet serves `workflows.off` in the settings list; here the mock's settings slice does.
// Over `workflows-off`.

import { button, deckRow, inside, place, region, role, text, toSystemPage, walk } from "../walk";

const armada = role("group", "Armada", { exact: true });
const panel = (name: string) => region(name, { exact: true });

const workflowsOff = walk("workflows-off", [
  ...toSystemPage("Workflows", "Workflows, with All repositories picked"),
  { look: role("switch", "Workflows for Jobs"), say: "Workflows for Jobs, at the top" },
  { press: role("switch", "Workflows for Jobs"), say: "Off" },
  ...toSystemPage("Features", "Settings, Features"),
  { look: role("switch", /^Workflows for Jobs/), say: "The same switch, off here too" },
  { press: role("switch", /^Workflows for Jobs/), say: "On again" },
  ...toSystemPage("Workflows", "Workflows"),
  { look: role("switch", "Off on this machine"), say: "A switch on each row. Prototype is off on this machine" },
  { press: inside(armada, role("switch", "On on this machine")), say: "Design plan is off for the whole machine" },
  { press: button("Repository", { exact: true }), say: "Pick one repository" },
  { press: role("menuitem", /armada$/), say: "armada" },
  { look: role("switch", "Off in armada"), say: "Bug is off in armada only" },
  { look: text("Off on this machine"), say: "A workflow off on the machine says so in place of its switch" },
  { press: role("switch", "On in armada"), say: "Docs only is off for armada only" },
  { press: place("Cockpit"), say: "Cockpit" },
  { press: deckRow("Add a retry cap to the store client"), say: "A Job held on its start card" },
  { press: role("combobox", "Workflow"), say: "Workflow lists Feature. The three that are off are gone, and None stays" },
  { press: deckRow("Retry cap in the store client"), say: "A Job with no workflow" },
  { look: panel("Brief"), say: "Brief" },
  { look: panel("Done"), say: "Done, folded" },
  { look: panel("Now"), say: "Now, with what waits on you" },
  { look: panel("Open"), say: "Open" },
  { key: "Alt+1", on: panel("Done"), say: "⌥1 opens Done, ⌥2 Now, ⌥3 Open" },
  { look: text(/Read the retry cap from settings/), say: "What the Job made, under Done" },
]);

export { workflowsOff as "workflows-off" };
