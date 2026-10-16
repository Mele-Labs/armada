import { useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { CircleCheck, CircleX, RotateCcw, Square, SquareTerminal, X } from "lucide-react";

import { Button } from "../Button/Button";
import { Tooltip } from "../Tooltip/Tooltip";

/** How a terminal run stands. `code` is null where it was stopped. */
export type TerminalPhase = { state: "running" } | { state: "ended"; code: number | null };

const exitSaid = (phase: TerminalPhase) => (phase.state === "running" ? "Running" : phase.code === null ? "Stopped" : `Exit ${phase.code}`);

/** A run's mark: the terminal glyph, animated while it runs, then passed, failed or stopped. Its tooltip says which. */
export function TerminalMark({ phase }: { phase: TerminalPhase }) {
  const said = exitSaid(phase);
  const result = phase.state === "running" ? "running" : phase.code === 0 ? "passed" : phase.code === null ? "stopped" : "failed";
  return (
    <Tooltip label={said}>
      <span className="armada-terminal-mark" data-result={result} role="img" aria-label={result === "running" ? "Running" : result === "passed" ? "Passed" : result === "stopped" ? "Stopped" : "Failed"}>
        {result === "running" ? <SquareTerminal size={16} strokeWidth={2} aria-hidden /> : result === "passed" ? <CircleCheck size={16} strokeWidth={2} aria-hidden /> : <CircleX size={16} strokeWidth={2} aria-hidden />}
      </span>
    </Tooltip>
  );
}

export type TerminalPaneProps = {
  command: string;
  phase: TerminalPhase;
  onStop: () => void;
  onRerun: () => void;
  onClose: () => void;
  /** The body: the screen the run is drawn on. */
  children: ReactNode;
};

/**
 * **A command running in a terminal of its own**, docked in a Session's view. The frame is this and the
 * body is the caller's, the terminal screen: the mark, the command on one line, and the keys that end
 * or repeat the run. Stop while it runs; Run again and Close once it has ended.
 */
export function TerminalPane({ command, phase, onStop, onRerun, onClose, children }: TerminalPaneProps) {
  const running = phase.state === "running";
  return (
    <section className="armada-terminal" role="group" aria-label="Terminal" data-state={phase.state}>
      <header className="armada-terminal__head">
        <TerminalMark phase={phase} />
        <span className="armada-terminal__command">
          <Tooltip label={command}>
            <code className="armada-terminal__text">{command}</code>
          </Tooltip>
        </span>
        <span className="armada-terminal__keys">
          {running ? (
            <Tooltip label="Stop">
              <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={onStop}>
                <Square size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : (
            <>
              <Tooltip label="Run again">
                <Button variant="ghost" size="sm" iconOnly aria-label="Run again" onClick={onRerun}>
                  <RotateCcw size={16} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
              <Tooltip label="Close">
                <Button variant="ghost" size="sm" iconOnly aria-label="Close" onClick={onClose}>
                  <X size={16} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            </>
          )}
        </span>
      </header>
      <div className="armada-terminal__body">{children}</div>
    </section>
  );
}

/** One `--space-4` per arrow press, as a dock's handle moves. */
const HEIGHT_STEP = 16;
/** The least the pane keeps, and the least it leaves the thread above it. */
const HEIGHT_MIN = 96;
const THREAD_MIN = 160;

/**
 * **The pane docked at the foot of a view**, about two fifths of it, with a handle on its top edge. A
 * drag moves the edge; the arrow keys do too, and Home puts it back.
 */
export function TerminalDock(props: TerminalPaneProps) {
  const box = useRef<HTMLDivElement>(null);
  const drag = useRef<{ pointerId: number; startY: number; startHeight: number } | null>(null);
  const [height, setHeight] = useState<number | null>(null);
  const [dragging, setDragging] = useState(false);

  const room = () => {
    const parent = box.current?.parentElement;
    return parent === null || parent === undefined ? 600 : Math.max(HEIGHT_MIN, parent.clientHeight - THREAD_MIN);
  };
  const clamp = (value: number) => Math.min(room(), Math.max(HEIGHT_MIN, value));
  const now = () => box.current?.getBoundingClientRect().height ?? HEIGHT_MIN;

  function pointerDown(event: PointerEvent<HTMLDivElement>): void {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, startY: event.clientY, startHeight: now() };
    setDragging(true);
    event.currentTarget.focus();
    event.preventDefault();
  }
  function pointerMove(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current === null || drag.current.pointerId !== event.pointerId) return;
    setHeight(clamp(drag.current.startHeight + (drag.current.startY - event.clientY)));
  }
  function endDrag(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
  }
  function keyDown(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "ArrowUp") setHeight(clamp(now() + HEIGHT_STEP));
    else if (event.key === "ArrowDown") setHeight(clamp(now() - HEIGHT_STEP));
    else if (event.key === "Home") setHeight(null);
    else return;
    event.preventDefault();
  }

  return (
    <div ref={box} className="armada-terminal-dock" {...(height === null ? {} : { style: { flexBasis: height } })}>
      <div
        className="armada-terminal-dock__handle"
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize terminal"
        aria-valuemin={HEIGHT_MIN}
        {...(height === null ? {} : { "aria-valuenow": Math.round(height) })}
        tabIndex={0}
        data-dragging={dragging || undefined}
        onDoubleClick={() => setHeight(null)}
        onPointerDown={pointerDown}
        onPointerMove={pointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onKeyDown={keyDown}
      />
      <TerminalPane {...props} />
    </div>
  );
}
