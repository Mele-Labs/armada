// Run sits on a Studio's rail beside Note, Link and Sketch, off with its reason
// while the Studio is read-only — the owner, 2 Oct 2026.

import { button, inside, role, walk } from "../walk";

const rail = role("group", "What you can put on this Studio");

export const runOnTheStudioRail = walk("studios", [
  { press: button("Studios", { exact: true }), say: "This repository's Studios" },
  { press: role("cell", "The Board's legend", { exact: true }), say: "Opened from the list, it is read-only" },
  { look: inside(rail, button("Run", { exact: true })), say: "Run is on the rail, off until Continue" },
  { press: button("Continue"), say: "Continue makes the Studio editable" },
  { look: rail, say: "Note, Link, Sketch, and Run" },
  { press: inside(rail, button("Run", { exact: true })), say: "Run opens what the checkout declares" },
  { look: role("menu"), say: "Checks and Commands, then servers" },
]);
