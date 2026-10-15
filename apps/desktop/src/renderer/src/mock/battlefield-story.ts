// A twelve-hour Session built on the battlefield, in nine moments a walk steps through. **Every fact is one Fleet records or would**:
// reads, edits, subagents, questions and pull request stages are timeline entries as they are written today; the task list with its
// ticks (the TodoWrite hook) and a walk shown (`show_window`) are the two Fleet writes no entry for yet, and are stood in here.
// Minutes are from the Session's start; the moment sets the clock it is read at. #2301.

import { repository } from "@armada/screens/src/fixtures/build/base";
import type { TimelineEvent } from "@armada/protocol";

import type { ProgressView, Task } from "../cockpit/deck/progress";

const ROOT = repository().root;
const ARMADA = repository().manifest!.id;
const T0 = Date.parse("2026-10-14T06:00:00Z");
const iso = (minutes: number) => new Date(T0 + minutes * 60_000).toISOString();

/** The Session the field is about, and the plot it writes most. */
export const FEATURED = "bp1";
export const BUILT_ON = "apps/desktop";
/** The moments, as minutes into the Session: scope, plan, implement, quiet ten minutes, quiet forty, the road begun, the road run, landed, a day on. */
const NOW = [50, 130, 342, 350, 380, 540, 575, 590, 590 + 20 * 60];
const LANDED = 584;
const PR = 2310;

let count = 0;
type Rest = { detail?: string; items?: string[]; id?: string };
function entry(id: string, minutes: number, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: Rest = {}): TimelineEvent {
  count += 1;
  const { id: _id, ...more } = rest;
  const subject = kind.startsWith("pr.") ? ({ kind: "pull_request", number: PR } as const) : ({ kind: "session", id } as const);
  return { id: count, at: iso(minutes), kind, subject, tone, says, manifest_id: ARMADA, ...more };
}
const mine = (minutes: number, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest?: Rest) => entry(FEATURED, minutes, kind, tone, says, rest);
const read = (minutes: number, files: string[]) => mine(minutes, "session.read", "run", `Read ${files.length} files`, { detail: files[0]!, items: files });
const edit = (minutes: number, files: string[]) => mine(minutes, "session.edited", "run", `Edited ${files.length} files`, { detail: files[0]!, items: files });
const sub = (minutes: number, says: "Subagent started" | "Subagent finished", detail: string) => mine(minutes, "session.subagent", "run", says, { detail });
const DIR = "apps/desktop/src/renderer/src/cockpit";

const AGENTS = ["Explore: map the dock call sites", "Plan: split the dock panel", "general-purpose: write the dock tests", "Explore: check the token use"];

const EVENTS: TimelineEvent[] = [
  mine(0, "session.started", "run", "Started"),
  // Scope: it reads, a walk of the current dock is shown, two questions are asked.
  read(8, [`${DIR}/Dock.tsx`, `${DIR}/dock.css`]),
  read(14, [`${DIR}/Cockpit.tsx`]),
  mine(25, "session.waiting", "ask", "Walk shown", { detail: "?walk=dockToday" }),
  mine(31, "session.asked", "ask", "Asked: which panel owns the dock?"),
  read(38, [`${DIR}/deck/Deck.tsx`]),
  mine(44, "session.asked", "ask", "Asked: keep the dock keys?"),
  // Plan: more reading, then the list is written at 95.
  read(60, [`${DIR}/deck/keys.tsx`, `${DIR}/deck/store.ts`]),
  read(85, [`${DIR}/deck/panes.tsx`]),
  // Implement. Edits at 140 and 145 are before the first tick, 160 to 200 before the second, 220 before the third, none between the third and fourth.
  edit(140, [`${DIR}/Dock.tsx`]),
  edit(145, [`${DIR}/Dock.tsx`, `${DIR}/dock.css`]),
  edit(160, [`${DIR}/dock/store.ts`]),
  edit(200, [`${DIR}/dock/store.ts`]),
  edit(220, [`${DIR}/dock/Header.tsx`]),
  edit(300, [`${DIR}/dock/Handles.tsx`]),
  edit(330, [`${DIR}/dock/Handles.tsx`, `${DIR}/dock/keys.ts`]),
  edit(340, [`${DIR}/dock/keys.ts`]),
  // Four subagents. Three finish at once after the quiet begins; the last runs on, and ends at 405.
  sub(310, "Subagent started", AGENTS[0]!),
  sub(315, "Subagent started", AGENTS[1]!),
  sub(322, "Subagent started", AGENTS[2]!),
  sub(330, "Subagent started", AGENTS[3]!),
  sub(346, "Subagent finished", AGENTS[0]!),
  sub(347, "Subagent finished", AGENTS[1]!),
  sub(349, "Subagent finished", AGENTS[2]!),
  sub(405, "Subagent finished", AGENTS[3]!),
  // Forty quiet minutes end: it edits again, and finishes.
  edit(392, [`${DIR}/dock/Page.tsx`]),
  edit(398, [`${DIR}/dock/Page.tsx`]),
  edit(460, [`${DIR}/dock/dock.test.tsx`]),
  // The road. A walk of the finished thing, the pull request, its Checks, the auto-merge, the merge.
  mine(500, "session.waiting", "ask", "Walk shown", { detail: "?walk=dock" }),
  entry(FEATURED, 520, "pr.opened", "idle", "Opened", { detail: "armada/dock-panel" }),
  entry(FEATURED, 555, "pr.checks", "run", "Checks running"),
  entry(FEATURED, 570, "pr.queued", "run", "In the queue"),
  entry(FEATURED, LANDED, "pr.merged", "ok", "Merged"),
];

