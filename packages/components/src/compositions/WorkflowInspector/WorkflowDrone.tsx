import type { ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { DroneTurns, type DroneTurn } from "../DroneTurns/DroneTurns";

/**
 * One Drone, read beside the step panel that listed it — the owner's
 * experiment, 29 Sep 2026: *opening the drone panel next to the workflow panel
 * like we did with the file diff*. The Drones tab's reading, in the step
 * panel's frame: its head holds still, its turns scroll, and what a person
 * may do sits at the foot.
 */
export type WorkflowDroneProps = {
  title: string;
  /** Where it is and what it has spent, on one line. */
  subtitle?: ReactNode;
  turns: DroneTurn[];
  /** Why there are no turns, where there are none. */
  emptyNote: string;
  /** It is still writing, so the newest turn is kept in view. */
  live: boolean;
  /** Beside the close: the kill, while it runs. */
  controls?: ReactNode;
  /** Under the turns: the message box, while it runs. */
  footer?: ReactNode;
  onClose: () => void;
};

export function WorkflowDrone({ title, subtitle, turns, emptyNote, live, controls, footer, onClose }: WorkflowDroneProps) {
  return (
    <div className="armada-wf-inspector armada-wf-drone armada-glass" aria-label={title} role="region">
      <header className="armada-wf-inspector__head">
        <div className="armada-wf-inspector__titles">
          <h3 className="armada-wf-inspector__name">{title}</h3>
          {subtitle === undefined ? null : <p className="armada-wf-drone__where">{subtitle}</p>}
        </div>
        <div className="armada-wf-drone__acts">
          {controls}
          <Button variant="secondary" size="sm" ground="card" onClick={onClose}>
            Close
          </Button>
        </div>
      </header>
      <div className="armada-wf-inspector__body">
        <DroneTurns key={title} turns={turns} emptyNote={emptyNote} live={live} steps={false} />
      </div>
      {footer === undefined ? null : <div className="armada-wf-drone__foot">{footer}</div>}
    </div>
  );
}
