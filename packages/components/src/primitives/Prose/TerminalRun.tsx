import { createContext, useCallback, useEffect, useMemo, useRef, useSyncExternalStore } from "react";

import { Button } from "../Button/Button";
import { ShellDock, TerminalDock, TerminalMark, TerminalPane, type ShellDockPane, type ShellPaneProps, type TerminalPaneProps } from "../TerminalPane/TerminalPane";
import { XtermScreen } from "../TerminalPane/XtermScreen";
import type { CodePhase, CodeRun, TerminalSize } from "./RunnableBlock";
import { changePane, paneFor, subscribeToPane } from "./runs";

/**
 * Where a run draws as a terminal in place of lines of output, which a Session's view provides where its
 * draft serves terminal runs. A block's run is drawn under the block (`InlineTerminal`), and a run started
 * from the box is docked in the view.
 */
export type ProseTerminalValue = { dock: TerminalDockRun };

export const ProseTerminal = createContext<ProseTerminalValue | null>(null);

/** The one docked pane of a view: which command it holds, its run, and whether it is showing. */
export type TerminalDockRun = {
  command: string | null;
  run: CodeRun;
  shown: boolean;
  /** The block the pane is drawn under, in place, or null where it is docked in the view. */
  inline: string | null;
  /** Opens the pane on `command`, and starts it once the pane has its size. A run going is stopped first. `inline` is the block that asked, which draws the pane under itself; a run started again keeps the place it had. */
  start: (command: string, inline?: string) => void;
  /** Lets the dock know a block is drawn, and of what command; the answer takes it back. */
  register: (block: string, command: string) => () => void;
  /** Whether the pane is drawn under this block: the one that started it, or, where that is not drawn now (the view was left and came back), the first of its command. */
  claims: (block: string) => boolean;
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
  const { command, shown, waiting, inline } = useSyncExternalStore(
    useMemo(() => subscribeToPane(entry), [entry]),
    () => entry.view,
  );
  // Held, since a block registers again whenever this changes.
  const register = useCallback(
    (block: string, of: string) => {
      entry.blocks.set(block, of);
      changePane(entry, {});
      return () => {
        entry.blocks.delete(block);
        changePane(entry, {});
      };
    },
    [entry],
  );
  const size = useRef<TerminalSize | null>(null);
  // A fit belongs to the screen that made it: the next view fits its own.
  useEffect(() => () => void (size.current = null), [key]);
  return {
    command,
    run,
    shown,
    waiting,
    inline,
    start: (next, placed) => {
      const fitted = size.current;
      changePane(entry, { command: next, shown: true, waiting: fitted === null, inline: placed ?? null });
      if (fitted !== null) run.start(next, fitted);
    },
    register,
    claims: (block) => {
      if (inline === null || command === null) return false;
      if (entry.blocks.get(inline) === command) return inline === block;
      return [...entry.blocks].find(([, of]) => of === command)?.[0] === block;
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
  const pane = dock.inline !== null ? null : paneOf(dock);
  if (shell !== undefined) return <ShellDock shell={shell.pane} run={pane} front={shell.front} onFront={shell.onFront} />;
  return pane === null ? null : <TerminalDock {...pane} />;
}

/** The rows an in-place run has, which is as tall as its screen grows. */
const IN_PLACE_ROWS = 24;

/** The pane the dock holds, while it is open; `grows` makes its screen as tall as the run has printed. */
function paneOf(dock: TerminalDockRun, grows?: number): TerminalPaneProps | null {
  const { command, run, shown, waiting } = dock;
  const phase: CodePhase = waiting ? { state: "running" } : run.phase;
  if (!shown || command === null || phase.state === "idle") return null;
  return {
    command,
    phase,
    onStop: run.stop,
    onRerun: () => dock.start(command, dock.inline ?? undefined),
    onClose: dock.hide,
    children: screenOf(run, phase, (cols, rows) => dock.sized({ cols, rows }), grows),
  };
}

function screenOf(run: CodeRun, phase: CodePhase, onSize: (cols: number, rows: number) => void, grows?: number) {
  return <XtermScreen out={run.out} run={run.runs} running={phase.state === "running"} write={run.write} onSize={onSize} {...(grows === undefined ? {} : { grows })} />;
}

/** A block's run, drawn right under the block, as tall as what it has printed up to 24 rows and then scrolling. */
export function InlineTerminal({ dock }: { dock: TerminalDockRun }) {
  const pane = dock.inline !== null ? paneOf(dock, IN_PLACE_ROWS) : null;
  return pane === null ? null : <TerminalPane {...pane} />;
}

/**
 * A run held by what drew the block, such as a Needs you card, drawn under the block as `InlineTerminal` draws a docked one.
 * The holder starts it with no screen yet, so it starts at the terminal's own size and is told the screen's once it fits.
 */
export function HeldTerminal({ command, run }: { command: string; run: CodeRun }) {
  const { phase } = run;
  if (phase.state === "idle") return null;
  return (
    <TerminalPane command={command} phase={phase} onStop={run.stop} onRerun={() => run.start(command)} onClose={run.close}>
      {screenOf(run, phase, run.resize, IN_PLACE_ROWS)}
    </TerminalPane>
  );
}

/** What a block shows of its run while the pane is put away: its mark, which a press brings the pane back from. */
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
