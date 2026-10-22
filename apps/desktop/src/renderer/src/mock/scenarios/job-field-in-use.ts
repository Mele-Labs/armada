// One Job's Field as the owner used it, for the walk `jobFieldInUse`, a moment for each `later` step. The Job's drone writes
// FieldPanel.tsx while a subagent writes areas.rs; then the Job's drone stops writing and reads a doc; then the subagent finishes and
// the Job opens a walk while its drone writes layout.ts. The draft answers `get_session_file_change` with a diff that grows a line every
// few seconds while a file is written, as `job-field-lens.ts` does, and refuses any other path.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { TimelineEvent, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { FileChangeRead, Session, SessionAttachment, SessionRow, WorkDrone } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const ROOT = repository().root;
const PANEL = "apps/desktop/src/renderer/src/session-view/FieldPanel.tsx";
const LAYOUT = "apps/desktop/src/renderer/src/cockpit/deck/layout.ts";
const READ = "docs/concepts/cockpit.md";
const ID = "jf-use";
const TITLE = "Fix the Field on real Jobs";
const HELPER = { id: "sa-store", agent: "Rust", task: "Keep the stamp in the store", file: "crates/store/src/areas.rs", area: "crates/store" } as const;

/** How often a file being written grows a line. */
const GROW_MS = 2_500;

const stamp = (secondsAgo = 0) => new Date(Date.now() - secondsAgo * 1000).toISOString();
const areaOf = (file: string) => file.split("/").slice(0, 2).join("/");

const main = (file: string, doing: WorkDrone["doing"] = "write"): WorkDrone => ({ id: "main", path: areaOf(file), doing, file });
const helper: WorkDrone = { id: HELPER.id, agent: HELPER.agent, path: HELPER.area, doing: "write", file: HELPER.file };
const subagent = (state: "running" | "done"): SessionAttachment => ({ kind: "subagent", id: HELPER.id, task: HELPER.task, state, agentType: HELPER.agent, ...(state === "done" ? { report: "Written." } : {}) });

const own: Session = {
  id: ID,
  title: TITLE,
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: stamp(60),
  standing: "working",
  rows: [
    { id: "ju-u", at: stamp(300), kind: "message", from: { kind: "you" }, text: "The Field should show the whole repository, and the card should come back." },
    { id: "ju-t0", at: stamp(1), kind: "tool", text: `Edit ${PANEL}` },
  ],
  attachments: [{ kind: "branch", name: "armada/field-in-use", slot: 1 }, subagent("running")],
  work: { field: [main(PANEL), helper], files: [{ path: PANEL, act: "edit" }] },
};

let made = 0;
const event = (kind: TimelineEvent["kind"], secondsAgo: number, extra: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id: 4000 + ++made,
  at: stamp(secondsAgo),
  kind,
  subject: { kind: "session", id: ID },
  title: TITLE,
  tone: "run",
  says: "",
  manifest_id: ARMADA,
  live: true,
  ...extra,
});
const edited = (secondsAgo: number, path: string) => event("session.edited", secondsAgo, { says: "Edited 1 file", detail: path, items: [path] });

const ADDED = ["  const held = pinned === undefined ? undefined : writers.find((one) => one.key === pinned);", "  if (held !== undefined) return held;", "  let out: Writer | undefined;", "  for (const one of writers) if (out === undefined || one.since >= out.since) out = one;", "  return out;", "}"];

/** A file's change after `grown` lines of its hunk. */
function diffOf(path: string, grown: number): string {
  const added = Array.from({ length: grown }, (_, k) => `+${ADDED[k % ADDED.length]}`);
  return [`diff --git a/${path} b/${path}`, `--- a/${path}`, `+++ b/${path}`, `@@ -40,2 +40,${2 + grown} @@`, " // The card.", ...added, " // End.", ""].join("\n");
}

/** What each moment does to the Job, and the entries Fleet writes as it comes. */
type Moment = { change: (one: Session) => Session; events: () => TimelineEvent[] };

