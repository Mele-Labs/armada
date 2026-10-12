import type { ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";

/**
 * The panel beside a walk window's page: the Session that showed the page, or the Job whose server it
 * walks — `docs/practices/capture-window.md`, *The panel beside the page*. The owner, 11 Oct 2026:
 * going back and forth between the Session and the walk was painful, so what answers it sits beside it.
 *
 * **It names its kind before its title.** A Session and a Job are not the same thing to answer, and the
 * head is the one place that says which this is. What it holds is the caller's: a Session's thread and
 * composer, or a Job's decision, drawn by the components Bridge's own window draws them with.
 */
export type WalkPanelProps = {
  kind: "session" | "job";
  /** The Session's title, or the Job's handle. */
  title: string;
  /** Where it stands, drawn after the title: a Job's state as its badge. */
  status?: ReactNode;
  /** Bring Bridge's window forward on this Session or Job, for what the panel does not draw. */
  onOpenInBridge: () => void;
  children: ReactNode;
};

export function WalkPanel({ kind, title, status, onOpenInBridge, children }: WalkPanelProps) {
  const named = kind === "session" ? "Session" : "Job";
  return (
    <section className="armada-walk-panel" aria-label={`${named}: ${title}`}>
      <header className="armada-walk-panel__head">
        <span className="armada-walk-panel__kind">{named}</span>
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

/** One part of what the panel holds, under its own name: a Job's brief, or its decision. */
export function WalkPanelSection({ head, children }: { head: string; children: ReactNode }) {
  return (
    <section className="armada-walk-panel__section" aria-label={head}>
      <h3 className="armada-walk-panel__section-head">{head}</h3>
      {children}
    </section>
  );
}
