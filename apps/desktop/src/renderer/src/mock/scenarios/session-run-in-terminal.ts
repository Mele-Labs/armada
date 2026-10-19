// A Session waiting on the person to sign in to Cloudflare and deploy, with the one command in its message
// and in Needs you. Run, it opens a terminal that takes keys, docked above the box: wrangler's login, then
// its three prompts. When it ends the Session is told the exit and the last lines, as a `ran` row, and
// answers. Over the `sessions` scenario. The walk `run-in-terminal-docked` plays it.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { tailOf } from "../terminal-run/ansi";
import { startDeploy, type Deploy } from "../terminal-run/wrangler";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const COMMAND = "cd ~/Development/armada/.armada/slots/slot-1/apps/store && pnpm exec wrangler login && pnpm exec wrangler deploy";
const ITEM = "d-deploy";
/** The lines the Session is sent. */
const TAIL_LINES = 20;

const deploying: Session = {
  id: "s60",
  title: "Deploy the store",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "d1", at: "14:58:12", kind: "message", from: { kind: "you" }, text: "Deploy the store worker." },
    {
      id: "d2",
      at: "15:02:40",
      kind: "message",
      from: { kind: "agent" },
      text: `The deploy needs you to sign in to Cloudflare:\n\n\`\`\`bash\n${COMMAND}\n\`\`\``,
    },
  ],
  attachments: [{ kind: "slot", slot: 1 }, { kind: "branch", name: "store/deploy", slot: 1 }],
  waitingFor: [{ id: ITEM, text: "Deploy the store to Cloudflare", since: minutesAgo(2), source: "agent", act: { kind: "run", target: COMMAND } }],
};

function sessionsOf(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]) {
  const store = s200Sessions.draft!.sessions!(board, [deploying]);
  const running = new Set<Deploy>();
  const timers = new Set<number>();
  let rows = 0;
  const after = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };
  const stamp = () => new Date().toISOString();
  const add = (id: string, make: (rowId: string, at: string) => SessionRow) => store.edit(id, (one) => ({ ...one, rows: [...one.rows, make(`t${(rows += 1)}`, stamp())] }));

  // The Session is told the moment the run ends, whatever the exit, and answers a clean one.
  const told = (id: string, command: string, code: number | null, printed: string) => {
    const tail = tailOf(printed, TAIL_LINES);
    add(id, (rowId, at) => ({ id: rowId, at, kind: "ran", command, code, tail }));
    if (code !== 0) return;
    const address = /https:\/\/armada-store\.\S+\.workers\.dev/.exec(tail)?.[0] ?? "";
    after(700, () => store.edit(id, (one) => ({ ...one, turn: { state: "working" } })));
    after(2000, () => {
      add(id, (rowId, at) => ({ id: rowId, at, kind: "message", from: { kind: "agent" }, text: `The store is live at ${address}. Putting that address in \`crates/config/settings.toml\`.` }));
      store.edit(id, (one) => ({ ...one, turn: { state: "idle" }, lastTurnAt: new Date().toISOString() }));
    });
  };

  return {
    ...store,
    terminalRuns: true as const,
    run: (id: string, command: string, on: Parameters<NonNullable<typeof store.run>>[2]) => {
      if (!command.includes("wrangler deploy")) return store.run!(id, command, (event) => on(event.kind === "out" ? { kind: "out", text: event.text.replaceAll("\n", "\r\n") } : event));
      let printed = "";
      const program = startDeploy(
        command,
        (text) => {
          printed += text;
          on({ kind: "out", text });
        },
        (code) => {
          running.delete(program);
          told(id, command, code, printed);
          on({ kind: "end", code });
        },
      );
      running.add(program);
      return { stop: program.stop, write: program.write };
    },
    // Settling the item is all this run owes: the Session was told by the run's own end.
    answerWaiting: (id: string, itemId: string, given: Parameters<NonNullable<typeof store.answerWaiting>>[2]) =>
      itemId === ITEM
        ? store.edit(id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId) }))
        : store.answerWaiting?.(id, itemId, given),
    dispose: () => {
      running.forEach((one) => one.stop());
      timers.forEach((one) => window.clearTimeout(one));
      store.dispose();
    },
  };
}

export const s256SessionRunInTerminal: Scenario = {
  ...s200Sessions,
  name: "session-run-in-terminal",
  says: "A Session waiting on the person to sign in to Cloudflare and deploy, in a terminal that takes keys",
  draft: { sessions: sessionsOf },
};
