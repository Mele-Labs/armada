// Runs a command where a Session works. **One path** for the Needs-you ledger and a call card.
// A shell command that exits clean is settled by Fleet, which takes its item off the list and tells
// the agent. A slash command goes to the terminal and Fleet sees no run, so it is settled here.

import { isSlashCommand, type CodeRunner } from "@armada/components";
import type { Session, SessionCommand, SessionsDraft } from "@armada/screens/src/draft/sessions";

/** What holds the run of a waiting item past the card or row that started it: the ledger's row and the call card of one item share it. */
export const waitingRunKey = (sessionId: string, itemId: string): string => `waiting-run:${sessionId}:${itemId}`;

/**
 * `settle` is how the item of a slash command is settled once it is sent: given what settles it, it may
 * hold that back, as a card does to show the exit before it goes. Left out, it settles at once.
 */
export function runnerOf(draft: SessionsDraft, session: Session, settle: (now: () => void) => void = (now) => now()): CodeRunner | undefined {
  const run = draft.run;
  if (run === undefined) return undefined;
  const names = new Set((session.commands ?? []).map((one) => one.name));
  return (command, on, size) => {
    // A command the terminal lists is a skill or slash command: it goes to the terminal as typed, not to a shell.
    if (session.terminal === true && draft.command !== undefined && isSlashCommand(command.trim(), names)) {
      draft.command(session.id, command.trim());
      on({ kind: "end", code: 0 });
      const asked = (session.waitingFor ?? []).find((one) => one.act?.kind === "run" && one.act.target.trim() === command.trim());
      if (asked !== undefined) settle(() => draft.answerWaiting?.(session.id, asked.id, { text: "Ran: exit 0" }));
      return { stop: () => {} };
    }
    return run(session.id, command, on, size);
  };
}

/**
 * The shell command a message asks for with a leading `!`, as in the agent CLI's own box. One line and
 * something after the `!`: a bare `!` or a pasted block is a message like any other.
 */
export function bangOf(text: string): string | undefined {
  const trimmed = text.trim();
  if (!trimmed.startsWith("!") || /[\r\n]/.test(trimmed)) return undefined;
  const command = trimmed.slice(1).trim();
  return command === "" ? undefined : command;
}

/**
 * Whether a message is `/mcp` typed in a hosted Job, which opens the Job's Loadout on its MCP servers rather than going
 * to the agent: the CLI's own `/mcp` is a screen a hosted Job has no terminal to draw. A terminal Job's goes as typed.
 */
export function opensMcp(session: Pick<Session, "terminal">, text: string, files: number): boolean {
  return session.terminal !== true && files === 0 && text.trim() === "/mcp";
}

const MCP: SessionCommand = { name: "mcp", says: "The Job's MCP servers, in its Loadout" };
const WITH_MCP = new WeakMap<readonly SessionCommand[], readonly SessionCommand[]>();

/** What `/` offers in a hosted Job: the CLI's own commands and `/mcp`, kept as one list while the CLI's is the same. */
export function hostedCommands(commands: readonly SessionCommand[]): readonly SessionCommand[] {
  const kept = WITH_MCP.get(commands);
  if (kept !== undefined) return kept;
  const listed = commands.some((one) => one.name === MCP.name) ? commands : [...commands, MCP];
  WITH_MCP.set(commands, listed);
  return listed;
}
