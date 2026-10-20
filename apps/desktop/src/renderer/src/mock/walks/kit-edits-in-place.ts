// A Kit item's file is its detail: a skill shows its SKILL.md whole in the editor, a sub agent its file, and a plugin,
// a folder, offers Show in Finder alone. Over `kit-edits-in-place`.

import { button, inside, region, role, systemPage, toSystemPage, walk } from "../walk";

const detail = (name: string) => role("article", name);
const editor = (name: string, file: string) => inside(detail(name), role("group", `Editor ${file}`, { exact: true }));

export const kitEditsInPlace = walk("kit-edits-in-place", [
  ...toSystemPage("Skills", "Skills"),
  { press: inside(region("Skills"), button(/^humanizer/)), say: "A skill" },
  { look: editor("humanizer", "SKILL.md"), say: "Its SKILL.md is the detail, in the editor" },
  { type: "@8 \n\nRead it aloud before you send it.", into: editor("humanizer", "SKILL.md"), say: "A line added" },
  { look: inside(detail("humanizer"), role("img", "Unsaved")), say: "A dot says it is not saved" },
  { key: "Meta+s", on: editor("humanizer", "SKILL.md"), say: "⌘S saves" },
  { look: editor("humanizer", "SKILL.md"), say: "Saved" },
  { press: systemPage("Sub agents"), say: "Sub agents" },
  { press: inside(region("Sub agents"), button(/^reviewer/)), say: "A sub agent" },
  { look: editor("reviewer", "reviewer.md"), say: "Its file, whole" },
  { press: systemPage("Plugins"), say: "Plugins" },
  { press: inside(region("Plugins"), button(/^armada/)), say: "A plugin is a folder" },
  { look: inside(detail("armada"), button("Show in Finder")), say: "It offers Show in Finder alone" },
]);
