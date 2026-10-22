// One Job played through on its Field for the walk `jobFieldOffTheFloor`, a moment for each `later` step: writing, thinking, a subagent
// starting, checks that fail with a file named in the record, a re-run after a fix that passes as the subagent finishes, walking the app,
// a call to the forge, and waiting on you. Every moment is what Fleet records: `check` and `window` rows in the thread, tool rows while
// the agent calls tools, and `session.edited`, `session.read` and `session.called` entries dated when each moment comes.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { TimelineEvent, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session, SessionRow, WorkDrone } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const ROOT = repository().root;
const DECK = "apps/desktop/src/renderer/src/cockpit/deck/";
const LAYOUT = `${DECK}layout.ts`;
const BLAMED = `${DECK}layout.test.ts`;
const WIRE = "packages/protocol/src/wire.ts";
const EVENTS = "packages/protocol/src/events.ts";
const ID = "jf-pad";
const TITLE = "Draw the pad on the Field";
const SUBAGENT = "sa-wire";
/** The Agent call that started it, which Fleet keys its drone by. */
const CALL = "toolu-wire";

const stamp = (secondsAgo = 0) => new Date(Date.now() - secondsAgo * 1000).toISOString();

const MAIN: WorkDrone = { id: "main", path: "apps/desktop", doing: "write", file: LAYOUT };
const HELPER: WorkDrone = { id: CALL, agent: "subagent", path: "packages/protocol", doing: "write", file: EVENTS };

const own: Session = {
  id: ID,
  title: TITLE,
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: stamp(60),
  standing: "working",
  rows: [
    { id: "jp-u", at: stamp(300), kind: "message", from: { kind: "you" }, text: "Draw a pad at the near edge of the Field with a light for each check." },
    { id: "jp-t0", at: stamp(1), kind: "tool", text: `Edit ${LAYOUT}` },
  ],
  attachments: [{ kind: "branch", name: "armada/field-pad", slot: 1 }],
  work: { field: [MAIN], files: [{ path: LAYOUT, act: "edit" }] },
};

let made = 0;
const event = (session: Session, kind: TimelineEvent["kind"], secondsAgo: number, extra: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id: 1000 + ++made,
  at: stamp(secondsAgo),
  kind,
  subject: { kind: "session", id: session.id },
  title: session.title!,
  tone: "run",
  says: "",
  manifest_id: ARMADA,
  live: true,
  ...extra,
});
const burst = (kind: "session.read" | "session.edited", secondsAgo: number, path: string) =>
  event(own, kind, secondsAgo, { says: `${kind === "session.read" ? "Read" : "Edited"} 1 file`, detail: path, items: [path] });

const check = (id: string, name: string, state: "running" | "passed" | "failed", target?: string): SessionRow => ({ id, at: stamp(), kind: "check", name, run: 400 + made++, state, ...(target === undefined ? {} : { target }) });

/** Each row replaced by id, as Fleet replaces a check's row when it ends; a new id is added. */
function withRows(one: Session, rows: readonly SessionRow[]): Session {
  const ids = new Set(rows.map((row) => row.id));
  const kept = one.rows.filter((row) => !ids.has(row.id));
  return { ...one, rows: [...kept, ...rows] };
}

/**
 * What each moment does to the Job, whether the agent is calling tools through it, and the entries Fleet writes as it comes. `passes` is
 * how many seconds the step stands for: what the Job's rows recorded is that much older when the moment comes.
 */
type Moment = { change: (one: Session) => Session; busy: boolean; events: () => TimelineEvent[]; passes?: number };

/** Every row the Job holds, dated `seconds` earlier: the time a walk's step stands for has passed since each was recorded. */
const aged = (one: Session, seconds: number): Session => ({ ...one, rows: one.rows.map((row) => (Number.isFinite(Date.parse(row.at)) ? { ...row, at: new Date(Date.parse(row.at) - seconds * 1000).toISOString() } : row)) });

