// A Job view's docked place: a block's run and the Job's shell, switched as tabs where both show.

import { useEffect, useState, type ReactNode } from "react";
import { DockedTerminal, XtermScreen, type ShellDockPane, type ShellPaneProps, type TerminalDockRun } from "@armada/components";

import { shellsOf, useJobShell, type ShellPlace } from "./job-shell";

export type JobDock = {
  /** The runs' dock, which brings its run to the front as it starts or is shown again. */
  dock: TerminalDockRun;
  /** Terminal: the shell's pane in front, with a new shell where none is running. Absent where the view serves no terminal. */
  terminal: (() => void) | undefined;
  /** What is docked, or nothing. */
  pane: ReactNode;
};

/** `served` is whether the view serves terminal runs; a view that does also docks the Job's shell. */
export function useJobDock(sessionId: string, place: ShellPlace, run: TerminalDockRun, served: boolean): JobDock {
  const shells = served ? shellsOf(window.armada) : null;
  const { shell, shown } = useJobShell(shells, place, sessionId);
  const [front, setFront] = useState<ShellDockPane>("shell");
  // A shell the agent opened shows in the view of its Job, in front.
  useEffect(
    () =>
      shells?.onOpened((opened) => {
        if (opened !== sessionId) return;
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
  const running = shell !== undefined && !shell.ended;
  const pane: ShellPaneProps | null =
    !shown || shell === undefined
      ? null
      : {
          phase: running ? { state: "running" } : { state: "ended" },
          onHide: () => shells.show(place, sessionId, false),
          onEnd: () => shells.stop(sessionId),
          onOpen: () => shells.want(place, sessionId),
          onClose: () => shells.show(place, sessionId, false),
          // Keyed by the shell, so a new one is a new screen that fits and says its size before it opens.
          children: <XtermScreen key={shell.screen} out={shell.out} run={shell.screen} running={running} write={(data) => shells.write(sessionId, data)} onSize={(cols, rows) => shells.sized(sessionId, cols, rows)} />,
        };
  return {
    dock,
    terminal: () => {
      shells.want(place, sessionId);
      setFront("shell");
    },
    pane: <DockedTerminal dock={dock} shell={{ pane, front, onFront: setFront }} />,
  };
}
