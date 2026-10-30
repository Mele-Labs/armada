// Three Sessions held outside their slots (the owner, 30 Oct 2026): a read says read, a write says
// write, and a node script the guard cannot class says run, with the command above it.

import { button, inside, place, region, role, text, walk } from "../walk";

const CARD = role("article", "Waiting on you");
const JOBS = region("Jobs", { exact: true });

export const aReachCardThatSaysRun = walk("session-reach-outside", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: inside(JOBS, button(/Retry budget/)), say: "A job held on a cat of a file in Documents" },
  { look: inside(CARD, text("Permission to read")), say: "A known read says read" },
  { press: place("Cockpit"), say: "Back to Jobs" },
  { press: inside(JOBS, button(/Release notes/)), say: "A job held on a write to the Desktop" },
  { look: inside(CARD, text("Permission to write")), say: "A known write says write" },
  { press: place("Cockpit"), say: "Back to Jobs" },
  { press: inside(JOBS, button(/Tidy other repo/)), say: "A job held on a node script handed another repo" },
  { look: inside(CARD, text("Permission to run")), say: "The guard cannot tell what it does, so it says run" },
  { look: inside(CARD, text(/tidy\.mjs ~\/code\/other-repo/)), say: "The command it would run" },
  { look: inside(CARD, text("Always applies to commands on")), say: "Always covers commands on this path" },
]);
