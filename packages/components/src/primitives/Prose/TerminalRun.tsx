import { createContext, useEffect, useMemo, useRef, useSyncExternalStore } from "react";

import { Button } from "../Button/Button";
import { ShellDock, TerminalDock, TerminalMark, type ShellDockPane, type ShellPaneProps, type TerminalPaneProps } from "../TerminalPane/TerminalPane";
import { XtermScreen } from "../TerminalPane/XtermScreen";
import type { CodePhase, CodeRun, TerminalSize } from "./RunnableBlock";
import { changePane, paneFor, subscribeToPane } from "./runs";

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

/**
 * A docked pane's state, which lives under `key` and not in the view or the block that started it: a view that is left and drawn
 * again finds its pane, with the run going or ended, and only Stop, or a start of another run, ends the run. `run` should be held
 * under the same `key` (`useCodeRun`), or leaving the view stops it as an unkept run is.
 */
export function useTerminalDock(run: CodeRun, key: string): TerminalDockRun {
  const entry = paneFor(key);
  const { command, shown, waiting } = useSyncExternalStore(
    useMemo(() => subscribeToPane(entry), [entry]),
    () => entry.view,
  );
  const size = useRef<TerminalSize | null>(null);
  // A fit belongs to the screen that made it: the next view fits its own.
  useEffect(() => () => void (size.current = null), [key]);
  return {
    command,
    run,
    shown,
    waiting,
    start: (next) => {
      const fitted = size.current;
      changePane(entry, { command: next, shown: true, waiting: fitted === null });
      if (fitted !== null) run.start(next, fitted);
    },
    show: () => changePane(entry, { shown: true }),
    hide: () => changePane(entry, { shown: false }),
    sized: (fitted) => {
      size.current = fitted;
      if (waiting && command !== null) {
        changePane(entry, { waiting: false });
        run.start(command, fitted);
      } else if (run.phase.state === "running") run.resize(fitted.cols, fitted.rows);
    },
  };
}

/** The Job's shell, where the view docks it beside its runs: its pane while showing, and which of the two is in front. */
export type DockedShell = { pane: ShellPaneProps | null; front: ShellDockPane; onFront: (pane: ShellDockPane) => void };

/**
 * The view's docked pane, between its thread and its composer: drawn while it holds a run and has not been put away.
 * With `shell`, the Job's shell shares the place, and with both showing they are tabs.
 */
export function DockedTerminal({ dock, shell }: { dock: TerminalDockRun; shell?: DockedShell }) {
  const { command, run, shown, waiting } = dock;
  const phase: CodePhase = waiting ? { state: "running" } : run.phase;
  const pane: TerminalPaneProps | null =
    !shown || command === null || phase.state === "idle"
      ? null
      : {
          command,
          phase,
          onStop: run.stop,
          onRerun: () => dock.start(command),
          onClose: dock.hide,
          children: <XtermScreen out={run.out} run={run.runs} running={phase.state === "running"} write={run.write} onSize={(cols, rows) => dock.sized({ cols, rows })} />,
        };
  if (shell !== undefined) return <ShellDock shell={shell.pane} run={pane} front={shell.front} onFront={shell.onFront} />;
  return pane === null ? null : <TerminalDock {...pane} />;
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
