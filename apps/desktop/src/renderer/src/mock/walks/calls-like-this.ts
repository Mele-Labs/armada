// A job in auto held on a Bash write to ~/Desktop: its card names the call's kind in a Calls like row,
// starting on the script, a press on node widens it and a press on drive.mjs takes it back, and Only in
// starts off. Allowed for calls like it, the next drive.mjs call runs with no card, and the cleanup.mjs
// call after it holds a new card. `calls-like-this.test.tsx` holds the claims.

import { button, inside, place, region, role, text, walk } from "../walk";

const CARD = role("article", "Waiting on you");
const LIKE = inside(CARD, role("group", "Calls like"));
const THREAD = region("Thread");

export const callsLikeThis = walk("calls-like-this", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: inside(region("Jobs", { exact: true }), button(/Browser checks/)), say: "A job held on a write outside its slots" },
  { look: inside(CARD, text("Permission to write")), say: "A write" },
  { look: inside(LIKE, button("drive.mjs")), say: "Calls like starts on the script" },
  { press: inside(LIKE, button("node")), say: "Press node to allow every node call" },
  { press: inside(LIKE, button("drive.mjs")), say: "Press drive.mjs to allow this script only" },
  { look: inside(CARD, role("switch", "Only in shot.png")), say: "Only in is off, so the folder does not narrow it" },
  { press: inside(CARD, button("Allow calls like this for this job")), say: "Allow calls like this for this job" },
  { press: inside(THREAD, text("node, Bash")), say: "The calls it made" },
  { later: inside(THREAD, text(/drive\.mjs --browser/)), say: "The next drive.mjs call ran with no card" },
  { look: inside(LIKE, button("cleanup.mjs")), say: "A cleanup.mjs call is another script, so it holds a new card" },
]);
