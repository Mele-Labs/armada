// `feed-more-kinds` with the quiet rows folded: the reads, edits and commands a Session ran between two
// louder events are one row of glyphs, opened for the rows inside. The walk `cockpit-feed-folded` plays it.

import type { Scenario } from "../moment";
import { s243FeedMoreKinds as base } from "./feed-more-kinds";

export const s243FeedMoreKindsFolded: Scenario = {
  ...base,
  name: "feed-more-kinds-folded",
  says: "The Cockpit's timeline with each Session's reads, edits and commands folded into one row of glyphs",
  draft: { ...base.draft, quiet: "folded" },
};
