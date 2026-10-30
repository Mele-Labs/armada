// What the deck's panes and panels share: the mark a state wears and the pull request chip. Ported from
// proto/cockpit-flight-deck. #2301.

import { Check, CircleDot, GitMerge, GitPullRequest, ShieldCheck, ShieldX, type LucideIcon } from "lucide-react";
import { Tooltip } from "@armada/components";

import type { Mark, Pr } from "./ship";

/** One state as its registry glyph, named by a tooltip and never by a word. */
export function MarkGlyph({ mark, size = 16 }: { mark: Mark; size?: 12 | 16 | 20 }) {
  const { Glyph } = mark;
  return (
    <Tooltip label={mark.said}>
      <span className="deck-mark" data-tone={mark.tone} data-bucket={mark.bucket} role="img" aria-label={mark.said}>
        <Glyph size={size} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** Where a pull request is, as a glyph and the words its tooltip says. */
export const STAGE: Record<Pr["stage"], { Glyph: LucideIcon; said: string }> = {
  checking: { Glyph: CircleDot, said: "Checks running" },
  red: { Glyph: ShieldX, said: "Checks failed" },
  queued: { Glyph: GitMerge, said: "In the merge queue" },
  ready: { Glyph: ShieldCheck, said: "Checks passed" },
  landed: { Glyph: Check, said: "Merged" },
};

/** A pull request as a chip on the strip or pane that owns it: its number and the stage it is at. */
export function PrChip({ pr, onOpen }: { pr: Pr; onOpen?: () => void }) {
  const { Glyph, said } = STAGE[pr.stage];
  const body = (
    <>
      <GitPullRequest size={12} aria-hidden="true" />
      <span>#{pr.number}</span>
      <Glyph size={12} aria-hidden="true" />
    </>
  );
  return (
    <Tooltip label={`#${pr.number} ${said}`}>
      {onOpen === undefined ? (
        <span className="deck-pr" data-stage={pr.stage} role="img" aria-label={`#${pr.number}, ${said}`}>
          {body}
        </span>
      ) : (
        <button type="button" className="deck-pr" data-stage={pr.stage} aria-label={`#${pr.number}, ${said}`} onClick={onOpen}>
          {body}
        </button>
      )}
    </Tooltip>
  );
}
