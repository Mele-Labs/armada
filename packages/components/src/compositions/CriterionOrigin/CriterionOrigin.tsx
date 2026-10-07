import { Diff, Quote, Ticket, UserPen, type LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Where a criterion's words came from, as a mark — the owner's picks of 3 Oct
 * 2026, group `Criterion origin` in `packages/icons/icons/`.
 *
 * **A state is never text, and a bare mark names itself on hover** (the
 * owner's two standing rules). The words slice 4 drew here — *From issue*,
 * *From your prompt*, *You wrote this* — are each mark's tooltip and
 * accessible name instead.
 *
 * **Not the verification source and not the actor.** Who will decide the line
 * stays words beside it: `docs/contracts/iconography.md`, Where the answer is
 * no icon.
 */
export type CriterionOriginKind = "issue" | "prompt" | "person";

const GLYPH: Record<CriterionOriginKind, LucideIcon> = {
  issue: Ticket,
  prompt: Quote,
  person: UserPen,
};

/** What each mark is called, on hover and to a screen reader. */
export const ORIGIN_SAID: Record<CriterionOriginKind, string> = {
  issue: "From an issue",
  prompt: "From your request",
  person: "Written or reworded by a person",
};

/** 12px at strokeWidth 2, the badge geometry. */
const MARK = 12;
const STROKE = 2;

export function CriterionOriginMark({ origin }: { origin: CriterionOriginKind }) {
  const Glyph = GLYPH[origin];
  const said = ORIGIN_SAID[origin];
  return (
    <Tooltip asChild label={said}>
      <span className="armada-criterion-mark" data-origin={origin} role="img" aria-label={said}>
        <Glyph size={MARK} strokeWidth={STROKE} aria-hidden />
      </span>
    </Tooltip>
  );
}

/**
 * The issue these words came from has been edited since Fleet read it.
 *
 * **Since Fleet read it, never since the words froze.** Slice 4 said
 * *frozen*, which was wrong at the gate: a proposal nobody has approved holds
 * nothing frozen. The Job keeps the words either way (#1530).
 */
export function CriterionMovedMark({ at }: { at: string }) {
  const said = `The issue has been edited since Fleet read it, last on ${at}`;
  return (
    <Tooltip asChild label={said}>
      <span className="armada-criterion-mark" data-moved="true" role="img" aria-label={said}>
        <Diff size={MARK} strokeWidth={STROKE} aria-hidden />
      </span>
    </Tooltip>
  );
}
