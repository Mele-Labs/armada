import { AppWindow, Briefcase, FileCog } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Where a retro item's fix lands — `LandsIn` on the wire. `docs/concepts/retro.md`. */
export type Lands = "armada" | "kit" | "manifest";

/**
 * Where one retro item's fix lands, as a 12px mark named by its tooltip — the
 * owner, 3 Oct 2026. Drawn beside `WhoMark`, the two read as whose way and
 * where the fix goes. Each glyph is a row or a usage in
 * `packages/icons/icons.toml`: `app-window` Armada, `briefcase` the Kit,
 * `file-cog` the Manifest, all three Proposed here.
 */
export function LandsMark({ lands }: { lands: Lands }) {
  const { Glyph, says } = MARKS[lands];
  return (
    <Tooltip label={says}>
      <span className="armada-lands-mark" role="img" aria-label={says}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The place as a word, for the label a mark sits beside. */
export function landsWord(lands: Lands): string {
  return WORDS[lands];
}

const WORDS: Record<Lands, string> = { armada: "Armada", kit: "Kit", manifest: "Manifest" };

const MARKS: Record<Lands, { Glyph: typeof Briefcase; says: string }> = {
  armada: { Glyph: AppWindow, says: "Lands in Armada" },
  kit: { Glyph: Briefcase, says: "Lands in Kit" },
  manifest: { Glyph: FileCog, says: "Lands in the Manifest" },
};