const MOMENTS: readonly Moment[] = [
  // Thinking: the agent stops calling tools, the turn goes on, and a quarter of a minute passes.
  { change: (one) => one, busy: false, events: () => [], passes: 15 },
  // A subagent starts on the protocol, and the Job reads the wire types.
  {
    change: (one) => ({
      ...one,
      attachments: [...one.attachments, { kind: "subagent", id: SUBAGENT, call: CALL, task: "Check the wire types", state: "running" }],
      work: { ...one.work, field: [MAIN, HELPER] },
    }),
    busy: true,
    events: () => [event(own, "session.subagent", 1, { says: "Subagent started", detail: "Explore: Check the wire types" }), burst("session.read", 1, WIRE)],
  },
  // The checks run: two pass and the tests are running.
  { change: (one) => withRows(one, [check("c-fmt", "fmt", "passed"), check("c-lint", "lint", "passed"), check("c-test", "test", "running", BLAMED)]), busy: false, events: () => [] },
  // The tests fail, and the record names the test file.
  { change: (one) => withRows(one, [check("c-test", "test", "failed", BLAMED)]), busy: false, events: () => [] },
  // A fix to the layout, and the checks run again from the first.
  { change: (one) => withRows(one, [check("c-fmt2", "fmt", "running")]), busy: false, events: () => [burst("session.edited", 3, LAYOUT)] },
  // All three pass, and the subagent finishes.
  {
    change: (one) => ({
      ...withRows(one, [check("c-fmt2", "fmt", "passed"), check("c-lint2", "lint", "passed"), check("c-test2", "test", "passed", BLAMED)]),
      attachments: one.attachments.map((held) => (held.kind === "subagent" ? { ...held, state: "done" as const, report: "The wire types match." } : held)),
      work: { ...one.work, field: [MAIN] },
    }),
    busy: false,
    events: () => [event(own, "session.subagent", 0, { says: "Subagent finished", detail: "Explore: Check the wire types" })],
  },
  // The Job walks the app.
  { change: (one) => withRows(one, [{ id: "jp-walk", at: stamp(), kind: "window", title: "The pad on the Field", url: "http://127.0.0.1:5176/?walk=jobFieldOffTheFloor" }]), busy: false, events: () => [event(own, "session.walked", 0, { says: "Walked", detail: "The pad on the Field" })] },
  // The walk ends, and the Job opens a pull request on the forge.
  {
    change: (one) => withRows(one, [{ id: "jp-walk", at: stamp(30), kind: "window", title: "The pad on the Field", url: "http://127.0.0.1:5176/?walk=jobFieldOffTheFloor", ended: stamp() }]),
    busy: true,
    events: () => [event(own, "session.called", 0, { says: "Called the forge", detail: "pr create", call: { service: "forge", name: "the forge", line: "pr create" } })],
  },
  // The turn ends on a question for him.
  {
    change: (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", waitingFor: [{ id: "merge", text: "Merge once you have walked the pad", since: stamp(), source: "agent" }] }),
    busy: false,
    events: () => [],
  },
];

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: ARMADA, slot, path: `${ROOT}/.armada/slots/slot-${slot}`, base: "main", warm: true, behind: 0, held, ...rest });

const pool = (): WorktreesHeld => ({ slots: [bay(1, { state: "hosted_session", session_id: ID, session_title: TITLE }, { branch: "armada/field-pad", since: stamp(600) })] });

/** The store, with the moments in place of the usual turns, and a tool row each second while the agent is calling tools. */
function store(): SessionsStore {
  const inner = sessionsStore([own], [], new Map(), []);
  let at = 0;
  let busy = true;
  let ticks = 0;
  const timer = window.setInterval(() => {
    if (!busy) return;
    ticks += 1;
    inner.edit(ID, (one) => withRows(one, [{ id: `jp-t${ticks}`, at: stamp(), kind: "tool", text: `Edit ${LAYOUT}` }]));
  }, 1000);
  return {
    ...inner,
    later: () => {
      const moment = MOMENTS[at++];
      if (moment === undefined) return;
      busy = moment.busy;
      inner.edit(ID, (one) => moment.change(moment.passes === undefined ? one : aged(one, moment.passes)));
    },
    dispose: () => {
      window.clearInterval(timer);
      inner.dispose();
    },
  };
}

function build(): Scenario {
  const base = holding("job-field-off-the-floor", "One Job played through on its Field: writing, thinking, a subagent, checks failing and passing, a walk, a call and waiting on you");
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(),
    draft: {
      sessions: store,
      events: [event(own, "session.started", 600, { says: "Started" }), burst("session.read", 240, WIRE), burst("session.edited", 20, LAYOUT)],
      eventsLater: MOMENTS.map((one) => one.events),
      areas: mockAreas,
    },
  };
}

export const s315JobFieldOffTheFloor: Scenario = build();
