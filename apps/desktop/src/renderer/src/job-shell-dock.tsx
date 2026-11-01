// A Job view's docked place: a block's run and the Job's shell, switched as tabs where both show. The shell can leave for a window of
// its own (Move to window) and docks again when that window closes. In the Cockpit the shell is a Terminal tab (`TerminalBody`), and the
// view's dock holds a block's runs alone.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { DockedTerminal, XtermScreen, type ShellDockPane, type ShellPaneProps, type TerminalDockRun } from "@armada/components";

import { shellsOf, useJobShell, type JobShell, type JobShells, type ShellPlace } from "./job-shell";
import { viewingNow } from "./cockpit/viewed";

export type JobDock = {
  /** The runs' dock, which brings its run to the front as it starts or is shown again. */
  dock: TerminalDockRun;
  /** Terminal: the shell's pane in front, with a new shell where none is running. Absent where the view serves no terminal. Flight control's tile does the same (`useJobTerminal`). */
  terminal: (() => void) | undefined;
  /** What is docked, or nothing. */
  pane: ReactNode;
};

/**
 * `served` is whether the view serves terminal runs; a view that does also docks the Job's shell, unless `shellElsewhere`: the
 * Cockpit docks it in a panel of its own under the Job (`useDeckShell`), and the view's dock holds a block's runs alone.
 */
export function useJobDock(sessionId: string, place: ShellPlace, run: TerminalDockRun, served: boolean, shellElsewhere = false): JobDock {
  const shells = served && !shellElsewhere ? shellsOf(window.armada) : null;
  const { shell, shown } = useJobShell(shells, place, sessionId);
  const [front, setFront] = useState<ShellDockPane>("shell");
  // A shell docked from the side panel or its window comes to the front, as every other way of showing it does.
  useEffect(() => {
    if (shown) setFront("shell");
  }, [shown]);
  useShellArrivals(shells, place, sessionId, () => setFront("shell"));
  if (!served) return { dock: run, terminal: undefined, pane: null };
  const dock: TerminalDockRun = {
    ...run,
    start: (command, inline) => {
      setFront("run");
      run.start(command, inline);
    },
    show: () => {
      setFront("run");
      run.show();
    },
  };
  if (shells === null) return { dock, terminal: undefined, pane: <DockedTerminal dock={dock} /> };
  const pane = !shown || shell === undefined ? null : shellPane(shells, place, sessionId, shell);
  return {
    dock,
    terminal: () => {
      shells.want(place, sessionId);
      setFront("shell");
    },
    pane: <DockedTerminal dock={dock} shell={{ pane, front, onFront: setFront }} />,
  };
}

/**
 * A shell the agent opened shows in the view of its Job, or in the window it already has; a shell moved back out of its window docks.
 * `docked` hears each that docks.
 */
export function useShellArrivals(shells: JobShells | null, place: ShellPlace, sessionId: string, docked?: () => void): void {
  const told = useRef(docked);
  told.current = docked;
  useEffect(() => {
    if (shells === null) return;
    const offOpened = shells.onOpened((opened) => {
      // A shell the agent opens while the Job is not on screen waits behind its Terminal key, which pulses while it prints.
      if (opened !== sessionId || !viewingNow(sessionId)) return;
      if (place === "page" && shells.hasWindow(sessionId) && toWindow(shells, place, sessionId)) return;
      shells.show(place, sessionId, true);
      told.current?.();
    });
    if (place !== "page") return offOpened;
    const offBack = shells.onMovedBack((move) => {
      if (move.sessionId !== sessionId) return;
      // This window may never have followed the shell, which the window opened: main says which it is and how it stands.
      shells.movedIn(sessionId, move.shell);
      shells.show(place, sessionId, true);
      told.current?.();
    });
    return () => {
      offOpened();
      offBack();
    };
  }, [shells, place, sessionId]);
}

/** The docked shell's pane: Hide and End shell, New shell and Close, and on a Job's page the move to a window. */
function shellPane(shells: JobShells, place: ShellPlace, sessionId: string, shell: JobShell): ShellPaneProps {
  const running = !shell.ended;
  return {
    phase: running ? { state: "running" } : { state: "ended" },
    onHide: () => shells.show(place, sessionId, false),
    onEnd: () => shells.stop(sessionId),
    onOpen: () => shells.want(place, sessionId),
    onClose: () => shells.show(place, sessionId, false),
    // A call card's box keeps its shell docked.
    ...(place === "page"
      ? {
          moves: [
            {
              to: "window",
              onMove: () => void toWindow(shells, place, sessionId),
            },
          ],
        }
      : {}),
    // Keyed by the shell, so a new one is a new screen that fits and says its size before it opens.
    children: <XtermScreen key={shell.screen} out={shell.out} run={shell.screen} running={running} write={(data) => shells.write(sessionId, data)} onSize={(cols, rows) => shells.sized(sessionId, cols, rows)} />,
  };
}

/**
 * The Job's docked shell as the Terminal tab draws it, in a pane of its own. Null while it is put away. A shell the agent opens is
 * heard by `TerminalSync`, which is drawn whether or not a Terminal tab is.
 */
export function useDeckShell(sessionId: string, served: boolean): ShellPaneProps | null {
  const shells = served ? shellsOf(window.armada) : null;
  const { shell, shown } = useJobShell(shells, "page", sessionId);
  return shells === null || !shown || shell === undefined ? null : shellPane(shells, "page", sessionId, shell);
}

/** The Job's shell as Flight control's Terminal tile reads and presses it, where the Job has no panes: whether it shows docked or in its window. */
export type JobTerminal = { shown: boolean; windowed: boolean; /** The shell printed a moment ago, so something is running in it. */ running: boolean; press: () => void };

/** How long after its last output a shell still counts as running something. */
const IDLE_MS = 3000;

/** Whether the shell printed within `IDLE_MS`, drawn again as it goes quiet. */
function useShellRunning(shell: JobShell | undefined): boolean {
  const printedAt = shell?.ended === false ? shell.printedAt : undefined;
  const [, tick] = useState(0);
  useEffect(() => {
    if (printedAt === undefined) return;
    const wait = printedAt + IDLE_MS - Date.now();
    if (wait <= 0) return;
    const timer = setTimeout(() => tick((n) => n + 1), wait);
    return () => clearTimeout(timer);
  }, [printedAt]);
  return printedAt !== undefined && Date.now() - printedAt < IDLE_MS;
}

/**
 * The Job's shell from outside its view, for a Job with no Terminal tab. A press opens it, with a new shell where none is running; while it shows, a press puts it away and it keeps running.
 * A window of its own is brought to the front instead: its own close puts it away. Absent where the window serves no shell.
 */
export function useJobTerminal(sessionId: string, served: boolean): JobTerminal | undefined {
  const shells = served ? shellsOf(window.armada) : null;
  const { shell, shown: docked, windowed } = useJobShell(shells, "page", sessionId);
  const running = useShellRunning(shell);
  if (shells === null) return undefined;
  return {
    shown: docked || windowed,
    windowed,
    running,
    press: () => {
      if (windowed) shells.toWindow(sessionId);
      else if (docked) shells.show("page", sessionId, false);
      else shells.want("page", sessionId);
    },
  };
}

/** The shell in its own window, which opens it there; a pane docked here is put away, so one screen sizes it. */
function toWindow(shells: JobShells, place: ShellPlace, sessionId: string): boolean {
  if (!shells.toWindow(sessionId)) return false;
  shells.show(place, sessionId, false);
  return true;
}
