// One Job writing on its Field for the walk `jobFieldLens`, a moment for each `later` step: its drone writes layout.ts, then
// FieldPanel.tsx, then three subagents write a file each at once, and then they finish. The draft answers `get_session_file_change`
// for each file the Job has been seen writing with a diff that grows a line every few seconds while it is being written, and refuses any
// other path, as Fleet does. The `session.edited` entries are dated when each moment comes.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { TimelineEvent, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { FileChangeRead, Session, SessionAttachment, SessionRow, WorkDrone } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const ROOT = repository().root;
const LAYOUT = "apps/desktop/src/renderer/src/cockpit/deck/layout.ts";
const PANEL = "apps/desktop/src/renderer/src/session-view/FieldPanel.tsx";
const ID = "jf-lens";
const TITLE = "Show the change on the Field";

/** The three subagents that write at once: their id, type, task and the file each writes. */
const HELPERS = [
  { id: "sa-wire", agent: "Explore", task: "Check the wire types", file: "packages/protocol/src/wire.ts", area: "packages/protocol" },
  { id: "sa-docs", agent: "Docs", task: "Write the lens into the docs", file: "docs/concepts/cockpit.md", area: "docs/concepts" },
  { id: "sa-store", agent: "Rust", task: "Keep the stamp in the store", file: "crates/store/src/areas.rs", area: "crates/store" },
] as const;

/** How often a file being written grows a line. */
const GROW_MS = 2_500;

const stamp = (secondsAgo = 0) => new Date(Date.now() - secondsAgo * 1000).toISOString();

const main = (file: string): WorkDrone => ({ id: "main", path: file.split("/").slice(0, 2).join("/"), doing: "write", file });
const helpers = (): WorkDrone[] => HELPERS.map((one) => ({ id: one.id, agent: one.agent, path: one.area, doing: "write", file: one.file }));

const own: Session = {
  id: ID,
  title: TITLE,
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: stamp(60),
  standing: "working",
  rows: [
    { id: "jl-u", at: stamp(300), kind: "message", from: { kind: "you" }, text: "Lift the file a drone is writing and show its change on a card." },
    { id: "jl-t0", at: stamp(1), kind: "tool", text: `Edit ${LAYOUT}` },
  ],
  attachments: [{ kind: "branch", name: "armada/field-lens", slot: 1 }],
  work: { field: [main(LAYOUT)], files: [{ path: LAYOUT, act: "edit" }] },
};

