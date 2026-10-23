// A Job view's docked place: a block's run and the Job's shell, switched as tabs where both show. On a Job's page the shell opens
// in the side panel or a window of its own instead where `terminal.openJobTerminalIn` says so, read at each open; a block's run
// docks either way. In the Cockpit the shell docks in a panel of its own under the Job, and the view's dock holds a block's runs alone.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { DockedTerminal, XtermScreen, type ShellDockPane, type ShellPaneProps, type TerminalDockRun } from "@armada/components";

import { shellsOf, useJobShell, type JobShell, type JobShells, type ShellPlace } from "./job-shell";
import { saveJobTerminalPlace, useJobTerminalPlace, type JobTerminalPlace } from "./job-terminal-place";
import { closePanel, openInPanel, usePanel } from "./shared-panel/store";

export type JobDock = {
  /** The runs' dock, which brings its run to the front as it starts or is shown again. */
  dock: TerminalDockRun;
  /** Terminal: the shell's pane in front, with a new shell where none is running. Absent where the view serves no terminal. Flight control's tile does the same (`useJobTerminal`). */
  terminal: (() => void) | undefined;
  /** What is docked, or nothing. */
  pane: ReactNode;
};

/** A call card's box keeps its shell docked. */
const placeIn = (place: ShellPlace, setting: JobTerminalPlace): JobTerminalPlace => (place === "page" ? setting : "docked");

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
  const where = placeIn(place, useJobTerminalPlace(shells !== null && place === "page"));
  const placed = useRef(where);
  placed.current = where;
  useShellArrivals(shells, place, sessionId, placed, () => setFront("shell"));
  if (!served) return { dock: run, terminal: undefined, pane: null };
  const dock: TerminalDockRun = {
    ...run,
    start: (command) => {
      setFront("run");
      run.start(command);
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
      openShell(shells, place, sessionId, where);
      setFront("shell");
    },
    pane: <DockedTerminal dock={dock} shell={{ pane, front, onFront: setFront }} />,
  };
}

/**
 * A shell the agent opened shows in the view of its Job, where the setting says, or in the window it already has; a shell moved
 * back out of its window docks, or opens in the side panel. `docked` hears each that docks.
 */
function useShellArrivals(shells: JobShells | null, place: ShellPlace, sessionId: string, placed: { current: JobTerminalPlace }, docked?: () => void): void {
  const told = useRef(docked);
  told.current = docked;
  useEffect(() => {
    if (shells === null) return;
    const offOpened = shells.onOpened((opened) => {
      if (opened !== sessionId) return;
      if (place === "page" && (shells.hasWindow(sessionId) || placed.current === "window") && toWindow(shells, place, sessionId)) return;
      if (placed.current === "side panel" && toPanel(shells, place, sessionId)) return;
      shells.show(place, sessionId, true);
      told.current?.();
    });
    if (place !== "page") return offOpened;
    const offBack = shells.onMovedBack((move) => {
      if (move.sessionId !== sessionId) return;
      // This window may never have followed the shell, which the window opened: main says which it is and how it stands.
      shells.movedIn(sessionId, move.shell);
      if (move.to === "side panel" && openInPanel({ kind: "terminal", sessionId })) {
        shells.show(place, sessionId, false);
        return;
      }
      shells.show(place, sessionId, true);
      told.current?.();
    });
    return () => {
      offOpened();
      offBack();
    };
  }, [shells, place, sessionId, placed]);
}

/** The docked shell's pane: Hide and End shell, New shell and Close, and on a Job's page the moves to the side panel and a window. */
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
              to: "side panel",
              onMove: () => {
                saveJobTerminalPlace("side panel");
                toPanel(shells, place, sessionId);
              },
            },
            {
              to: "window",
              onMove: () => {
                saveJobTerminalPlace("window");
                toWindow(shells, place, sessionId);
              },
            },
          ],
        }
      : {}),
    // Keyed by the shell, so a new one is a new screen that fits and says its size before it opens.
    children: <XtermScreen key={shell.screen} out={shell.out} run={shell.screen} running={running} write={(data) => shells.write(sessionId, data)} onSize={(cols, rows) => shells.sized(sessionId, cols, rows)} />,
  };
}

/**
 * The Job's docked shell as the Cockpit draws it, in a panel of its own between the Job and the merge line, so the thread keeps the
 * Job's whole pane. Null while it is put away. A shell the agent opens shows here, or in the side panel or its window where the setting says so.
 */
export function useDeckShell(sessionId: string, served: boolean): ShellPaneProps | null {
  const shells = served ? shellsOf(window.armada) : null;
  const { shell, shown } = useJobShell(shells, "page", sessionId);
  const placed = useRef<JobTerminalPlace>("docked");
  placed.current = useJobTerminalPlace(shells !== null);
  useShellArrivals(shells, "page", sessionId, placed);
  return shells === null || !shown || shell === undefined ? null : shellPane(shells, "page", sessionId, shell);
}

/** The Job's shell as Flight control's Terminal tile reads and presses it: whether it shows on the Job's page, docked, in the side panel or in its window. */
export type JobTerminal = { shown: boolean; windowed: boolean; press: () => void };

/**
 * The Job page's shell from outside its view. A press opens it as the view's own Terminal did; while it shows, a press puts it away and
 * it keeps running, as a layer tile closes its layer. A window of its own is brought to the front instead: its own close puts it away.
 * Absent where the window serves no shell.
 */
export function useJobTerminal(sessionId: string, served: boolean): JobTerminal | undefined {
  const shells = served ? shellsOf(window.armada) : null;
  const { shown: docked, windowed } = useJobShell(shells, "page", sessionId);
  const open = usePanel().open;
  const inPanel = open?.kind === "terminal" && open.sessionId === sessionId;
  const where = useJobTerminalPlace(shells !== null);
  if (shells === null) return undefined;
  return {
    shown: docked || inPanel || windowed,
    windowed,
    press: () => {
      if (windowed) shells.toWindow(sessionId);
      else if (docked) shells.show("page", sessionId, false);
      else if (inPanel) closePanel();
      else openShell(shells, "page", sessionId, where);
    },
  };
}

/** The pane in front, with a new shell where none is running: in its window or the side panel where the setting says so and either is served. */
function openShell(shells: JobShells, place: ShellPlace, sessionId: string, where: JobTerminalPlace): void {
  if (where === "window" && toWindow(shells, place, sessionId)) return;
  if (where === "side panel" && toPanel(shells, place, sessionId)) return;
  shells.want(place, sessionId);
}

/** The shell in its own window, which opens it there; a pane docked here is put away, so one screen sizes it. */
function toWindow(shells: JobShells, place: ShellPlace, sessionId: string): boolean {
  if (!shells.toWindow(sessionId)) return false;
  shells.show(place, sessionId, false);
  return true;
}

/** The shell in the side panel, with a new one where none is running; a pane docked before the setting changed is put away. */
function toPanel(shells: JobShells, place: ShellPlace, sessionId: string): boolean {
  shells.fresh(sessionId);
  if (!openInPanel({ kind: "terminal", sessionId })) return false;
  shells.show(place, sessionId, false);
  return true;
}