const MOMENTS: readonly Moment[] = [
  // The Job's drone stops writing and reads a doc; the subagent still writes.
  { change: (one) => ({ ...one, work: { ...one.work, field: [main(READ, "read"), helper] } }), events: () => [event("session.read", 0, { says: "Read 1 file", detail: READ, items: [READ] })] },
  // The subagent finishes, the Job opens a walk, and its drone writes layout.ts while the walk stays open.
  {
    change: (one) => ({
      ...one,
      attachments: [...one.attachments.filter((held) => held.kind !== "subagent"), subagent("done")],
      rows: [...one.rows, { id: "ju-walk", at: stamp(), kind: "window", title: "The Field in use", url: "http://127.0.0.1:5176/?walk=jobFieldInUse" }],
      work: { ...one.work, field: [main(LAYOUT)], files: [{ path: PANEL, act: "edit" }, { path: LAYOUT, act: "edit" }] },
    }),
    events: () => [event("session.walked", 0, { says: "Walked", detail: "The Field in use" }), edited(0, LAYOUT)],
  },
];

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: ARMADA, slot, path: `${ROOT}/.armada/slots/slot-${slot}`, base: "main", warm: true, behind: 0, held, ...rest });

const pool = (): WorktreesHeld => ({ slots: [bay(1, { state: "hosted_session", session_id: ID, session_title: TITLE }, { branch: "armada/field-in-use", since: stamp(600) })] });

/** The files being written now. */
const writing = (one: Session | undefined): string[] => (one?.work?.field ?? []).flatMap((drone) => (drone.doing === "write" && drone.file !== undefined ? [drone.file] : []));

/** The store, with the moments in place of the usual turns, a tool row each second, and a line more on each file being written every few seconds. */
function store(): SessionsStore {
  const inner = sessionsStore([own], [], new Map(), []);
  let at = 0;
  let ticks = 0;
  const grown = new Map<string, number>([[PANEL, 3], [HELPER.file, 2]]);
  const current = () => inner.get().find((one) => one.id === ID);
  const tools = window.setInterval(() => {
    ticks += 1;
    const file = writing(current())[0] ?? READ;
    inner.edit(ID, (one) => ({ ...one, rows: [...one.rows.filter((row: SessionRow) => row.kind !== "tool"), { id: `ju-t${ticks}`, at: stamp(), kind: "tool", text: `Edit ${file}` }] }));
  }, 1000);
  const growth = window.setInterval(() => {
    for (const path of writing(current())) grown.set(path, Math.min(12, (grown.get(path) ?? 0) + 1));
  }, GROW_MS);
  const fileChange = async (id: string, path: string, seen?: string): Promise<FileChangeRead> => {
    const n = grown.get(path);
    if (id !== ID || n === undefined) return { ok: false };
    const stamped = `${path}@${n}`;
    return seen === stamped ? { ok: true, stamp: stamped, diff: "", unchanged: true } : { ok: true, stamp: stamped, diff: diffOf(path, n) };
  };
  return {
    ...inner,
    fileChange,
    later: () => {
      const moment = MOMENTS[at++];
      if (moment === undefined) return;
      inner.edit(ID, moment.change);
      for (const path of writing(current())) if (!grown.has(path)) grown.set(path, 1);
    },
    dispose: () => {
      window.clearInterval(tools);
      window.clearInterval(growth);
      inner.dispose();
    },
  };
}

function build(): Scenario {
  const base = holding("job-field-in-use", "One Job's Field in use: the whole repository, a pinned drone that stops writing, a hover card, and a walk left open while the drone writes");
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(),
    draft: {
      sessions: store,
      events: [event("session.started", 600, { says: "Started" }), edited(4, PANEL)],
      eventsLater: MOMENTS.map((one) => one.events),
      areas: mockAreas,
    },
  };
}

export const s318JobFieldInUse: Scenario = build();
