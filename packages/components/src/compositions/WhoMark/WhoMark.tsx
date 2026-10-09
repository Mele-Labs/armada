import { Bot, Server, UserCheck } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Whose way a retro item got in — `Whose` on the wire. `docs/concepts/retro.md`. */
export type Who = "drone" | "agent" | "owner" | "fleet";

/**
 * Whose way one retro item got in, as a 12px mark named by its tooltip.
 *
 * **A mark rather than a word** (the owner's standing rule, 2 Oct 2026): a
 * bare glyph carries a tooltip naming it and nothing else. Each glyph is a
 * `usage` row in `packages/icons/icons/` — `bot` a Drone, `user-check` the
 * person, `server` Fleet, the last Proposed.
 */
export function WhoMark({ who }: { who: Who }) {
  const { Glyph, says } = MARKS[who];
  // Named as an image, the merge line's marks' way, so a reader without a
  // pointer hears whose way it was; the tooltip is what a pointer reads.
  return (
    <Tooltip label={says}>
      <span className="armada-who-mark" role="img" aria-label={says}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** Whose way, as a word, for the label a mark sits beside. */
export function whoWord(who: Who): string {
  return MARKS[who].says;
}

const MARKS: Record<Who, { Glyph: typeof Bot; says: string }> = {
  drone: { Glyph: Bot, says: "Drone" },
  agent: { Glyph: Bot, says: "Agent" },
  owner: { Glyph: UserCheck, says: "You" },
  fleet: { Glyph: Server, says: "Fleet" },
};
