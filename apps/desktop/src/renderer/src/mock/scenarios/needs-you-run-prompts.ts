// A Job waiting on the person to unlock a signing key, with the command in Needs you. Run, it opens a terminal
// under the command that asks for a passphrase; what is typed reaches the program, which exits 0, and the Job is
// told the exit as a `ran` row. Over the `sessions` scenario. The walk `needs-you-run-prompts` plays it.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { tailOf } from "../terminal-run/ansi";
import { startUnlock } from "../terminal-run/passphrase";
import type { Deploy } from "../terminal-run/wrangler";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const COMMAND = "./scripts/unlock-signing-key";
const ITEM = "k-unlock";

const signing: Session = {
  id: "s61",
  title: "Sign the release tag",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(1),
  rows: [
    { id: "k1", at: "09:40:02", kind: "message", from: { kind: "you" }, text: "Tag and sign the 2.4.0 release." },
    { id: "k2", at: "09:41:30", kind: "message", from: { kind: "agent" }, text: `The signing key is locked. Unlock it and I will sign the tag:\n\n\`\`\`bash\n${COMMAND}\n\`\`\`` },
  ],
  attachments: [{ kind: "slot", slot: 2 }, { kind: "branch", name: "release/2.4.0", slot: 2 }],
  waitingFor: [{ id: ITEM, text: "Unlock the signing key", since: minutesAgo(1), source: "agent", act: { kind: "run", target: COMMAND } }],
};

function sessionsOf(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]) {
  const store = s200Sessions.draft!.sessions!(board, [signing]);
  const running = new Set<Deploy>();
  let rows = 0;
  const add = (id: string, make: (rowId: string, at: string) => SessionRow) => store.edit(id, (one) => ({ ...one, rows: [...one.rows, make(`u${(rows += 1)}`, new Date().toISOString())] }));
  return {
    ...store,
    terminalRuns: true as const,
    run: (id: string, command: string, on: Parameters<NonNullable<typeof store.run>>[2]) => {
      if (command !== COMMAND) return store.run!(id, command, (event) => on(event.kind === "out" ? { kind: "out", text: event.text.replaceAll("\n", "\r\n") } : event));
      let printed = "";
      const program = startUnlock(
        command,
        (text) => {
          printed += text;
          on({ kind: "out", text });
        },
        (code) => {
          running.delete(program);
          add(id, (rowId, at) => ({ id: rowId, at, kind: "ran", command, code, tail: tailOf(printed, 20) }));
          on(code === null ? { kind: "end", code, stopped: true } : { kind: "end", code });
        },
      );
      running.add(program);
      return { stop: program.stop, write: program.write };
    },
    answerWaiting: (id: string, itemId: string, given: Parameters<NonNullable<typeof store.answerWaiting>>[2]) =>
      itemId === ITEM
        ? store.edit(id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId) }))
        : store.answerWaiting?.(id, itemId, given),
    dispose: () => {
      running.forEach((one) => one.stop());
      store.dispose();
    },
  };
}

export const s333NeedsYouRunPrompts: Scenario = {
  ...s200Sessions,
  name: "needs-you-run-prompts",
  says: "A Job waiting on the person to unlock a signing key, from Needs you, in a terminal that asks for a passphrase",
  draft: { sessions: sessionsOf },
};
