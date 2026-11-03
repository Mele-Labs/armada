// A Job's shells: as many terminals as the person and the Job's agent open, each in the worktree, which the person types
// into from Bridge and the agent types into through its tools. Over `session-run-in-terminal`, so the block
// Run in "Deploy the store" docks beside them as a tab. Fleet's half is here: `openTerminal` opens a shell and
// mints its ID (t1, t2, never reused in a Job), hands back one already open, the run calls act on it by its id, and
// `terminal.opened` and `terminal.exited` follow it. `listTerminals` names the open ones and says whether a program holds the
// foreground in each, and `renameTerminal` keeps the person's name for the agent's list.
// Each time passes, the agent opens a terminal of its own, once, and types `pnpm dev` into it. The walk `job-terminals` plays it.

import {
  type JobTerminalChange,
  type TerminalMovedBack,
  type TerminalWindowId,
} from "../../../../shared/api/sessions";
import type { OpenTerminal, SessionRunMessage } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { unanswered, type Scenario } from "../moment";
import { startShell, type FakeShell } from "../terminal-run/shell";
import { s256SessionRunInTerminal } from "./session-run-in-terminal";

/** The Job whose agent types into its shell, and the worktree it holds. */
export const SHELL_JOB = "s60";
const WORKTREE = "~/Development/armada/.armada/slots/slot-1";
const AGENT_TYPES = "pnpm dev";

type ShellFleet = ReturnType<typeof shellFleet>;

/** A terminal's window or shell under one key: the Job, and the terminal's ID where there is one. */
const keyOf = ({ sessionId, terminal }: TerminalWindowId): string =>
  terminal === undefined ? sessionId : `${sessionId}#${terminal}`;

/** The number in a terminal's ID. */
const numberOf = (terminal: string): number => Number(terminal.slice(1));

/** The number in a `keyOf` that names a terminal. */
const numberIn = (key: string): number => numberOf(key.slice(key.indexOf("#") + 1));

/** The window's Fleet, so time passing can reach the agent. The latest window wins. */
let live: ShellFleet | undefined;

