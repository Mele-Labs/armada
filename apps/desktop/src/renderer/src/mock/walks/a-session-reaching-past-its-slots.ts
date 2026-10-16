// A Session in auto held on a read of ~/Documents (the owner, 16 Oct 2026): its card offers four
// answers over the path, a press on Documents picks the folder, Always allow settles it, and
// Settings, Features lists it beside the blocked ~/.ssh. `session-reach-outside.test.tsx` holds
// the claims. #2441.

import { button, inside, place, region, role, text, toSystemPage, walk } from "../walk";

const CARD = role("article", "Waiting on you");
const PICKER = inside(CARD, role("group", "Folder for always"));
const ANSWERS = inside(CARD, role("group", "Answers"));
const LIST = role("list", "Folders sessions reach");

export const aSessionReachingPastItsSlots = walk("session-reach-outside", [
  { press: place("Sessions"), say: "Sessions" },
  { press: inside(region("Sessions"), button(/Custom margins/)), say: "A session held on a read outside its slots" },
  { look: CARD, say: "What it wants to read, and why it is asked" },
  { look: ANSWERS, say: "Reject once for this call, and Always in its own panel" },
  { look: inside(CARD, text("Always applies to")), say: "The path in the panel is what Always covers" },
  { look: inside(PICKER, button("HANDOFF.md")), say: "It starts on the file itself" },
  { press: inside(PICKER, button("Documents")), say: "Press a folder to widen it" },
  { look: inside(CARD, text("/Users/user/Documents and everything in it")), say: "Documents and all it holds, lit up to the end" },
  { press: inside(ANSWERS, button("Always allow")), say: "Always allow everything under Documents" },
  ...toSystemPage("Settings", "Settings"),
  { press: role("tab", "Features"), say: "Features" },
  { look: inside(LIST, text("/Users/user/Documents")), say: "The folder it allowed" },
  { look: inside(LIST, text("/Users/user/.ssh")), say: "A folder blocked earlier" },
  { look: inside(LIST, button("Remove /Users/user/Documents")), say: "Remove takes it off" },
]);
