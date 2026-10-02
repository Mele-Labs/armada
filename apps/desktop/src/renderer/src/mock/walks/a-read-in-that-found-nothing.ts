// A read-in whose scout answers with nothing to place lands one Note off its Finding saying so.
// The owner, 2 Oct 2026, in place of a toast.
// `studio-read-nothing.test.tsx` holds the claim.

import { NOTHING_FOUND } from "../studio-fleet";
import { button, inside, role, walk } from "../walk";

const picked = role("group", "What is picked");

export const aReadInThatFoundNothing = walk("studio-read-nothing", [
  { press: button("Studios", { exact: true }), say: "This repository's Studios" },
  { press: button("Open", { exact: true }), say: "A Studio holding one Link, to a glossary page" },
  { press: button("Continue", { exact: true }), say: "Continue it, so it can be read in" },
  { press: role("group", /^Link: /), say: "Pick the Link" },
  { press: inside(picked, button("Read in", { exact: true })), say: "Read it in" },
  { press: inside(role("dialog"), button("Read in", { exact: true })), say: "The scout reads it, in a Zone of its own" },
  { press: button("Fit", { exact: true }), say: "Fit the board to see what came back" },
  { look: role("group", `Note: ${NOTHING_FOUND}`, { exact: true }), say: "It answered and asked for nothing, so a Note off the Finding says so" },
]);
