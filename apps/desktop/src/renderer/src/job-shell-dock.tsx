// A Job view's docked place: a block's run and the Job's shell, switched as tabs where both show. On a Job's page the shell opens
// in the side panel instead where `terminal.openJobTerminalIn` says so, read at each open; a block's run docks either way.
// In the Cockpit the shell docks in a panel of its own under the Job, and the view's dock holds a block's runs alone.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { DockedTerminal, XtermScreen, type ShellDockPane, type ShellPaneProps, type TerminalDockRun } from "@armada/components";

import { shellsOf, useJobShell, type JobShell, type JobShells, type ShellPlace } from "./job-shell";
import { saveJobTerminalPlace, useJobTerminalPlace } from "./job-terminal-place";
import { closePanel, openInPanel, usePanel } from "./shared-panel/store";

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
  // A shell docked from the side panel comes to the front, as every other way of showing it does.
  useEffect(() => {
    if (shown) setFront("shell");
  }, [shown]);
  // A call card's box keeps its shell docked.
  const sidePanel = useJobTerminalPlace(shells !== null && place === "page") === "side panel" && place === "page";
  const inPanel = useRef(sidePanel);
  inPanel.current = sidePanel;
  // A shell the agent opened shows in the view of its Job, in front.
  useEffect(
    () =>
      shells?.onOpened((opened) => {
        if (opened !== sessionId) return;
        if (inPanel.current && toPanel(shells, place, sessionId)) return;
        shells.show(place, sessionId, true);
        setFront("shell");
      }),
    [shells, place, sessionId],
  );
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
      openShell(shells, place, sessionId, sidePanel);
      setFront("shell");
    },
    pane: <DockedTerminal dock={dock} shell={{ pane, front, onFront: setFront }} />,
  };
}

/** The docked shell's pane: Hide and End shell, New shell and Close, and on a Job's page the toggle to the side panel. */
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
          ],
        }
      : {}),
    // Keyed by the shell, so a new one is a new screen that fits and says its size before it opens.
    children: <XtermScreen key={shell.screen} out={shell.out} run={shell.screen} running={running} write={(data) => shells.write(sessionId, data)} onSize={(cols, rows) => shells.sized(sessionId, cols, rows)} />,
  };
}

/**
 * The Job's docked shell as the Cockpit draws it, in a panel of its own between the Job and the merge line, so the thread keeps the
 * Job's whole pane. Null while it is put away. A shell the agent opens shows here, or in the side panel where the setting says so.
 */
export function useDeckShell(sessionId: string, served: boolean): ShellPaneProps | null {
  const shells = served ? shellsOf(window.armada) : null;
  const { shell, shown } = useJobShell(shells, "page", sessionId);
  const inPanel = useRef(false);
  inPanel.current = useJobTerminalPlace(shells !== null) === "side panel";
  useEffect(
    () =>
      shells?.onOpened((opened) => {
        if (opened !== sessionId) return;
        if (inPanel.current && toPanel(shells, "page", sessionId)) return;
        shells.show("page", sessionId, true);
      }),
    [shells, sessionId],
  );
  return shells === null || !shown || shell === undefined ? null : shellPane(shells, "page", sessionId, shell);
}

/** The Job's shell as Flight control's Terminal tile reads and presses it: whether it shows on the Job's page, docked or in the side panel. */
export type JobTerminal = { shown: boolean; press: () => void };

/**
 * The Job page's shell from outside its view. A press opens it as the view's own Terminal did; while it shows, a press puts it away and
 * it keeps running, as a layer tile closes its layer. Absent where the window serves no shell.
 */
export function useJobTerminal(sessionId: string, served: boolean): JobTerminal | undefined {
  const shells = served ? shellsOf(window.armada) : null;
  const { shown: docked } = useJobShell(shells, "page", sessionId);
  const open = usePanel().open;
  const inPanel = open?.kind === "terminal" && open.sessionId === sessionId;
  const sidePanel = useJobTerminalPlace(shells !== null) === "side panel";
  if (shells === null) return undefined;
  return {
    shown: docked || inPanel,
    press: () => {
      if (docked) shells.show("page", sessionId, false);
      else if (inPanel) closePanel();
      else openShell(shells, "page", sessionId, sidePanel);
    },
  };
}

/** The pane in front, with a new shell where none is running: in the side panel where the setting says so and a panel is hosted. */
function openShell(shells: JobShells, place: ShellPlace, sessionId: string, sidePanel: boolean): void {
  if (sidePanel && toPanel(shells, place, sessionId)) return;
  shells.want(place, sessionId);
}

/** The shell in the side panel, with a new one where none is running; a pane docked before the setting changed is put away. */
function toPanel(shells: JobShells, place: ShellPlace, sessionId: string): boolean {
  shells.fresh(sessionId);
  if (!openInPanel({ kind: "terminal", sessionId })) return false;
  shells.show(place, sessionId, false);
  return true;
}
