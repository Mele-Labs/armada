import { Bot, Server, UserCheck } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Whose way a retro item got in — `Whose` on the wire. `docs/concepts/retro.md`. */
export type Who = "drone" | "owner" | "fleet";

/**
 * Whose way one retro item got in, as a 12px mark named by its tooltip.
 *
 * **A mark rather than a word** (the owner's standing rule, 2 Oct 2026): a
 * bare glyph carries a tooltip naming it and nothing else. Each glyph is a
 * `usage` row in `packages/icons/icons.toml` — `bot` a Drone, `user-check` the
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

const MARKS: Record<Who, { Glyph: typeof Bot; says: string }> = {
  drone: { Glyph: Bot, says: "Drone" },
  owner: { Glyph: UserCheck, says: "You" },
  fleet: { Glyph: Server, says: "Fleet" },
};
