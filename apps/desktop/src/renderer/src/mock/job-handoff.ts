// A Job that has grown too large asks to carry on in a new one. He says yes with 1 and the new Job takes its place everywhere: one row,
// the same position and title, the old Job hidden behind it. Mock only; nothing in Fleet or the protocol carries it. Built on the
// Cockpit with four Jobs, so a row's place can be seen to hold. The walks `jobHandoff` and `jobHandoffFails` play it.
//
// Keys. The call card: 1 carry on in a new Job, 2 stay here, Answer sends the one picked (the card's own keys). The row: h opens the earlier Job; Esc or Back returns.
//
// **The transfer is Fleet's own, `release_session_holds` with a target** (`docs/concepts/session/what-is-kept.md`, *Releasing what a Job holds*):
// each held row moves to the new Job as it stands and the old Job keeps none. Fleet's covers slot, branch and pull request; here every row moves.

import { HANDOFF_STAY, HANDOFF_YES } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "./holding";
import type { Scenario } from "./moment";
import { sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const YES = HANDOFF_YES;
const STAY = HANDOFF_STAY;
/** How long the new Job takes to come up, long enough to see the ring turn. */
const COMING_UP_MS = 1800;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });
const tool = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "tool", text });

const BIG = "hand-1";
const PRUNE = "hand-2";
const AUDIT = "hand-3";
const NAMES = "hand-4";

const QUESTION = {
  question: "Carry on in a new Job?",
  header: "Context",
  multi_select: false,
  options: [
    { label: YES, description: "" },
    { label: STAY, description: "" },
  ],
} as const;

const big = (): Session => ({
  id: BIG,
  title: "Retry the slot lease",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurn: "14:12",
  lastTurnAt: ago(2),
  standing: "idle",
  context: { tokens: 372_000, window: 400_000 },
  rows: [
    you("b-1", "13:20:00", "Make the slot lease retry when two Jobs ask for the same bay at once."),
    tool("b-2", "13:20:12", "Read crates/fleet/src/lease.rs"),
    agent("b-3", "13:21:02", "Two Jobs can ask for the same bay between the read and the write. The second is refused and gives up at once. The fix is to ask again after a short wait, up to a cap."),
    tool("b-4", "13:24:20", "Write crates/fleet/src/retry.rs"),
    tool("b-5", "13:27:01", 'Bash git commit -m "Retry the slot lease when the pool is contended"'),
    agent("b-6", "13:40:12", "The retry is in Fleet and its checks pass. The wire types and the bay tile are left. This thread is close to the window."),
  ],
  attachments: [
    { kind: "slot", slot: 3 },
    { kind: "branch", name: "armada/lease-retry", slot: 3 },
    { kind: "pull_request", number: 2390, title: "Retry the slot lease", branch: "armada/lease-retry", address: "https://git.example/armada/pull/2390", checks: { state: "pending" }, state: "open", auto: false },
    { kind: "artifact", form: "file", id: "docs/spikes/lease-contention.md", title: "Lease contention measured" },
    { kind: "background", id: "bg-1", form: "command", task: "Serve the mock", command: "pnpm --filter @armada/desktop mock", started: ago(24), state: "running" },
  ],
  asked: { command: QUESTION.question, call: "hand-q1", offers: ["allow_once", "refuse"], questions: [QUESTION] },
});

const other = (id: string, title: string, rest: Partial<Session> & Pick<Session, "rows">): Session => ({
  id,
  title,
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "13:50",
  lastTurnAt: ago(20),
  standing: "idle",
  attachments: [],
  ...rest,
});

const prune = other(PRUNE, "Prune the stale branches", { rows: [you("p-1", "13:30:00", "Prune the branches that are merged and a month old."), agent("p-2", "13:31:10", "Eleven branches are merged and old enough.")] });
const audit = other(AUDIT, "Rotate the audit log", {
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(1),
  rows: [you("a-1", "13:50:00", "Rotate the audit log at midnight."), tool("a-2", "14:00:05", "Edit crates/store/src/audit.rs")],
  attachments: [{ kind: "branch", name: "armada/audit-rotation", slot: 1 }],
});
const names = other(NAMES, "Name the store migrations", { rows: [you("n-1", "13:10:00", "Name the store migrations by their order."), agent("n-2", "13:41:10", "The migrations are named.")], lastTurnAt: ago(50) });

/** The store with the hand off in it. `fails` is a new Job that does not come up. */
function store(fails: boolean): SessionsStore {
  const base = sessionsStore([big(), prune, audit, names]);
  // The mock's first start is a fixture Session with a script of its own; it is spent here and kept off the ship.
  base.start();
  const own = new Set([BIG, PRUNE, AUDIT, NAMES]);
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];

  async function handOff(id: string) {
    base.edit(id, (one) => {
      const { asked, ...rest } = one;
      void asked;
      return { ...rest, handoff: "starting" };
    });
    await new Promise<void>((resolve) => window.setTimeout(resolve, COMING_UP_MS));
    const old = base.get().find((one) => one.id === id);
    const made = fails || old === undefined ? undefined : await base.start();
    if (made === undefined || old === undefined) {
      // The old Job is as it was and keeps its row, the thread untouched; the row carries the mark. The call was answered, so none comes back.
      base.edit(id, (one) => ({ ...one, handoff: "failed" }));
      return;
    }
    own.add(made);
    // **The transfer is `release_session_holds` with a target**: each held row moves as it stands, and the old Job keeps none.
    base.edit(made, () => ({
      id: made,
      ...(old.title === undefined ? {} : { title: old.title }),
      manifestId: OWNER,
      cwd: ROOT,
      ...(old.model === undefined ? {} : { model: old.model }),
      ...(old.effort === undefined ? {} : { effort: old.effort }),
      ...(old.mode === undefined ? {} : { mode: old.mode }),
      context: { tokens: 14_000, window: 400_000 },
      replaces: id,
      attachments: old.attachments as readonly SessionAttachment[],
      trail: { started: new Date().toISOString() },
      rows: [agent("n-1", "14:14:00", "The retry is in Fleet and its checks pass. The wire types and the bay tile are left.")],
      turn: { state: "idle" },
      standing: "idle",
      lastTurn: "14:14",
      lastTurnAt: ago(0),
    }));
    base.edit(id, (one) => {
      const { handoff, ...rest } = one;
      void handoff;
      return { ...rest, dead: "ended", replacedBy: made, attachments: [], turn: { state: "idle" } };
    });
  }

  return {
    ...base,
    get: () => {
      const all = base.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => own.has(one.id));
      }
      return only;
    },
    answer: (id, answer, answers) => {
      const asked = base.get().find((one) => one.id === id)?.asked;
      if (id === BIG && asked !== undefined && answers?.[0]?.chosen[0] === YES) return void handOff(id);
      base.answer(id, answer, answers);
    },
  };
}

export function jobHandoff(name: string, said: string, fails = false): Scenario {
  const base = holding(name, said);
  return { ...base, state: { ...base.state, repository: null }, draft: { sessions: () => store(fails) } };
}
