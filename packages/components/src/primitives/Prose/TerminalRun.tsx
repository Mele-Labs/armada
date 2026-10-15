import { createContext, useEffect, useRef, useState } from "react";

import { Button } from "../Button/Button";
import { TerminalDock, TerminalMark } from "../TerminalPane/TerminalPane";
import { XtermScreen } from "../TerminalPane/XtermScreen";
import type { CodePhase, CodeRun, TerminalSize } from "./RunnableBlock";

/**
 * Where a run draws as a terminal in place of lines of output, which a Session's view provides where its
 * draft serves terminal runs. The pane is docked in the view and the block holds only its mark.
 */
export type ProseTerminalValue = { dock: TerminalDockRun };

export const ProseTerminal = createContext<ProseTerminalValue | null>(null);

/** The one docked pane of a view: which command it holds, its run, and whether it is showing. */
export type TerminalDockRun = {
  command: string | null;
  run: CodeRun;
  shown: boolean;
  /** Opens the pane on `command`, and starts it once the pane has its size. A run going is stopped first. */
  start: (command: string) => void;
  show: () => void;
  hide: () => void;
  /** The pane fitted its screen: the size a waiting run starts at, else the size a running one is told. */
  sized: (size: TerminalSize) => void;
  /** Whether the pane is open and waiting for its size to start its run. */
  waiting: boolean;
};

/** A docked pane's state, which lives in the view and not in the block that started it. `key` is what the view is of: a change of it puts the pane away. */
export function useTerminalDock(run: CodeRun, key: string): TerminalDockRun {
  const [command, setCommand] = useState<string | null>(null);
  const [shown, setShown] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const kept = useRef(run);
  kept.current = run;
  const size = useRef<TerminalSize | null>(null);
  useEffect(
    () => () => {
      kept.current.stop();
      kept.current.close();
      size.current = null;
      setCommand(null);
      setShown(false);
      setWaiting(false);
    },
    [key],
  );
  return {
    command,
    run,
    shown,
    waiting,
    start: (next) => {
      setCommand(next);
      setShown(true);
      if (size.current === null) setWaiting(true);
      else run.start(next, size.current);
    },
    show: () => setShown(true),
    hide: () => setShown(false),
    sized: (fitted) => {
      size.current = fitted;
      if (waiting && command !== null) {
        setWaiting(false);
        run.start(command, fitted);
      } else if (run.phase.state === "running") run.resize(fitted.cols, fitted.rows);
    },
  };
}

/** The view's docked pane, between its thread and its composer: drawn while it holds a run and has not been put away. */
export function DockedTerminal({ dock }: { dock: TerminalDockRun }) {
  const { command, run, shown, waiting } = dock;
  const phase: CodePhase = waiting ? { state: "running" } : run.phase;
  if (!shown || command === null || phase.state === "idle") return null;
  return (
    <TerminalDock command={command} phase={phase} onStop={run.stop} onRerun={() => dock.start(command)} onClose={dock.hide}>
      <XtermScreen out={run.out} run={run.runs} running={phase.state === "running"} write={run.write} onSize={(cols, rows) => dock.sized({ cols, rows })} />
    </TerminalDock>
  );
}

/** What a block shows of a docked run: its mark, which a press brings the pane back from. */
export function DockedMark({ dock, phase }: { dock: TerminalDockRun; phase: CodePhase }) {
  if (phase.state === "idle") return null;
  return (
    <span className="armada-prose__run-state">
      <Button variant="ghost" size="sm" iconOnly aria-label="Show terminal" onClick={dock.show}>
        <TerminalMark phase={phase} />
      </Button>
    </span>
  );
}
