// The Job's shell: one terminal per Job that stays open in its worktree, which the person types into from
// Bridge and the Job's agent types into through its tools. Over `session-run-in-terminal`, so the block
// Run in "Deploy the store" docks beside the shell as a tab. Fleet's half is here: `openTerminal` hands
// back the shell already open, the run calls act on it by its id, and `terminal.opened` and
// `terminal.exited` follow it. Each time passes, the agent types `pnpm dev` into the Job's shell, opening
// one where none is open. The walk `job-shell` plays it.

import type { JobTerminalChange, TerminalMovedBack } from "../../../../shared/api/sessions";
import type { SessionRunMessage } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import { startShell, type FakeShell } from "../terminal-run/shell";
import { s256SessionRunInTerminal } from "./session-run-in-terminal";

/** The Job whose agent types into its shell, and the worktree it holds. */
export const SHELL_JOB = "s60";
const WORKTREE = "~/Development/armada/.armada/slots/slot-1";
const AGENT_TYPES = "pnpm dev";

type ShellFleet = ReturnType<typeof shellFleet>;

/** The window's Fleet, so time passing can reach the agent. The latest window wins. */
let live: ShellFleet | undefined;

/** Fleet's shells. Like main, it replays nothing to a window that is handed a shell it already follows. */
function shellFleet() {
  const hearing = new Set<(runId: string, message: SessionRunMessage) => void>();
  const watching = new Set<(change: JobTerminalChange) => void>();
  const shells = new Map<string, FakeShell>();
  const openIn = new Map<string, string>();
  const calls = { opened: [] as { sessionId: string; cols?: number; rows?: number }[], stopped: [] as string[], started: [] as string[], written: [] as string[], windows: [] as string[] };
  let minted = 0;
  // Main's half: what each shell printed, for a window that comes to it late, and the Jobs with a window of their own.
  const printed = new Map<string, string>();
  const windowed = new Set<string>();
  const windowsHeard = new Set<(sessionIds: readonly string[]) => void>();
  const raisedHeard = new Set<(sessionId: string) => void>();
  const movesHeard = new Set<(move: TerminalMovedBack) => void>();
  const lastShell = new Map<string, { runId: string; ended: boolean }>();
  const send = (runId: string, message: SessionRunMessage) => {
    if (message.kind === "output") printed.set(runId, (printed.get(runId) ?? "") + message.text);
    hearing.forEach((one) => one(runId, message));
  };
  const tell = (change: JobTerminalChange) => watching.forEach((one) => one(change));
  const windowsChanged = () => windowsHeard.forEach((one) => one([...windowed]));
  /** The person closing the Job's window, or main closing it for a move back. */
  const closeWindow = (sessionId: string) => {
    if (windowed.delete(sessionId)) windowsChanged();
  };

  const open = (sessionId: string): string => {
    const already = openIn.get(sessionId);
    if (already !== undefined) return already;
    const runId = `shell-${(minted += 1)}`;
    calls.started.push(runId);
    openIn.set(sessionId, runId);
    lastShell.set(sessionId, { runId, ended: false });
    // Main follows the shell before the window hears it opened, so nothing it prints is missed.
    tell({ session_id: sessionId, run_id: runId, change: "opened" });
    const shell = startShell(
      WORKTREE,
      (text) => send(runId, { kind: "output", text }),
      (code) => {
        shells.delete(runId);
        if (openIn.get(sessionId) === runId) openIn.delete(sessionId);
        if (lastShell.get(sessionId)?.runId === runId) lastShell.set(sessionId, { runId, ended: true });
        send(runId, { kind: "exited", code });
        tell({ session_id: sessionId, run_id: runId, change: "exited" });
      },
    );
    shells.set(runId, shell);
    return runId;
  };

  return {
    calls,
    closeWindow,
    onRaised: (on: (sessionId: string) => void) => {
      raisedHeard.add(on);
      return () => void raisedHeard.delete(on);
    },
    /** The agent's `type_in_terminal`: the Job's shell, opened where it has none, and its words with Enter after a beat. */
    agentTypes: (sessionId: string, words: string) => {
      const runId = open(sessionId);
      window.setTimeout(() => shells.get(runId)?.write(`${words}\r`), 400);
    },
    dispose: () => [...shells.values()].forEach((one) => one.stop()),
    api: {
      openTerminal: async (sessionId: string, cols?: number, rows?: number) => {
        calls.opened.push({ sessionId, ...(cols === undefined ? {} : { cols }), ...(rows === undefined ? {} : { rows }) });
        return { ok: true as const, value: { run_id: open(sessionId) } };
      },
      writeSessionRun: async (runId: string, data: string) => {
        calls.written.push(data);
        shells.get(runId)?.write(data);
        return { ok: true as const, value: { run_id: runId } };
      },
      resizeSessionRun: async (runId: string) => ({ ok: true as const, value: { run_id: runId } }),
      stopSessionRun: async (runId: string) => {
        calls.stopped.push(runId);
        shells.get(runId)?.stop();
        return { ok: true as const, value: { run_id: runId } };
      },
      onSessionRunMessage: (on: (runId: string, message: SessionRunMessage) => void) => {
        hearing.add(on);
        return () => void hearing.delete(on);
      },
      onSessionTerminal: (on: (change: JobTerminalChange) => void) => {
        watching.add(on);
        return () => void watching.delete(on);
      },
      // A second open brings the Job's window to the front: the mock's window hears it raised, and the Jobs with a window do not change.
      openTerminalWindow: async (sessionId: string) => {
        calls.windows.push(sessionId);
        if (windowed.has(sessionId)) {
          raisedHeard.forEach((one) => one(sessionId));
          return;
        }
        windowed.add(sessionId);
        windowsChanged();
      },
      getTerminalWindows: async () => [...windowed],
      onTerminalWindows: (on: (sessionIds: readonly string[]) => void) => {
        windowsHeard.add(on);
        return () => void windowsHeard.delete(on);
      },
      readTerminalSoFar: async (runId: string) => printed.get(runId) ?? null,
      moveTerminalBack: async (sessionId: string, to: TerminalMovedBack["to"]) => {
        closeWindow(sessionId);
        const shell = lastShell.get(sessionId);
        movesHeard.forEach((one) => one({ sessionId, to, ...(shell === undefined ? {} : { shell }) }));
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

/** The person closing a Job's terminal window with the window's own close. */
export const closeTerminalWindow = (sessionId: string) => live?.closeWindow(sessionId);

/** The mock's window for a Job hearing its window asked for again, which brings it to the front. */
export const onTerminalWindowRaised = (on: (sessionId: string) => void): (() => void) => live?.onRaised(on) ?? (() => undefined);

const base = s256SessionRunInTerminal;

export const s322SessionShell: Scenario = {
  ...base,
  name: "session-shell",
  says: "A Job's shell, open in its worktree, which the person and the Job's agent both type into",
  // A served line, so the Cockpit draws the merge queue under the Job and the docked shell between them.
  state: { ...base.state, mergeLines: { lines: [{ root: repository().root, line: [], off: [], landed: [], sent_back: [] }] } },
  behaves: (fleet) => {
    live = shellFleet();
    return { ...base.behaves?.(fleet), ...live.api };
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
          store.edit(SHELL_JOB, (one) => ({ ...one, rows: [...one.rows, { id: `sh${(rows += 1)}`, at, kind: "message", from: { kind: "agent" }, text: "Starting the dev server in the terminal." }] }));
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
