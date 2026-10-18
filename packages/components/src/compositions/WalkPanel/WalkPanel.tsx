import type { ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";

/**
 * The panel beside a walk window's page: the Session that showed the page —
 * `docs/practices/capture-window.md`, *The panel beside the page*. The owner, 11 Oct 2026:
 * going back and forth between the Session and the walk was painful, so what answers it sits beside it.
 *
 * **It names what it holds before its title.** What it holds is the caller's: a Session's thread and
 * composer, drawn by the components Bridge's own window draws them with.
 */
export type WalkPanelProps = {
  /** The Session's title. */
  title: string;
  /** Where it stands, drawn after the title. */
  status?: ReactNode;
  /** Bring Bridge's window forward on this Session, for what the panel does not draw. */
  onOpenInBridge: () => void;
  children: ReactNode;
};

export function WalkPanel({ title, status, onOpenInBridge, children }: WalkPanelProps) {
  return (
    <section className="armada-walk-panel" aria-label={`Job: ${title}`}>
      <header className="armada-walk-panel__head">
        <span className="armada-walk-panel__kind">Job</span>
        <h2 className="armada-walk-panel__title" title={title}>
          {title}
        </h2>
        {status}
        <Button variant="ghost" size="sm" onClick={onOpenInBridge}>
          Open in Bridge
        </Button>
      </header>
      <div className="armada-walk-panel__body">{children}</div>
    </section>
  );
}
