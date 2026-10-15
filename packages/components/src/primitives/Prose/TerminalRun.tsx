import { createContext, useEffect, useRef, useState } from "react";

import { Button } from "../Button/Button";
import { TerminalDock, TerminalMark, TerminalPane, type TerminalScreen } from "../TerminalPane/TerminalPane";
import type { CodePhase, CodeRun } from "./RunnableBlock";

/**
 * Where a run draws as a terminal in place of lines of output, which a Session's view provides where its
 * draft serves a screen. `inline` opens the pane under the block that was run, and `onClosed` hears it
 * put away. `docked` holds the pane in the view and the block only its mark.
 */
export type ProseTerminalValue =
  | { place: "inline"; Screen: TerminalScreen; onClosed?: () => void }
  | { place: "docked"; Screen: TerminalScreen; dock: TerminalDockRun };

export const ProseTerminal = createContext<ProseTerminalValue | null>(null);

/** The one docked pane of a view: which command it holds, its run, and whether it is showing. */
export type TerminalDockRun = {
  command: string | null;
  run: CodeRun;
  shown: boolean;
  start: (command: string) => void;
  show: () => void;
  hide: () => void;
};

/** A docked pane's state, which lives in the view and not in the block that started it. `key` is what the view is of: a change of it puts the pane away. */
export function useTerminalDock(run: CodeRun, key: string): TerminalDockRun {
  const [command, setCommand] = useState<string | null>(null);
  const [shown, setShown] = useState(false);
  const kept = useRef(run);
  kept.current = run;
  useEffect(
    () => () => {
      kept.current.stop();
      kept.current.close();
      setCommand(null);
      setShown(false);
    },
    [key],
  );
  return {
    command,
    run,
    shown,
    start: (next) => {
      setCommand(next);
      setShown(true);
      run.start(next);
    },
    show: () => setShown(true),
    hide: () => setShown(false),
  };
}

/** The pane under a block that was run. */
export function InlineTerminal({ run, command, terminal }: { run: CodeRun; command: string; terminal: Extract<ProseTerminalValue, { place: "inline" }> }) {
  const { phase } = run;
  if (phase.state === "idle") return null;
  const { Screen } = terminal;
  return (
    <TerminalPane
      command={command}
      phase={phase}
      onStop={run.stop}
      onRerun={() => run.start(command)}
      onClose={() => {
        run.close();
        terminal.onClosed?.();
      }}
    >
      <Screen out={run.out} running={phase.state === "running"} write={run.write} />
    </TerminalPane>
  );
}

/** The view's docked pane, between its thread and its composer: drawn while it holds a run and has not been put away. */
export function DockedTerminal({ dock, Screen }: { dock: TerminalDockRun; Screen: TerminalScreen }) {
  const { command, run, shown } = dock;
  const { phase } = run;
  if (!shown || command === null || phase.state === "idle") return null;
  return (
    <TerminalDock command={command} phase={phase} onStop={run.stop} onRerun={() => dock.start(command)} onClose={dock.hide}>
      <Screen out={run.out} running={phase.state === "running"} write={run.write} />
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
