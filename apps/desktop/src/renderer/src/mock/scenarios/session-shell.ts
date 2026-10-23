// The Job's shell: one terminal per Job that stays open in its worktree, which the person types into from
// Bridge and the Job's agent types into through its tools. Over `session-run-in-terminal`, so the block
// Run in "Deploy the store" docks beside the shell as a tab. Fleet's half is here: `openTerminal` hands
// back the shell already open, the run calls act on it by its id, and `terminal.opened` and
// `terminal.exited` follow it. Each time passes, the agent types `pnpm dev` into the Job's shell, opening
// one where none is open. The walk `job-shell` plays it.

import type { JobTerminalChange } from "../../../../shared/api/sessions";
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
  const calls = { opened: [] as { sessionId: string; cols?: number; rows?: number }[], stopped: [] as string[] };
  let minted = 0;
  const send = (runId: string, message: SessionRunMessage) => hearing.forEach((one) => one(runId, message));
  const tell = (change: JobTerminalChange) => watching.forEach((one) => one(change));

  const open = (sessionId: string): string => {
    const already = openIn.get(sessionId);
    if (already !== undefined) return already;
    const runId = `shell-${(minted += 1)}`;
    openIn.set(sessionId, runId);
    // Main follows the shell before the window hears it opened, so nothing it prints is missed.
    tell({ session_id: sessionId, run_id: runId, change: "opened" });
    const shell = startShell(
      WORKTREE,
      (text) => send(runId, { kind: "output", text }),
      (code) => {
        shells.delete(runId);
        if (openIn.get(sessionId) === runId) openIn.delete(sessionId);
        send(runId, { kind: "exited", code });
        tell({ session_id: sessionId, run_id: runId, change: "exited" });
      },
    );
    shells.set(runId, shell);
    return runId;
  };

  return {
    calls,
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
    },
  };
}

/** What the scenario's Fleet was asked, for a test. */
export const shellCalls = () => live?.calls;

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
