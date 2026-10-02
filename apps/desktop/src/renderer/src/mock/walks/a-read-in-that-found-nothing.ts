// A read-in whose scout answers with nothing to place says so in a toast. The owner, 2 Oct 2026.
// `studio-read-nothing.test.tsx` holds the claim.

import { button, inside, role, text, walk } from "../walk";

const picked = role("group", "What is picked");

export const aReadInThatFoundNothing = walk("studio-read-nothing", [
  { press: button("Studios", { exact: true }), say: "This repository's Studios" },
  { press: button("Open", { exact: true }), say: "A Studio holding one Link, to a glossary page" },
  { press: button("Continue", { exact: true }), say: "Continue it, so it can be read in" },
  { press: role("group", /^Link: /), say: "Pick the Link" },
  { press: inside(picked, button("Read in", { exact: true })), say: "Read it in" },
  { press: inside(role("dialog"), button("Read in", { exact: true })), say: "The scout starts reading, in a Zone of its own" },
  { look: inside(role("status"), text("The read-in found nothing.")), say: "It answered and asked for nothing, so the window says so" },
]);