/** Fleet's shells. Like main, it replays nothing to a window that is handed a shell it already follows. */
function shellFleet() {
  const hearing = new Set<(runId: string, message: SessionRunMessage) => void>();
  const watching = new Set<(change: JobTerminalChange) => void>();
  const shells = new Map<string, FakeShell>();
  /** Each Job's running shells: terminal ID to run. */
  const openIn = new Map<string, Map<string, string>>();
  /** The last ID minted in each Job. */
  const mintedIn = new Map<string, number>();
  /** The terminal the Job's agent opened for itself. */
  const agentIn = new Map<string, string>();
  const calls = {
    opened: [] as {
      sessionId: string;
      cols?: number;
      rows?: number;
      terminal?: string;
      new?: boolean;
    }[],
    stopped: [] as string[],
    started: [] as string[],
    written: [] as string[],
    resized: [] as {
      runId: string;
      cols: number;
      rows: number;
    }[],
    windows: [] as TerminalWindowId[],
    renamed: [] as {
      sessionId: string;
      terminal: string;
      name: string;
    }[],
  };
  let minted = 0;
  // Main's half: what each shell printed, for a window that comes to it late, and the Jobs with a window of their own.
  const printed = new Map<string, string>();
  const windowed = new Map<string, TerminalWindowId>();
  const windowsHeard = new Set<(windows: readonly TerminalWindowId[]) => void>();
  const raisedHeard = new Set<(id: TerminalWindowId) => void>();
  const movesHeard = new Set<(move: TerminalMovedBack) => void>();
  /** The person's names for terminals, by `keyOf`: `Terminal N` where there is none. */
  const names = new Map<string, string>();
  const lastShell = new Map<
    string,
    {
      runId: string;
      ended: boolean;
    }
  >();
  const send = (runId: string, message: SessionRunMessage) => {
    if (message.kind === "output") printed.set(runId, (printed.get(runId) ?? "") + message.text);
    hearing.forEach((one) => one(runId, message));
  };
  const tell = (change: JobTerminalChange) => watching.forEach((one) => one(change));
  const windowsChanged = () =>
    windowsHeard.forEach((one) =>
      one([
        ...windowed.values(),
      ]),
    );
  /** The person closing the terminal's window, or main closing it for a move back. */
  const closeWindow = (id: TerminalWindowId) => {
    if (windowed.delete(keyOf(id))) windowsChanged();
  };

  const runningIn = (sessionId: string) => {
    let held = openIn.get(sessionId);
    if (held === undefined) openIn.set(sessionId, (held = new Map()));
    return held;
  };

  const mint = (sessionId: string): string => {
    const last = (mintedIn.get(sessionId) ?? 0) + 1;
    mintedIn.set(sessionId, last);
    return `t${last}`;
  };

  /** The lowest-numbered of the Job's running shells. */
  const lowest = (held: Map<string, string>): string | undefined =>
    [
      ...held.keys(),
    ].sort((x, y) => numberOf(x) - numberOf(y))[0];

  /**
   * A shell in the Job as Fleet opens one: a new one, the named one (its running shell, or a new one under the same ID where it ended),
   * or neither, which is the lowest-numbered running, else `t1`. A terminal the Job never had is refused, as is `new` with a terminal.
   */
  const open = (
    sessionId: string,
    which: Pick<OpenTerminal, "terminal" | "new"> = {},
  ):
    | {
        runId: string;
        terminal: string;
      }
    | undefined => {
    const held = runningIn(sessionId);
    if (which.new === true && which.terminal !== undefined) return undefined;
    if (which.terminal !== undefined && numberOf(which.terminal) > (mintedIn.get(sessionId) ?? 0))
      return undefined;
    const terminal =
      which.new === true || (which.terminal === undefined && (mintedIn.get(sessionId) ?? 0) === 0)
        ? mint(sessionId)
        : (which.terminal ?? lowest(held) ?? "t1");
    const already = held.get(terminal);
    if (already !== undefined)
      return {
        runId: already,
        terminal,
      };
    const runId = `shell-${(minted += 1)}`;
    calls.started.push(runId);
    held.set(terminal, runId);
    lastShell.set(
      keyOf({
        sessionId,
        terminal,
      }),
      {
        runId,
        ended: false,
      },
    );
    // Main follows the shell before the window hears it opened, so nothing it prints is missed.
    tell({
      session_id: sessionId,
      run_id: runId,
      change: "opened",
      terminal,
    });
    const shell = startShell(
      WORKTREE,
      (text) =>
        send(runId, {
          kind: "output",
          text,
        }),
      (code) => {
        shells.delete(runId);
        if (held.get(terminal) === runId) held.delete(terminal);
        if (
          lastShell.get(
            keyOf({
              sessionId,
              terminal,
            }),
          )?.runId === runId
        )
          lastShell.set(
            keyOf({
              sessionId,
              terminal,
            }),
            {
              runId,
              ended: true,
            },
          );
        send(runId, {
          kind: "exited",
          code,
          stopped: code === null,
        });
        tell({
          session_id: sessionId,
          run_id: runId,
          change: "exited",
          terminal,
        });
      },
    );
    shells.set(runId, shell);
    return {
      runId,
      terminal,
    };
  };

  return {
    calls,
    closeWindow,
    onRaised: (on: (id: TerminalWindowId) => void) => {
      raisedHeard.add(on);
      return () => void raisedHeard.delete(on);
    },
    /** The agent's `type_in_terminal`: a terminal of its own, opened once, and its words with Enter after a beat. */
    agentTypes: (sessionId: string, words: string) => {
      if (agentIn.has(sessionId)) return;
      const { runId, terminal } = open(sessionId, {
        new: true,
      })!;
      agentIn.set(sessionId, terminal);
      window.setTimeout(() => shells.get(runId)?.write(`${words}\r`), 400);
    },
    dispose: () =>
      [
        ...shells.values(),
      ].forEach((one) => one.stop()),
    api: {
      openTerminal: async (
        sessionId: string,
        cols?: number,
        rows?: number,
        which?: Pick<OpenTerminal, "terminal" | "new">,
      ) => {
        calls.opened.push({
          sessionId,
          ...(cols === undefined
            ? {}
            : {
                cols,
              }),
          ...(rows === undefined
            ? {}
            : {
                rows,
              }),
          ...(which?.terminal === undefined
            ? {}
            : {
                terminal: which.terminal,
              }),
          ...(which?.new === true
            ? {
                new: true,
              }
            : {}),
        });
        const opened = open(sessionId, which);
        if (opened === undefined)
          return {
            ok: false as const,
            outcome: unanswered("/sessions/terminal/open"),
          };
        return {
          ok: true as const,
          value: {
            run_id: opened.runId,
            terminal: opened.terminal,
          },
        };
      },
      listTerminals: async (sessionId: string) => ({
        ok: true as const,
        value: {
          terminals: [
            ...runningIn(sessionId),
          ]
            .sort(([x], [y]) => numberOf(x) - numberOf(y))
            .map(([terminal, runId]) => ({
              terminal,
              name:
                names.get(
                  keyOf({
                    sessionId,
                    terminal,
                  }),
                ) ?? `Terminal ${numberOf(terminal)}`,
              run_id: runId,
              running: shells.get(runId)?.running() === true,
            })),
        },
      }),
      renameTerminal: async (sessionId: string, terminal: string, name: string) => {
        calls.renamed.push({
          sessionId,
          terminal,
          name,
        });
        const runId = openIn.get(sessionId)?.get(terminal);
        if (runId === undefined)
          return {
            ok: false as const,
            outcome: unanswered("/sessions/terminal/rename"),
          };
        const kept = name.trim();
        if (kept === "")
          names.delete(
            keyOf({
              sessionId,
              terminal,
            }),
          );
        else
          names.set(
            keyOf({
              sessionId,
              terminal,
            }),
            kept,
          );
        return {
          ok: true as const,
          value: {
            terminal,
            name: kept === "" ? `Terminal ${numberOf(terminal)}` : kept,
            run_id: runId,
            running: shells.get(runId)?.running() === true,
          },
        };
      },
      writeSessionRun: async (runId: string, data: string) => {
        calls.written.push(data);
        shells.get(runId)?.write(data);
        return {
          ok: true as const,
          value: {
            run_id: runId,
          },
        };
      },
      resizeSessionRun: async (runId: string, cols: number, rows: number) => {
        calls.resized.push({
          runId,
          cols,
          rows,
        });
        return {
          ok: true as const,
          value: {
            run_id: runId,
          },
        };
      },
      stopSessionRun: async (runId: string) => {
        calls.stopped.push(runId);
        shells.get(runId)?.stop();
        return {
          ok: true as const,
          value: {
            run_id: runId,
          },
        };
      },
      onSessionRunMessage: (on: (runId: string, message: SessionRunMessage) => void) => {
        hearing.add(on);
        return () => void hearing.delete(on);
      },
      onSessionTerminal: (on: (change: JobTerminalChange) => void) => {
        watching.add(on);
        return () => void watching.delete(on);
      },
      // A second open brings the terminal's window to the front: the mock's window hears it raised, and the terminals with a window do not change.
      openTerminalWindow: async (sessionId: string, terminal?: string) => {
        const id: TerminalWindowId = {
          sessionId,
          ...(terminal === undefined
            ? {}
            : {
                terminal,
              }),
        };
        calls.windows.push(id);
        if (windowed.has(keyOf(id))) {
          raisedHeard.forEach((one) => one(id));
          return;
        }
        windowed.set(keyOf(id), id);
        windowsChanged();
      },
      getTerminalWindows: async () => [
        ...windowed.values(),
      ],
      onTerminalWindows: (on: (windows: readonly TerminalWindowId[]) => void) => {
        windowsHeard.add(on);
        return () => void windowsHeard.delete(on);
      },
      readTerminalSoFar: async (runId: string) => printed.get(runId) ?? null,
      moveTerminalBack: async (
        sessionId: string,
        to: TerminalMovedBack["to"],
        terminal?: string,
      ) => {
        const id: TerminalWindowId = {
          sessionId,
          ...(terminal === undefined
            ? {}
            : {
                terminal,
              }),
        };
        closeWindow(id);
        // Main's `shellNow`: the named terminal's last shell, else the Job's lowest-numbered.
        const shell = lastShell.get(
          terminal === undefined
            ? ([
                ...lastShell.keys(),
              ]
                .filter((key) => key.startsWith(`${sessionId}#`))
                .sort((x, y) => numberIn(x) - numberIn(y))[0] ?? sessionId)
            : keyOf(id),
        );
        movesHeard.forEach((one) =>
          one({
            sessionId,
            ...(terminal === undefined
              ? {}
              : {
                  terminal,
                }),
            to,
            ...(shell === undefined
              ? {}
              : {
                  shell,
                }),
          }),
        );
      },
      onTerminalMovedBack: (on: (move: TerminalMovedBack) => void) => {
        movesHeard.add(on);
        return () => void movesHeard.delete(on);
      },
    },
  };
}

