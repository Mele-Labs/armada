// One of a Job's terminals in a window of its own (`docs/concepts/session/shared-terminal.md`): the whole document is the pane. The window's
// own close puts the shell away; its head moves it back into Bridge, docked or in the side panel, through main.

import { useEffect, useRef, useState } from "react";
import { ShellWindowPane, XtermScreen } from "@armada/components";
import type { SessionsRead } from "../../../shared/api/sessions";

import { shellsOf, useShellOf } from "../job-shell";
import { stateOf, type StateSource } from "../state-feed";

type WindowApi = Parameters<typeof shellsOf>[0] & Parameters<typeof stateOf>[0];

/** What the window's title says: the terminal's name, then the Job's title where the Sessions list has one. */
export const windowTitleOf = (name: string | undefined, title: string | undefined): string => {
  const terminal = name === undefined || name === "" ? "Terminal" : name;
  return title === undefined || title === "" ? terminal : `${terminal}: ${title}`;
};

const titleIn = (sessions: SessionsRead, sessionId: string): string | undefined =>
  sessions.state === "read"
    ? sessions.sessions.find((one) => one.id === sessionId)?.title
    : undefined;

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
  }, [
    source,
    sessionId,
  ]);
  return title;
}

/** The terminal's name as Fleet keeps it, read once the shell is there. Absent until then and where Fleet serves no list. */
export function useTerminalName(
  api: WindowApi,
  sessionId: string,
  terminalNamed: string | undefined,
): string | undefined {
  const shells = shellsOf(api);
  const key = shells?.keyOf(sessionId, terminalNamed) ?? sessionId;
  const shell = useShellOf(shells, key);
  const terminal = shells?.terminalOf(key) ?? terminalNamed;
  const running = typeof shell?.runId === "string" && !shell.ended;
  const [name, setName] = useState<string>();
  useEffect(() => {
    if (!running || terminal === undefined || api.listTerminals === undefined) return;
    let live = true;
    void api.listTerminals(sessionId).then((listed) => {
      if (live && listed.ok)
        setName(listed.value.terminals.find((one) => one.terminal === terminal)?.name);
    });
    return () => {
      live = false;
    };
  }, [
    api,
    sessionId,
    terminal,
    running,
  ]);
  return name;
}

/**
 * The window's document: the Job's shell `terminal` (the first, without one), opened or handed over as the pane fits, and its moves
 * back into Bridge.
 */
export function TerminalWindow({
  api,
  sessionId,
  terminal,
}: {
  api: WindowApi;
  sessionId: string;
  terminal?: string;
}) {
  const shells = shellsOf(api);
  const key = shells?.keyOf(sessionId, terminal) ?? sessionId;
  const shell = useShellOf(shells, key);
  const name = useTerminalName(api, sessionId, terminal);
  const title = useJobTitle(stateOf(api), sessionId);
  const screen = useRef<HTMLDivElement>(null);
  useEffect(() => {
    document.title = windowTitleOf(name, title);
  }, [
    name,
    title,
  ]);
  // The shell this terminal has, or a new one where none is running: Fleet hands over the one open.
  useEffect(
    () => shells?.fresh(key),
    [
      shells,
      key,
    ],
  );
  // Typing goes to the shell from the moment the window opens.
  const ready = shell !== undefined;
  useEffect(() => {
    if (ready)
      screen.current?.querySelector("textarea")?.focus({
        preventScroll: true,
      });
  }, [
    ready,
  ]);
  if (shells === null || shell === undefined) return null;
  const running = !shell.ended;
  return (
    <ShellWindowPane
      phase={
        running
          ? {
              state: "running",
            }
          : {
              state: "ended",
            }
      }
      onEnd={() => shells.stop(key)}
      onOpen={() => shells.fresh(key)}
      moves={[
        {
          to: "docked",
          onMove: () => shells.moveBack(key, "docked"),
        },
      ]}
    >
      {/* Esc is the shell's: nothing in this window closes on it. */}
      <div ref={screen} className="terminal-window__screen">
        <XtermScreen
          key={shell.screen}
          out={shell.out}
          run={shell.screen}
          running={running}
          write={(data) => shells.write(key, data)}
          onSize={(cols, rows) => shells.sized(key, cols, rows)}
        />
      </div>
    </ShellWindowPane>
  );
}