const TASKS: Task[] = [
  { id: "t1", text: "Dock layout", planned: iso(95), ticked: iso(150) },
  { id: "t2", text: "Dock store", planned: iso(95), ticked: iso(205) },
  { id: "t3", text: "Panel header", planned: iso(95), ticked: iso(250) },
  { id: "t4", text: "Drag handles", planned: iso(95), ticked: iso(262) },
  { id: "t5", text: "Keyboard map", planned: iso(95), ticked: iso(400) },
  { id: "t6", text: "Settings page", planned: iso(95), ticked: iso(440) },
  { id: "t7", text: "Dock tests", planned: iso(95), ticked: iso(470) },
];

/** Sessions that landed before: a pull request merged `ago` minutes before the day on, and the plot each built on. */
const OLD: { id: string; tile: string; slot: number; ago: number; hollow?: number }[] = [
  { id: "old1", tile: BUILT_ON, slot: 0, ago: 190 },
  { id: "old2", tile: BUILT_ON, slot: 1, ago: 520, hollow: 3 },
  { id: "old3", tile: "crates/fleet", slot: 0, ago: 900 },
  { id: "old4", tile: "packages/components", slot: 2, ago: 1800 },
];

function earlier(one: (typeof OLD)[number], now: number): ProgressView {
  const landed = (now - one.ago * 60_000 - T0) / 60_000;
  const edits = Array.from({ length: 7 }, (_, i) => entry(one.id, landed - 300 + i * 30, "session.edited", "run", "Edited 1 file", { detail: `${one.tile}/x.ts`, items: [`${one.tile}/x.ts`] }));
  const tasks = Array.from({ length: 7 }, (_, i): Task => ({ id: `${one.id}${i}`, text: `Task ${i + 1}`, planned: iso(landed - 320), ticked: iso(landed - 300 + i * 30 + (i === one.hollow ? -12 : 4)) }));
  return { key: `session:${one.id}`, root: ROOT, tile: one.tile, slot: one.slot, running: false, now, events: [...edits, entry(one.id, landed - 20, "pr.opened", "idle", "Opened"), entry(one.id, landed, "pr.merged", "ok", "Merged")], tasks };
}

/** The views at moment `moment`: the Session this walk is about, and on the day on, the structures that stand from before. */
export function storyAt(moment: number): readonly ProgressView[] {
  const now = T0 + NOW[Math.min(moment, NOW.length - 1)]! * 60_000;
  const featured: ProgressView = { key: `session:${FEATURED}`, root: ROOT, tile: BUILT_ON, running: moment < 5, now, events: EVENTS, tasks: TASKS };
  return moment < 8 ? [featured] : [featured, ...OLD.map((one) => earlier(one, now))];
}
