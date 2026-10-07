import type { ReactNode } from "react";
import { CircleDashed, CircleDot, Eye, ShieldX, SquareTerminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Session, framed as a bay is: a band filled in the state's hue over a body,
 * and the same hue on the frame, so the state is in the frame and not in a
 * sentence beside it. The hues are the Job state machine's and nothing new:
 * **blank** is not-started, **working** is running, **waiting** on you is
 * awaiting-review, and **failing** is a pull request whose Checks are red.
 */
export type SessionHue = "not-started" | "running" | "awaiting-review" | "completed-failed";

export type SessionState = "blank" | "working" | "waiting" | "failing";

const HUE: Record<SessionState, SessionHue> = {
  blank: "not-started",
  working: "running",
  waiting: "awaiting-review",
  failing: "completed-failed",
};

const MARK: Record<SessionState, LucideIcon> = {
  blank: CircleDashed,
  working: CircleDot,
  waiting: Eye,
  failing: ShieldX,
};

export const hueOf = (state: SessionState): SessionHue => HUE[state];

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
  /** Absent until the first turn has named it. */
  title?: string;
  children: ReactNode;
};

export function SessionFrame({ state, said, id, title, children }: SessionFrameProps) {
  return (
    <section className="armada-session-frame" data-hue={HUE[state]} aria-label={`Session ${id}`}>
      <header className="armada-session-frame__band">
        <SquareTerminal size={16} strokeWidth={2} aria-hidden />
        <span className="armada-session-frame__eyebrow">Session</span>
        <span className="armada-session-frame__id">{id}</span>
        <span className="armada-session-frame__title">{title}</span>
        <SessionMark state={state} said={said} />
      </header>
      <div className="armada-session-frame__body">{children}</div>
    </section>
  );
}
