// A Job's shell in a window of its own (`docs/concepts/session/shared-terminal.md`): the whole document is the pane. The window's
// own close puts the shell away; its head moves it back into Bridge, docked or in the side panel, through main.

import { useEffect, useRef, useState } from "react";
import { ShellWindowPane, XtermScreen } from "@armada/components";
import type { SessionsRead } from "../../../shared/api/sessions";

import { shellsOf, useShellOf } from "../job-shell";
import { saveJobTerminalPlace } from "../job-terminal-place";
import { stateOf, type StateSource } from "../state-feed";

type WindowApi = Parameters<typeof shellsOf>[0] & Parameters<typeof stateOf>[0];

/** What the window's title says: the Job's title where the Sessions list has one. */
export const windowTitleOf = (title: string | undefined): string => (title === undefined || title === "" ? "Terminal" : `Terminal: ${title}`);

const titleIn = (sessions: SessionsRead, sessionId: string): string | undefined =>
  sessions.state === "read" ? sessions.sessions.find((one) => one.id === sessionId)?.title : undefined;

/** The Job's title from the Sessions list, followed as it is renamed. */
function useJobTitle(source: StateSource, sessionId: string): string | undefined {
  const [title, setTitle] = useState<string>();
  useEffect(() => {
    let live = true;
    void source.state().then((state) => live && setTitle(titleIn(state.sessions, sessionId)));
    const off = source.subscribe((state) => setTitle(titleIn(state.sessions, sessionId)));
    return () => {
      live = false;
      off();
    };
  }, [source, sessionId]);
  return title;
}

/** The window's document: the Job's shell, opened or handed over as the pane fits, and its moves back into Bridge. */
export function TerminalWindow({ api, sessionId }: { api: WindowApi; sessionId: string }) {
  const shells = shellsOf(api);
  const shell = useShellOf(shells, sessionId);
  const title = useJobTitle(stateOf(api), sessionId);
  const screen = useRef<HTMLDivElement>(null);
  useEffect(() => {
    document.title = windowTitleOf(title);
  }, [title]);
  // The shell this Job has, or a new one where none is running: Fleet hands over the one open.
  useEffect(() => shells?.fresh(sessionId), [shells, sessionId]);
  // Typing goes to the shell from the moment the window opens.
  const ready = shell !== undefined;
  useEffect(() => {
    if (ready) screen.current?.querySelector("textarea")?.focus({ preventScroll: true });
  }, [ready]);
  if (shells === null || shell === undefined) return null;
  const running = !shell.ended;
  return (
    <ShellWindowPane
      phase={running ? { state: "running" } : { state: "ended" }}
      onEnd={() => shells.stop(sessionId)}
      onOpen={() => shells.fresh(sessionId)}
      moves={[
        {
          to: "docked",
          onMove: () => {
            saveJobTerminalPlace("docked");
            shells.moveBack(sessionId, "docked");
          },
        },
        {
          to: "side panel",
          onMove: () => {
            saveJobTerminalPlace("side panel");
            shells.moveBack(sessionId, "side panel");
          },
        },
      ]}
    >
      {/* Esc is the shell's: nothing in this window closes on it. */}
      <div ref={screen} className="terminal-window__screen">
        <XtermScreen key={shell.screen} out={shell.out} run={shell.screen} running={running} write={(data) => shells.write(sessionId, data)} onSize={(cols, rows) => shells.sized(sessionId, cols, rows)} />
      </div>
    </ShellWindowPane>
  );
}
