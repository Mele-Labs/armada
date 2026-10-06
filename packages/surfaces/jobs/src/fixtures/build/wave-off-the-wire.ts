// The Epic's wave as Fleet serves it since 23.14 (#1692), with no draft in
// between: each pass's line off the parent's `wave_rounds`, and the graph's
// edges off each member's own Board row, `waits_on`.
//
// **The same Jobs as `epic/wave`**, and none of its draft wave: what that
// moment hands the region as props, this one leaves to `waveOf` to read off
// the rows. A Job's brief, its expectations and what it has spent are the
// draft's alone, so its panel here carries what the wire does.

import type { ArcMoment } from "./arc-base";
import { WAVE_ID, waveChildren, waveParent, waveQuestions } from "./waves";

export function waveOffTheWire(): ArcMoment {
  return {
    name: "waveOffTheWire",
    says: "A wave read off the wire — each pass's line, and the order each Job waits in",
    fixtures: [waveParent(), ...waveChildren().map((one) => one.fixture)],
    opens: WAVE_ID,
    questions: waveQuestions(),
    draft: {},
  };
}
