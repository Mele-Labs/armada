// The four Studio changes the owner approved on 2 Oct 2026, on a Zone with a relation proposed
// across a Note: a proposal is a dot that opens, a kind pressed inside a Zone lands in it, the
// palette's Add rows draw the rail's icons, and Run has a palette row and `R`.
// `studio-zone.test.tsx` and `studio-palette.test.tsx` hold the claims.

import { button, inside, role, walk } from "../walk";

const rail = role("group", "What you can put on this Studio");
const proposal = button(/^You proposed: /);

export const fourStudioFixes = walk("studio-zone-proposal", [
  { press: button("Studios", { exact: true }), say: "This repository's Studios" },
  { press: button("Open", { exact: true }), say: "A Studio with a Zone, opened read-only" },
  { look: proposal, say: "The relation you proposed is a dot on its line, moved off the Note the line crosses" },
  { press: proposal, say: "Pointed at, it opens: who proposed it, and that Continue answers it" },
  { look: rail, say: "Run and the kinds are off until Continue, each saying why" },
  { press: button("Continue", { exact: true }), say: "Continue makes the Studio editable" },
  { press: proposal, say: "Now it opens with Accept and Reject, under the dot rather than over the Note" },
  { press: inside(rail, button("Add a Note", { exact: true })), say: "Arm a Note on the rail" },
  { press: role("group", "Zone", { exact: true }), say: "Press inside the Zone" },
  { type: "Pressed inside the Zone, so it is in the Zone", into: role("textbox", "Note", { exact: true }), say: "Write it" },
  { press: button("Add note"), say: "Add it" },
  { look: role("group", /^Note: Pressed inside the Zone/), say: "It sits in the Zone, and moves with it" },
  { press: button("Search, or describe work"), say: "The palette, as ⌘K opens it" },
  { look: role("option", /^Add a note/), say: "Add a note, link and sketch draw the rail's icons" },
  { look: role("option", /^Run\b/), say: "Run has a row, with R beside it" },
  { press: role("option", /^Run\b/), say: "Choose it" },
  { look: role("menu"), say: "The rail's Run menu opens, as a press on Run or R does" },
]);
