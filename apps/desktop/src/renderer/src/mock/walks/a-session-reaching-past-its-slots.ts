// A Session in auto held on a read of ~/Documents (the owner, 16 Oct 2026): its card says it is a
// read and offers five answers over the path, a press on Documents picks the folder, Always allow
// settles it. A second Session's write to ~/Desktop says it is a write, and Always allow for this
// session settles it. Settings, Features lists Documents beside the blocked ~/.ssh.
// `session-reach-outside.test.tsx` holds the claims. #2441.

import { button, inside, place, region, role, text, toSystemPage, walk } from "../walk";

const CARD = role("article", "Waiting on you");
const PICKER = inside(CARD, role("group", "Folder for always"));
const ANSWERS = inside(CARD, role("group", "Answers"));
const LIST = role("list", "Folders jobs reach");

export const aSessionReachingPastItsSlots = walk("session-reach-outside", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: inside(region("Jobs", { exact: true }), button(/Custom margins/)), say: "A job held on a read outside its slots" },
  { look: CARD, say: "The only thing it waits on, so the panel opens on its card" },
  { look: inside(CARD, text(/echo gutterline$/)), say: "The whole command, to its last word" },
  { look: inside(CARD, text("Permission to read")), say: "A read" },
  { look: ANSWERS, say: "Reject once for this call, and Always in its own panel" },
  { look: inside(CARD, text("Always applies to reads of")), say: "The path in the panel is what Always covers" },
  { look: inside(ANSWERS, button("Always allow for this job HANDOFF.md", { exact: true })), say: "Allowed for this job only, and not listed in Settings" },
  { look: inside(PICKER, button("HANDOFF.md")), say: "It starts on the file itself" },
  { press: inside(PICKER, button("Documents")), say: "Press a folder to widen it" },
  { look: inside(CARD, text("/Users/user/Documents/*")), say: "Documents and all it holds, lit up to the end" },
  { press: inside(ANSWERS, button("Always allow Documents", { exact: true })), say: "Always allow everything under Documents" },
  { press: place("Cockpit"), say: "Back to Jobs" },
  { press: inside(region("Jobs", { exact: true }), button(/Release notes/)), say: "Another job, held on a write" },
  { look: inside(CARD, text("Permission to write")), say: "A write" },
  { look: inside(CARD, text("Always applies to writes to")), say: "Always covers writes to this path" },
  { press: inside(ANSWERS, button("Always allow for this job notes.md", { exact: true })), say: "Allowed for this job until it is closed" },
  ...toSystemPage("Settings", "Settings"),
  { press: role("tab", "Features"), say: "Features" },
  { look: inside(LIST, text("/Users/user/Documents")), say: "The folder it allowed" },
  { look: inside(LIST, text("/Users/user/.ssh")), say: "A folder blocked earlier" },
  { look: inside(LIST, button("Remove /Users/user/Documents")), say: "Remove takes it off" },
]);
