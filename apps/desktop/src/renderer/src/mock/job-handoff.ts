// A Job that has grown too large asks to carry on in a new one. He says yes with 1 and the new Job takes its place everywhere: one row,
// the same position and title, the old Job hidden behind it. The store here plays what Fleet does (`crates/fleet/src/session_host/replacing.rs`
// and `handoff_answers.rs`). Built on the Cockpit with four Jobs, so a row's place can be seen to hold. The walks `jobHandoff`,
// `jobHandoffFails`, `jobHandoffCard` and `jobHandoffAuto` play it.
//
// Keys. The call card: 1 carry on in a new Job, 2 stay here, 3 stay and do not ask again this Job, 4 remind at a higher limit (then 1 to 3 pick the figure), Answer sends the one picked (the card's own keys). The row: h opens the earlier Job; Esc or Back returns.
//
// **The transfer is Fleet's own, `release_session_holds` with a target** (`docs/concepts/session/what-is-kept.md`, *Releasing what a Job holds*):
// each held row moves to the new Job as it stands and the old Job keeps none. Fleet's covers slot, branch and pull request; here every row moves.

import { HANDOFF_QUIET, HANDOFF_REMIND, HANDOFF_REMIND_AT, HANDOFF_REMIND_QUESTION, HANDOFF_STAY, HANDOFF_YES } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";
import { savedInto } from "@armada/settings/fake";

import { holding } from "./holding";
import type { Scenario } from "./moment";
import { sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const YES = HANDOFF_YES;
const STAY = HANDOFF_STAY;
const QUIET = HANDOFF_QUIET;
const REMIND = HANDOFF_REMIND;
/** The figures the second question offers, in tokens. */
const FIGURES = HANDOFF_REMIND_AT;
const figure = (tokens: number) => `${tokens / 1000}k`;
/** The Settings values of `plan.contextHandoff`. */
const AUTO = "carry on automatically";
const NEVER = "never";
/** What the setting reads, set by the scenario's save; the store reads it when the Context rule trips. */
let handoffSetting = "ask";
/** What Fleet records in `detail.reason` when the old Job would not end: a bare lowercase fact. */
const REFUSED = "old job did not end: the close was refused";
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
    { label: QUIET, description: "" },
    { label: REMIND, description: "" },
  ],
} as const;

/** The card's second answer, which the first one's REMIND brings forward in its place. */
const REMIND_QUESTION = {
  question: HANDOFF_REMIND_QUESTION,
  header: "Context",
  multi_select: false,
  options: FIGURES.map((tokens) => ({ label: figure(tokens), description: "" })),
} as const;

const askedOf = (questions: readonly (typeof QUESTION | typeof REMIND_QUESTION)[]): NonNullable<Session["asked"]> => ({
  command: questions[0]!.question,
  call: "hand-q1",
  offers: ["allow_once", "refuse"],
  questions,
});

const big = (growing: boolean): Session => ({
  id: BIG,
  title: "Retry the slot lease",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  standing: "idle",
  context: { tokens: growing ? 288_000 : 372_000, window: 400_000 },
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
  ...(growing ? {} : { asked: askedOf([QUESTION]) }),
});

const other = (id: string, title: string, rest: Partial<Session> & Pick<Session, "rows">): Session => ({
  id,
  title,
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
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

/** The store with the hand off in it. `fails` is a new Job that does not come up, on the first try and on the Retry after it; the next Retry succeeds. */
function store(fails: boolean, growing: boolean): SessionsStore {
  const base = sessionsStore([big(growing), prune, audit, names]);
  // The mock's first start is a fixture Session with a script of its own; it is spent here and kept off the ship.
  base.start();
  const own = new Set([BIG, PRUNE, AUDIT, NAMES]);
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  let tries = 0;
  /** Jobs told not to ask again, and the figure a Job was told to ask again at. These override the setting for that Job. */
  const quiet = new Set<string>();
  let remindAt: number | undefined;
  let tripped = false;

  async function handOff(id: string) {
    base.edit(id, (one) => {
      const { asked, ...rest } = one;
      void asked;
      return { ...rest, handoff: "starting" };
    });
    await new Promise<void>((resolve) => window.setTimeout(resolve, COMING_UP_MS));
    const old = base.get().find((one) => one.id === id);
    tries += 1;
    const made = (fails && tries < 3) || old === undefined ? undefined : await base.start();
    if (made === undefined || old === undefined) {
      // The old Job is as it was and keeps its row, the thread untouched; the row carries the mark. The call was answered, so none comes back.
      base.edit(id, (one) => ({ ...one, handoff: "failed", handoffReason: REFUSED }));
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
      lastTurnAt: ago(0),
    }));
    base.edit(id, (one) => {
      const { handoff, handoffReason, ...rest } = one;
      void handoff;
      void handoffReason;
      return { ...rest, dead: "ended", replacedBy: made, attachments: [], turn: { state: "idle" } };
    });
  }

  return {
    ...base,
    retryHandoff: (id) => void handOff(id),
    // A Job dispatched from the failed band shows on the list like any other.
    start: async (...args) => {
      const id = await base.start(...args);
      if (id !== undefined) own.add(id);
      return id;
    },
    get: () => {
      const all = base.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => own.has(one.id));
      }
      return only;
    },
    // The Context rule trips: the Job's own answer wins, then the setting says ask, carry on, or nothing.
    later: () => {
      base.later();
      if (!growing || tripped) return;
      tripped = true;
      base.edit(BIG, (one) => ({ ...one, context: { tokens: 372_000, window: 400_000 } }));
      if (quiet.has(BIG) || handoffSetting === NEVER) return;
      if (handoffSetting === AUTO) return void handOff(BIG);
      base.edit(BIG, (one) => ({ ...one, asked: askedOf([QUESTION]) }));
    },
    answer: (id, answer, answers) => {
      const asked = base.get().find((one) => one.id === id)?.asked;
      const first = answers?.[0];
      if (id === BIG && asked !== undefined && first !== undefined) {
        const clear = () => base.edit(id, (one) => {
          const { asked: _gone, ...rest } = one;
          return rest;
        });
        if (first.chosen[0] === YES) return void handOff(id);
        if (first.chosen[0] === QUIET) {
          quiet.add(id);
          return clear();
        }
        if (first.chosen[0] === REMIND) return base.edit(id, (one) => ({ ...one, asked: askedOf([REMIND_QUESTION]) }));
        if (first.question === REMIND_QUESTION.question) {
          remindAt = FIGURES.find((tokens) => figure(tokens) === first.chosen[0]);
          void remindAt;
          return clear();
        }
      }
      base.answer(id, answer, answers);
    },
  };
}

/** `growing` is a Job still under the rule's line, which a walk's `later` takes over it. */
export function jobHandoff(name: string, said: string, fails = false, growing = false): Scenario {
  const base = holding(name, said);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: { sessions: () => store(fails, growing) },
    // The Settings row is saved through here, and the store reads it where the rule trips.
    behaves: (fleet) => {
      handoffSetting = "ask";
      return {
        saveSettings: async (changes) => {
          const held = fleet.state().settings;
          if (held !== null) fleet.publish({ settings: savedInto(held, changes) });
          const next = changes["plan.contextHandoff"];
          if (typeof next === "string") handoffSetting = next;
          else if (next === null) handoffSetting = "ask";
          return { ok: true };
        },
      };
    },
  };
}
