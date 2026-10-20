// Kit's Open shows an item's file in the side panel: a skill opens its SKILL.md, a sub agent its file, and a plugin,
// a folder, offers Show in Finder alone. Over `kit-opens-in-panel`.

import { button, dialog, inside, region, role, systemPage, toSystemPage, walk } from "../walk";

const editor = (name: string) => inside(dialog(name), role("group", `Editor ${name}`, { exact: true }));
const close = (name: string) => inside(dialog(name), role("button", /^Close/));

export const kitOpensInPanel = walk("kit-opens-in-panel", [
  ...toSystemPage("Skills", "Skills"),
  { press: inside(region("Skills"), button(/^humanizer/)), say: "A skill, and its detail" },
  { press: inside(role("article", "humanizer"), button("Open", { exact: true })), say: "Open" },
  { look: editor("SKILL.md"), say: "The panel opens on its SKILL.md, in the editor" },
  { press: close("SKILL.md"), say: "Close" },
  { press: systemPage("Sub agents"), say: "Sub agents" },
  { press: inside(region("Sub agents"), button(/^reviewer/)), say: "A sub agent, one file" },
  { press: inside(role("article", "reviewer"), button("Open", { exact: true })), say: "Open" },
  { look: editor("reviewer.md"), say: "The panel opens on it" },
  { press: close("reviewer.md"), say: "Close" },
  { press: systemPage("Plugins"), say: "Plugins" },
  { press: inside(region("Plugins"), button(/^armada/)), say: "A plugin is a folder" },
  { look: inside(role("article", "armada"), button("Show in Finder")), say: "It offers Show in Finder alone" },
]);
