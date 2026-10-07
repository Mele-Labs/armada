import type { ReactNode } from "react";
import { CircleDashed, CircleDot, Eye, ShieldX, SquareTerminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Card } from "../../primitives/Card/Card";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Session open, in the glass panel a Job's detail and Overview's panels are
 * (`Card` on the canvas). **The state is one registry mark in the header with
 * its tooltip**, not a colour on the frame: blank is a step not yet started,
 * working is a Drone running, waiting is a Job at review, and failing is a
 * pull request's Checks red. `[conventions.session_state_borrowing]` in
 * `packages/icons/icons/` lends them.
 */
export type SessionState = "blank" | "working" | "waiting" | "failing";

const MARK: Record<SessionState, LucideIcon> = {
  blank: CircleDashed,
  working: CircleDot,
  waiting: Eye,
  failing: ShieldX,
};

/** The state's one mark, animated while a turn runs, named by its tooltip. */
export function SessionMark({ state, said }: { state: SessionState; said: string }) {
  const Glyph = MARK[state];
  return (
    <Tooltip label={said}>
      <span className="armada-session-mark" role="img" aria-label={said} data-pulsing={state === "working" || undefined}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

export type SessionFrameProps = {
  state: SessionState;
  /** What the state's mark says on hover. */
  said: string;
  id: string;
  /** What other sessions call it, drawn in place of `id`. */
  address?: string;
  /** Absent until the first turn has named it. */
  title?: string;
  /** Beside the state mark, at the head's trailing edge: the ledger's button where it has folded. */
  actions?: ReactNode;
  children: ReactNode;
};

export function SessionFrame({ state, said, id, address, title, actions, children }: SessionFrameProps) {
  return (
    <Card className="armada-session-frame" role="region" aria-label={`Session ${address ?? id}`}>
      <header className="armada-session-frame__head">
        <SquareTerminal size={16} strokeWidth={2} aria-hidden />
        <span className="armada-session-frame__id">{address ?? id}</span>
        <span className="armada-session-frame__title">{title}</span>
        {actions}
        <SessionMark state={state} said={said} />
      </header>
      <div className="armada-session-frame__body">{children}</div>
    </Card>
  );
}
