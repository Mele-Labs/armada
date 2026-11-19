import { Tooltip } from "../Tooltip/Tooltip";
import { SquareTerminal, CircleCheck, CircleX, Square, RotateCcw, X } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "../Button/Button";

/** How a terminal run stands. A null `code` is a run Stop ended (`stopped` true) or a signal did (`stopped` false); left out, the reason is not known and reads as a stop. */
export type TerminalPhase =
  | {
      state: "running";
    }
  | {
      state: "ended";
      code: number | null;
      stopped?: boolean;
    };

/** What an end says, for a tooltip: an exit code, a stop, or a signal. */
export function endedSaid(end: { code: number | null; stopped?: boolean }): string {
  if (end.code !== null) return `Exit ${end.code}`;
  return end.stopped === false ? "Ended by a signal" : "Stopped";
}

/** A run's mark: the terminal glyph, animated while it runs, then passed, failed or stopped. Its tooltip says which. */
export function TerminalMark({ phase }: { phase: TerminalPhase }) {
  const said = phase.state === "running" ? "Running" : endedSaid(phase);
  const result =
    phase.state === "running"
      ? "running"
      : phase.code === 0
        ? "passed"
        : phase.code === null && phase.stopped !== false
          ? "stopped"
          : "failed";
  return (
    <Tooltip label={said}>
      <span
        className="armada-terminal-mark"
        data-result={result}
        role="img"
        aria-label={
          result === "running"
            ? "Running"
            : result === "passed"
              ? "Passed"
              : result === "stopped"
                ? "Stopped"
                : "Failed"
        }
      >
        {result === "running" ? (
          <SquareTerminal size={16} strokeWidth={2} aria-hidden />
        ) : result === "passed" ? (
          <CircleCheck size={16} strokeWidth={2} aria-hidden />
        ) : (
          <CircleX size={16} strokeWidth={2} aria-hidden />
        )}
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

/** What a pane's header holds: what leads it, what follows on the line, and its keys. */
type PaneHead = {
  lead: ReactNode;
  line: ReactNode;
  keys: ReactNode;
};

/**
 * The frame every docked pane is drawn in. One component for the run and the shell, so the dock's tabs
 * stay mounted, and keep the focus, as the pane in front changes.
 */
function PaneFrame({
  state,
  head,
  tabs,
  placed,
  children,
}: {
  state: string;
  head: PaneHead;
  tabs?: ReactNode;
  placed?: "panel" | "window";
  children: ReactNode;
}) {
  return (
    <section
      className="armada-terminal"
      role="group"
      aria-label="Terminal"
      data-state={state}
      data-placed={placed}
    >
      <header className="armada-terminal__head">
        {tabs === undefined ? head.lead : <span className="armada-terminal__tabs">{tabs}</span>}
        <span className="armada-terminal__command">{head.line}</span>
        <span className="armada-terminal__keys">{head.keys}</span>
      </header>
      <div className="armada-terminal__body">{children}</div>
    </section>
  );
}

function runHead(
  { command, phase, onStop, onRerun, onClose }: Omit<TerminalPaneProps, "children">,
  tabbed: boolean,
): PaneHead {
  const mark = <TerminalMark phase={phase} />;
  return {
    lead: mark,
    // Beside the shell the command is its tab, and the run's mark follows the tabs.
    line: tabbed ? (
      mark
    ) : (
      <Tooltip label={command}>
        <code className="armada-terminal__text">{command}</code>
      </Tooltip>
    ),
    keys:
      phase.state === "running" ? (
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
      ),
  };
}

/**
 * **A command running in a terminal of its own**, docked in a Session's view. The frame is this and the
 * body is the caller's, the terminal screen: the mark, the command on one line, and the keys that end
 * or repeat the run. Stop while it runs; Run again and Close once it has ended.
 */
export function TerminalPane({ children, ...run }: TerminalPaneProps) {
  return (
    <PaneFrame state={run.phase.state} head={runHead(run, false)}>
      {children}
    </PaneFrame>
  );
}
