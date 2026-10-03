import { useId, useLayoutEffect, useRef, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { DroneMessageBox, type DroneMessageBoxProps } from "../DroneMessageBox/DroneMessageBox";
import { DroneTurns, type DroneTurn } from "../DroneTurns/DroneTurns";
import { DRONE_ACTIVITY, type JobDroneState } from "../JobDrones/JobDrones";
import { StepActivityMark } from "../StepActivityMark/StepActivityMark";

/**
 * Drone peek — one Drone, small: who it is, what it is writing, and a box to
 * tell it something. Its decision: *what does a peek into a Drone show?*
 * (owner, 29 Sep 2026).
 *
 * **A live tail, not a summary.** The body is the Drones sheet's own
 * transcript, `DroneTurns`, whole, in a short scroller that opens at the tail
 * and follows it while the Drone writes. Open goes to the whole Drone in the
 * Drones destination.
 *
 * **It names nothing about where it sits.** The words and the press are the
 * caller's; the Plan task panel is its first mount.
 */
export type DronePeekProps = {
  /** Which Drone, as job detail names it — `Drone on T6`. */
  title: ReactNode;
  state: JobDroneState;
  /** The state, spelled — Job drones' own words. */
  stateSays: string;
  /** How long it ran, on Job drones' `Run time` terms. Absent draws nothing. */
  ranFor?: ReactNode;
  /** Its transcript, in order. The peek shows the tail. */
  turns: DroneTurn[];
  /** The Drone is writing, so the tail is followed. */
  live: boolean;
  /** What the transcript says with no rows. Absent draws nothing. */
  emptyNote?: string;
  /** Open the whole Drone. Absent draws no Open. */
  onOpen?: () => void;
  /** The message box at the foot. Absent where the Drone cannot be reached. */
  message?: DroneMessageBoxProps;
};

export function DronePeek({
  title,
  state,
  stateSays,
  ranFor,
  turns,
  live,
  emptyNote,
  onOpen,
  message,
}: DronePeekProps) {
  const named = useId();
  const tail = useRef<HTMLDivElement>(null);
  // **Opens at the tail whatever the state.** `DroneTurns` opens a finished run
  // at its top; a failed Drone's last turns are why anybody opened this one.
  // Once — a live run is then followed by `DroneTurns` itself.
  useLayoutEffect(() => {
    const pane = tail.current;
    if (pane !== null) pane.scrollTop = pane.scrollHeight;
  }, []);
  return (
    <Card className="armada-drone-peek" role="group" aria-labelledby={named}>
      <div className="armada-drone-peek__head">
        <span className="armada-drone-peek__name" id={named}>
          {title}
        </span>
        {/* Job drones' own mark, named on hover, so a state reads the same in
            both — never the word (owner, 2 Oct 2026). */}
        <StepActivityMark
          activity={DRONE_ACTIVITY[state]}
          label={stateSays}
          says={stateSays}
          pulsing={state === "running"}
        />
        {ranFor === undefined ? null : (
          <Tooltip label="Run time">
            <span className="armada-drone-peek__ran">{ranFor}</span>
          </Tooltip>
        )}
        {onOpen === undefined ? null : (
          <Button size="sm" onClick={onOpen}>
            Open
          </Button>
        )}
      </div>
      {/* Focusable, so a keyboard can scroll back up the transcript. */}
      <div ref={tail} className="armada-drone-peek__tail" tabIndex={0} aria-label="Transcript">
        <DroneTurns turns={turns} {...(emptyNote === undefined ? {} : { emptyNote })} live={live} steps={false} />
      </div>
      {message === undefined ? null : <DroneMessageBox {...message} />}
    </Card>
  );
}
