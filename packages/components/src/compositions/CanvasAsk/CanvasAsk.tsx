import type { ReactNode } from "react";
import { Card } from "../../primitives/Card/Card";

/**
 * A card hung beside a canvas node to ask a person something about it, in
 * place. Its decision: *where does a held Drone ask when the node it sits under
 * is on a canvas?* (owner, 5 Oct 2026) — on a card joined to the node by an
 * edge, off to its right, with the answers on it.
 *
 * **It names nothing about what is asked.** The head's words and the box are
 * the caller's, so the canvas draws the same prompt the panels do. `nodrag
 * nopan nowheel` keep a press, a drag or a scroll inside it the card's own.
 */
export type CanvasAskProps = {
  /** What is asked, in a line. */
  title: ReactNode;
  /** The prompt itself. */
  children: ReactNode;
};

export function CanvasAsk({ title, children }: CanvasAskProps) {
  return (
    <Card className="armada-canvas-ask nodrag nopan nowheel" role="group" aria-label="Needs you">
      <div className="armada-canvas-ask__title">{title}</div>
      {children}
    </Card>
  );
}
