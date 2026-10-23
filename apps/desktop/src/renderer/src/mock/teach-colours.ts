// armada-allow-off-contract: a tweak's was and now travel the wire as hex strings, so these are fixture data and not a design value
// docs/concepts/teaching.md
// Mock-only: the two shared colours the mock Fleet's tweaks change, held apart so `teach.ts` is still read for a colour literal.

import type { Tweak } from "../teach";

/** The amber of a call that waits on the person, made louder. It is a shared token, so the panel lists every other place it is drawn. */
export const LOUDER: Tweak = {
  mod: "needs-you-louder",
  colour: "Needs you amber",
  token: "--status-awaiting-review",
  was: "#E5A93D",
  now: "#FFC233",
  places: ["Calls and call cards", "Workflow steps waiting on you", "Stranded bays", "Title bar alerts", "Timeline marks", "Doctor warnings"],
};

/** The grey of a stopped Job, made lighter: seeded as a tweak taught before the walk starts. */
export const LIGHTER: Tweak = {
  mod: "stopped-lighter",
  colour: "Stopped grey",
  token: "--status-killed",
  was: "#9BA3AC",
  now: "#B4BBC3",
  places: ["Stopped Job rows", "Timeline marks"],
};
