// Add and Remove on Kit's Skills and Plugins pages: the commands shown before anything runs, a failing install's own
// error in place, the new rows, a skill to the Trash and back with Undo, and a plugin removed after a confirm.
// Over `kit-add-and-remove`.

import { button, dialog, inside, region, role, systemPage, text, toSystemPage, walk } from "../walk";

const PLUGINS = region("Plugins", { exact: true });
const SKILLS = region("Skills", { exact: true });
const DETAIL = region("Detail", { exact: true });
const source = inside(DETAIL, role("textbox", "Source"));
const add = inside(DETAIL, button("Add", { exact: true }));

const kitAddAndRemove = walk("kit-add-and-remove", [
  ...toSystemPage("Plugins", "Plugins"),
  { press: inside(PLUGINS, button("Add plugin", { exact: true })), say: "Add, on the Plugins page" },
  { type: "example/no-such-plugin", into: source, say: "A source pasted" },
  { look: inside(DETAIL, region("Runs")), say: "The commands Add will run, shown before anything runs" },
  { look: inside(DETAIL, role("checkbox", "Also add to my Kit manifest")), say: "Checked by default" },
  { press: add, say: "Add" },
  { look: inside(DETAIL, region("Install failed")), say: "The command failed: its own error, in place, and no row added" },
  { type: "pbakaus/impeccable", into: source, say: "Another source" },
  { look: inside(inside(DETAIL, region("Runs")), text(/claude plugin install impeccable@impeccable/)), say: "The marketplace is added, then the plugin installed from it" },
  { press: add, say: "Add" },
  { look: inside(PLUGINS, button(/^impeccable/)), say: "The new plugin is in the list" },
  { press: inside(PLUGINS, button(/^impeccable/)), say: "The new plugin" },
  { look: inside(role("article", "impeccable"), button(/^impeccable/)), say: "It brings a skill" },
  { press: systemPage("Skills"), say: "Skills" },
  { look: inside(SKILLS, button(/^impeccable/)), say: "The plugin's skill is under Skills, marked with the plugin" },
  { press: inside(SKILLS, button(/^impeccable/)), say: "A skill the plugin brought" },
  { look: text(/^Comes with the impeccable plugin/), say: "No Remove of its own: it comes with the plugin" },
  { press: inside(SKILLS, button("Add skill", { exact: true })), say: "Add, on the Skills page" },
  { look: inside(DETAIL, role("radio", "Skill")), say: "Skill is picked, from the page" },
  { type: "example/release-notes", into: source, say: "A skill's source" },
  { look: inside(inside(DETAIL, region("Runs")), text(/npx skills add example\/release-notes/)), say: "The one command a skill takes" },
  { press: add, say: "Add" },
  { look: inside(SKILLS, button(/^release-notes/)), say: "The new skill is in the list, as Yours" },
  { press: inside(SKILLS, button(/^humanizer/)), say: "One of your own skills" },
  { press: button("Remove humanizer", { exact: true }), say: "Remove" },
  { look: inside(SKILLS, role("status")), say: "It went to the Trash, and Undo is offered" },
  { press: inside(SKILLS, button("Undo", { exact: true })), say: "Undo" },
  { look: inside(SKILLS, button(/^humanizer/)), say: "The row is back where it was" },
  { press: systemPage("Plugins"), say: "Plugins" },
  { press: inside(PLUGINS, button(/^impeccable/)), say: "The plugin" },
  { press: button("Remove impeccable", { exact: true }), say: "Remove" },
  { look: dialog("Remove impeccable"), say: "A confirm first: the command it runs, what goes with it, and no Undo" },
  { press: inside(dialog("Remove impeccable"), button("Remove", { exact: true })), say: "Remove" },
  { look: PLUGINS, say: "The plugin is gone, and its skill with it" },
]);

export { kitAddAndRemove as "kit-add-and-remove" };
