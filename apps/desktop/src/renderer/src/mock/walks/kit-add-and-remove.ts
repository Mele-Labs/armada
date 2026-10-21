// Add and Remove on Kit's Skills and Plugins pages. Add looks at a source first and draws what it is, with the
// commands Install runs, before anything runs: a plugin from a website's link, a skill, a source that is neither, and
// an install whose command fails. A Job to set the source up is offered on every answer. Then a skill to the Trash and
// back with Undo, and a plugin removed after a confirm. Over `kit-add-and-remove`.

import { button, dialog, inside, region, role, systemPage, text, toSystemPage, walk } from "../walk";

const PLUGINS = region("Plugins", { exact: true });
const SKILLS = region("Skills", { exact: true });
const DETAIL = region("Detail", { exact: true });
const FOUND = inside(DETAIL, region("Found", { exact: true }));
const source = inside(DETAIL, role("textbox", "Source"));
const lookAt = inside(DETAIL, button("Look", { exact: true }));
const install = inside(DETAIL, button("Install", { exact: true }));
const job = inside(DETAIL, button("Start a Job to set it up", { exact: true }));

const kitAddAndRemove = walk("kit-add-and-remove", [
  ...toSystemPage("Plugins", "Plugins"),
  { press: inside(PLUGINS, button("Add", { exact: true })), say: "Add, the same on Plugins and Skills" },
  { type: "https://impeccable.style/", into: source, say: "A website's link pasted" },
  { press: lookAt, say: "Look: Fleet works out what it is" },
  { look: FOUND, say: "A plugin: its name, its repository, its marketplace and the skills it brings" },
  { look: inside(DETAIL, region("Runs")), say: "The commands Install runs, from Fleet's answer, before anything runs" },
  { look: inside(DETAIL, role("checkbox", "Also add to my Kit manifest")), say: "Checked by default" },
  { press: job, say: "Or a Job sets it up and reports back" },
  { look: inside(DETAIL, role("status")), say: "The Job started" },
  { press: install, say: "Install" },
  { look: inside(PLUGINS, button(/^impeccable/)), say: "The new plugin is in the list" },
  { press: systemPage("Skills"), say: "Skills" },
  { look: inside(SKILLS, button(/^impeccable/)), say: "Its skill is under Skills, marked with the plugin" },
  { press: inside(SKILLS, button(/^impeccable/)), say: "A skill the plugin brought" },
  { look: text(/^Comes with the impeccable plugin/), say: "No Remove of its own: it comes with the plugin" },
  { press: inside(SKILLS, button("Add", { exact: true })), say: "Add, on Skills" },
  { type: "example/release-notes", into: source, say: "Another source" },
  { press: lookAt, say: "Look" },
  { look: FOUND, say: "A skill, by its name" },
  { look: inside(inside(DETAIL, region("Runs")), text(/npx skills add example\/release-notes/)), say: "The one command a skill takes" },
  { press: install, say: "Install" },
  { look: inside(SKILLS, button(/^release-notes/)), say: "The new skill is in the list, as Yours" },
  { press: inside(SKILLS, button("Add", { exact: true })), say: "Add" },
  { type: "https://example.com/notes", into: source, say: "A source that is neither" },
  { press: lookAt, say: "Look" },
  { look: FOUND, say: "No plugin or skill found, in Fleet's words" },
  { press: job, say: "A Job is the way on, so it is the first act" },
  { look: inside(DETAIL, role("status")), say: "The Job started" },
  { type: "example/broken-plugin", into: source, say: "A source whose install fails" },
  { press: lookAt, say: "Look" },
  { press: install, say: "Install" },
  { look: inside(DETAIL, region("Install failed")), say: "The command's own error, in place, and no row added" },
  { press: inside(SKILLS, button(/^humanizer/)), say: "One of your own skills" },
  { press: button("Remove humanizer", { exact: true }), say: "Remove" },
  { look: inside(SKILLS, role("status")), say: "It went to the Trash, and Undo is offered" },
  { press: inside(SKILLS, button("Undo", { exact: true })), say: "Undo" },
  { look: inside(SKILLS, button(/^humanizer/)), say: "The row is back where it was" },
  { press: systemPage("Plugins"), say: "Plugins" },
  { press: inside(PLUGINS, button(/^impeccable/)), say: "The plugin" },
  { press: button("Remove impeccable", { exact: true }), say: "Remove" },
  { look: dialog("Remove impeccable"), say: "A confirm first: the command Fleet runs, what goes with it, and no Undo" },
  { press: inside(dialog("Remove impeccable"), button("Remove", { exact: true })), say: "Remove" },
  { look: PLUGINS, say: "The plugin is gone, and its skill with it" },
]);

export { kitAddAndRemove as "kit-add-and-remove" };