/** What the scenario's Fleet was asked, for a test. */
export const shellCalls = () => live?.calls;

/** The person closing a terminal's window with the window's own close. */
export const closeTerminalWindow = (id: TerminalWindowId) => live?.closeWindow(id);

/** The mock's window for a terminal hearing its window asked for again, which brings it to the front. */
export const onTerminalWindowRaised = (on: (id: TerminalWindowId) => void): (() => void) =>
  live?.onRaised(on) ?? (() => undefined);

/** The shell's half of Fleet for a scenario in another file: this window's shells, which a Job's Terminal tab and Flight control's Terminal open. */
export function shellApi(): ShellFleet["api"] {
  live = shellFleet();
  return live.api;
}

/** Puts this window's shells away, with the Sessions that held them. */
export const disposeShells = (): void => live?.dispose();

const base = s256SessionRunInTerminal;

export const s322SessionShell: Scenario = {
  ...base,
  name: "session-shell",
  says: "A Job's shell, open in its worktree, which the person and the Job's agent both type into",
  // A served line, so the Cockpit draws the merge queue under the Job and the docked shell between them.
  state: {
    ...base.state,
    mergeLines: {
      lines: [
        {
          root: repository().root,
          line: [],
          off: [],
          landed: [],
          sent_back: [],
        },
      ],
    },
  },
  behaves: (fleet) => {
    live = shellFleet();
    return {
      ...base.behaves?.(fleet),
      ...live.api,
    };
  },
  draft: {
    ...base.draft,
    sessions: (board) => {
      const store = base.draft!.sessions!(board);
      let rows = 0;
      return {
        ...store,
        later: () => {
          store.later();
          const at = new Date().toISOString();
          store.edit(SHELL_JOB, (one) => ({
            ...one,
            rows: [
              ...one.rows,
              {
                id: `sh${(rows += 1)}`,
                at,
                kind: "message",
                from: {
                  kind: "agent",
                },
                text: "Starting the dev server in the terminal.",
              },
            ],
          }));
          live?.agentTypes(SHELL_JOB, AGENT_TYPES);
        },
        dispose: () => {
          live?.dispose();
          store.dispose();
        },
      };
    },
  },
};