let made = 0;
const event = (kind: TimelineEvent["kind"], secondsAgo: number, extra: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id: 2000 + ++made,
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

/** The lines a file's growing hunk adds, by its kind, in turn. */
const LINES: Readonly<Record<string, readonly string[]>> = {
  ts: ["  const lifted = writersOf(was, now, at);", "  const card = cardOf(lifted, pinned);", "  if (card === undefined) return undefined;", "  const diff = kept.get(card.path);", "  const rows = foldRows(diff.rows, SMALL_ROWS);", "  return { card, rows };"],
  tsx: ["      <FieldLensCard card={lens.card} large={large} />", "      {lens.slabs.map((one) => (", "        <LensCount key={one.path} slab={one} />", "      ))}", "      <LensWires stage={stage} />", "    </div>"],
  md: ["The file a drone is writing lifts off the floor as a slab.", "A card at the top right types out its newest hunk.", "Earlier files stack behind it as tabs.", "Other writers lift smaller, with their counts.", "A press in the strip pins the card.", ""],
  rs: ["    pub stamp: String,", "    pub diff: String,", "    #[serde(default)]", "    pub cut: bool,", "    pub unchanged: bool,", "}"],
};

/** A file's change after `grown` lines of its second hunk: a line changed near the top, and a hunk further down that grows. */
function diffOf(path: string, grown: number): string {
  const ext = path.split(".").pop() ?? "ts";
  const lines = LINES[ext] ?? LINES.ts!;
  const added = Array.from({ length: grown }, (_, k) => `+${lines[k % lines.length]}`);
  const comment = ext === "md" ? "" : "// ";
  return [
    `diff --git a/${path} b/${path}`,
    `--- a/${path}`,
    `+++ b/${path}`,
    "@@ -8,3 +8,3 @@",
    ` ${comment}What the Field draws.`,
    `-${comment}Only the touched folders.`,
    `+${comment}The whole repository.`,
    `@@ -40,2 +40,${2 + grown} @@`,
    ` ${comment}Lifted files.`,
    ...added,
    ` ${comment}End.`,
    "",
  ].join("\n");
}

/** What each moment does to the Job, and the entries Fleet writes as it comes. */
type Moment = { change: (one: Session) => Session; events: () => TimelineEvent[] };

const subagents = (state: "running" | "done"): SessionAttachment[] =>
  HELPERS.map((one) => ({ kind: "subagent", id: one.id, task: one.task, state, agentType: one.agent, ...(state === "done" ? { report: "Written." } : {}) }));

const MOMENTS: readonly Moment[] = [
  // The drone moves on to FieldPanel.tsx.
  { change: (one) => ({ ...one, work: { ...one.work, field: [main(PANEL)], files: [{ path: LAYOUT, act: "edit" }, { path: PANEL, act: "edit" }] } }), events: () => [edited(0, PANEL)] },
  // Three subagents start, each writing a file, at once.
  {
    change: (one) => ({ ...one, attachments: [...one.attachments, ...subagents("running")], work: { ...one.work, field: [main(PANEL), ...helpers()] } }),
    events: () => HELPERS.map((one) => event("session.subagent", 0, { says: "Subagent started", detail: `${one.agent}: ${one.task}` })),
  },
  // They finish; the Job's drone still writes FieldPanel.tsx.
  {
    change: (one) => ({ ...one, attachments: [...one.attachments.filter((held) => held.kind !== "subagent"), ...subagents("done")], work: { ...one.work, field: [main(PANEL)] } }),
    events: () => HELPERS.map((one) => event("session.subagent", 0, { says: "Subagent finished", detail: `${one.agent}: ${one.task}` })),
  },
];

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: ARMADA, slot, path: `${ROOT}/.armada/slots/slot-${slot}`, base: "main", warm: true, behind: 0, held, ...rest });

const pool = (): WorktreesHeld => ({ slots: [bay(1, { state: "hosted_session", session_id: ID, session_title: TITLE }, { branch: "armada/field-lens", since: stamp(600) })] });

/** The files being written now: the Job's drone's and each running subagent's. */
const writing = (one: Session | undefined): string[] => (one?.work?.field ?? []).flatMap((drone) => (drone.doing === "write" && drone.file !== undefined ? [drone.file] : []));

/** The store, with the moments in place of the usual turns, a tool row each second, and a line more on each file being written every few seconds. */
function store(): SessionsStore {
  const inner = sessionsStore([own], [], new Map(), []);
  let at = 0;
  let ticks = 0;
  const grown = new Map<string, number>([[LAYOUT, 3]]);
  const current = () => inner.get().find((one) => one.id === ID);
  const tools = window.setInterval(() => {
    ticks += 1;
    const file = writing(current())[0] ?? LAYOUT;
    inner.edit(ID, (one) => ({ ...one, rows: [...one.rows.filter((row: SessionRow) => row.kind !== "tool"), { id: `jl-t${ticks}`, at: stamp(), kind: "tool", text: `Edit ${file}` }] }));
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
  const base = holding("job-field-lens", "One Job writing on its Field: its drone writes two files in turn, then three subagents write at once, each file lifted with its change");
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(),
    draft: {
      sessions: store,
      events: [event("session.started", 600, { says: "Started" }), edited(4, LAYOUT)],
      eventsLater: MOMENTS.map((one) => one.events),
      areas: mockAreas,
    },
  };
}

export const s316JobFieldLens: Scenario = build();
