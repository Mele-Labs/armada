// A link dropped anywhere in Bridge. Pasted outside a text box it opens a panel over the page that looks at it at once
// and offers three things: install it in Kit, start a Job with it, or add it to the Job open. Pasted into a text box it
// goes in as typed, and an offer to open the panel stands beside the box. Kit's own Add opens the same panel, empty.
// Then an install that fails, a skill to the Trash and back with Undo, and a plugin removed after a confirm.
// Over `kit-add-and-remove`.

import { toSessions } from "../sessions/walk-kit";
import { button, deckRow, dialog, inside, region, role, systemPage, text, toSystemPage, walk } from "../walk";

const LINK = "https://impeccable.style/";
const PANEL = dialog("Add a link");
const FOUND = inside(PANEL, region("Found", { exact: true }));
const source = inside(PANEL, role("textbox", "Source"));
const lookAt = inside(PANEL, button("Look", { exact: true }));
const install = inside(PANEL, button("Install in Kit", { exact: true }));
const startJob = inside(PANEL, button("Start a Job with it", { exact: true }));
const close = inside(PANEL, role("button", /^Close( panel)?( Esc)?$/));
const PLUGINS = region("Plugins", { exact: true });
const SKILLS = region("Skills", { exact: true });
const JOBS = region("Jobs", { exact: true });

const kitAddAndRemove = walk("kit-add-and-remove", [
  toSessions,
  { press: deckRow(/Release notes script/), say: "A Job open on the Cockpit" },
  { paste: JOBS, text: LINK, say: "A link someone shared, pasted with nothing focused" },
  { look: FOUND, say: "A panel over the page looks at it at once: a plugin, with its repository, marketplace and the skills it brings" },
  { look: inside(PANEL, region("Runs")), say: "The commands Install in Kit runs, from Fleet's answer, before anything runs" },
  { look: inside(PANEL, button("Add to this Job", { exact: true })), say: "The Job open is offered, by name" },
  { press: inside(PANEL, button("Add to this Job", { exact: true })), say: "Add to this Job" },
  { look: inside(PANEL, role("status")), say: "Sent to the Job, as a message from you" },
  { press: close, say: "Closed" },
  { look: inside(region("Thread"), text(/^Use https:\/\/impeccable\.style\//)), say: "In the Job's thread, asking it to use the plugin" },
  { paste: JOBS, text: LINK, say: "Pasted again" },
  { look: inside(PANEL, role("checkbox", "Also add to my Kit manifest")), say: "Checked by default" },
  { press: install, say: "Install in Kit" },
  { look: inside(PANEL, role("status")), say: "Installed" },
  { press: close, say: "Closed" },
  ...toSystemPage("Plugins", "Plugins"),
  { look: inside(PLUGINS, button(/^impeccable/)), say: "The new plugin is in the list" },
  { press: systemPage("Skills"), say: "Skills" },
  { look: inside(SKILLS, button(/^impeccable/)), say: "Its skill is under Skills, marked with the plugin" },
  { press: inside(SKILLS, button(/^impeccable/)), say: "A skill the plugin brought" },
  { look: text(/^Comes with the impeccable plugin/), say: "No Remove of its own: it comes with the plugin" },
  { paste: inside(SKILLS, role("searchbox", "Filter Skills")), text: LINK, say: "A link pasted into a text box" },
  { look: inside(SKILLS, role("searchbox", "Filter Skills")), say: "It goes in as typed" },
  { look: role("group", "Link pasted"), say: "And an offer to open the panel stands beside the box for a few seconds" },
  { type: "", into: inside(SKILLS, role("searchbox", "Filter Skills")), say: "The filter cleared" },
  { press: inside(role("navigation", "System"), button("Add to Kit", { exact: true })), say: "Kit's own Add, in the sidebar, opens the same panel, empty" },
  { type: "https://example.com/notes", into: source, say: "A source that is neither" },
  { press: lookAt, say: "Look" },
  { look: FOUND, say: "No plugin or skill found, in Fleet's words, and no Install" },
  { press: startJob, say: "A Job is the way on, so it is the first act" },
  { look: inside(PANEL, role("status")), say: "The Job started" },
  { type: "example/broken-plugin", into: source, say: "A source whose install fails" },
  { press: lookAt, say: "Look" },
  { press: install, say: "Install in Kit" },
  { look: inside(PANEL, region("Install failed")), say: "The command's own error, in place, and no row added" },
  { press: close, say: "Closed" },
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
