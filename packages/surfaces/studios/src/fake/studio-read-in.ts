// A Studio holding one issue read in, as the owner's own read of #1657 came
// back on 2 Oct 2026 — a Finding, twelve Notes and a Contradiction — laid out
// the way Fleet now lands one: everything inside one Zone, the Finding first and
// each Cluster round its Notes (#1620), with the scout's one-Note Cluster dropped
// and its Note loose.
//
// **The words are his read-in's, as the scout wrote them**, so what the walk
// shows is what he saw as eighteen loose cards, not an invented board.

import type { Studio, StudioEdge, StudioNode } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { studying, type StudioFleet, type StudyingApi, type StudyingState } from "./studio-fleet";

const AT = "2026-10-02T04:20:00Z";

/** Fleet's `framing::INSET` and `HEAD`, and the read-in's column and row pitch. */
const INSET = 24;
const HEAD = 48;
const ACROSS = 340;
const DOWN = 180;

export const READ_IN_STUDIO = "01STUDIOREADIN1657000000000";
export const READ_IN_NAME = "Editing a task, read in from #1657";

const ISSUE = "read-in-issue";
const ZONE = "read-in-zone";

/** The scout's Clusters, each with the Notes it named. */
const CLUSTERS: readonly { id: string; title: string; notes: readonly string[] }[] = [
  {
    id: "read-in-problem",
    title: "The problem today",
    notes: [
      "A person cannot change a task's title, brief, files, done-when or model. The task panel's \"Edit this task\" form raises `Not implemented` and names this issue.",
      "This leaves the planning Drone as the only way to change a task, through a rewrite message sent with `redirect_drone`. That works only before the plan's gate, while the plan step waits on a person.",
      "Fleet serves `add_task` and `drop_task` to a person but nothing that edits an existing task. `update_task` is the Drone's MCP tool and writes only `task, state, reason, shown`.",
      "The owner found this driving `arc/group-failed` on 29 Sep 2026. He wants to \"modify anything on the task to help make the task more clear when the drone retries it\".",
    ],
  },
  {
    id: "read-in-build",
    title: "What to build",
    notes: [
      "Model is part of the edit (owner decision, 30 Sep 2026). A person picks a task's model directly, overriding the Job's tier map for that task.",
      "Tasks must be editable before the gate too, while a plan waits on a person, not only after a task fails.",
      "Build a person route `POST /jobs/{job_id}/tasks/{task_id}/edit`. It changes title, brief (`note`), files (`scope`) and `expects` in the recorded plan, and records who changed them.",
      "The next agent to pick up the task reads the edited task.",
      "When the route ships, its entry in `packages/protocol/src/pending.ts` is deleted.",
    ],
  },
  {
    id: "read-in-risks",
    title: "Risks to watch",
    notes: [
      "Watch for an edit after the gate. It changes a plan a Judge has already read. The record must show the plan as approved and the task as edited after it, without rewriting history.",
      "Watch for a `scope` change. It can move which group boundary a test runs at (`touchedByOf`).",
    ],
  },
];

/**
 * The scout's fourth Cluster, "Acceptance", named this one Note. **Fleet draws no Cluster round
 * fewer than two** (the owner, 2 Oct 2026), so it lands loose in the Zone, after the last Cluster.
 */
const ACCEPTANCE = "read-in-acceptance-note-1";

const noteId = (cluster: string, at: number) => `${cluster}-note-${at + 1}`;

const produced = (from: string, to: string): StudioEdge => ({
  id: `${from}>${to}`,
  from,
  to,
  kind: "produced",
  standing: "accepted",
  created_at: AT,
});

/** The read-in, landed in its Zone. A loose Note beside it is the person's own, to drop into it. */
export function readInOf1657(): Studio {
  const nodes: StudioNode[] = [
    {
      id: ISSUE,
      kind: "issue",
      address: "https://example.invalid/o/r/issues/1657",
      number: "1657",
      title: "Nobody can change a task's title, brief, files, done-when or model",
      state: "open",
      position: { x: 0, y: 0 },
      created_at: AT,
    },
    { id: ZONE, kind: "zone", position: { x: ACROSS, y: 0 }, created_at: AT },
    {
      id: "read-in-finding",
      kind: "finding",
      asked: "Read in https://example.invalid/o/r/issues/1657",
      sources: [{ address: "https://example.invalid/o/r/issues/1657", kind: "issue", cut: 0 }],
      learned: "The source's claims largely hold in the checkout. The one gap is a stale line citation.",
      ended: { outcome: "answered", cost_micros: 41_000 },
      within: ZONE,
      position: { x: INSET, y: HEAD },
      created_at: AT,
    },
    ...CLUSTERS.flatMap(({ id, title, notes }, column): StudioNode[] => [
      { id, kind: "cluster", title, within: ZONE, position: { x: INSET + (column + 1) * ACROSS, y: HEAD }, created_at: AT },
      ...notes.map(
        (said, at): StudioNode => ({
          id: noteId(id, at),
          kind: "note",
          said,
          within: id,
          position: { x: INSET, y: HEAD + at * DOWN },
          created_at: AT,
        }),
      ),
    ]),
    {
      id: ACCEPTANCE,
      kind: "note",
      said: "Done when, on a failed task, a person edits its brief in the Plan tab, restarts it (#1656), and the new agent is told the edited brief.",
      within: ZONE,
      position: { x: INSET + (CLUSTERS.length + 1) * ACROSS, y: HEAD },
      created_at: AT,
    },
    {
      id: "read-in-contradiction",
      kind: "contradiction",
      first: "Fleet serves `add_task` and `drop_task` to a person (`crates/fleet/src/commanding.rs:987-994`)",
      second: "`add_task` and `drop_task` are at lines 1020-1026. The claim holds but the citation is stale.",
      state: "reported",
      within: ZONE,
      position: { x: INSET + (CLUSTERS.length + 1) * ACROSS, y: HEAD + DOWN },
      created_at: AT,
    },
    {
      id: "read-in-loose",
      kind: "note",
      said: "Ask whether a Judge re-reads a plan edited after its gate",
      position: { x: 0, y: 2 * ACROSS },
      created_at: AT,
      added_by: "person",
    },
  ];
  const made = nodes.filter((node) => node.id !== ISSUE && node.id !== "read-in-loose");
  const members = CLUSTERS.flatMap(({ id, notes }) => notes.map((_, at) => produced(noteId(id, at), id)));
  return {
    id: READ_IN_STUDIO,
    manifest_id: repository().manifest!.id,
    name: READ_IN_NAME,
    created_at: AT,
    touched_at: AT,
    nodes,
    edges: [
      // The record keeps every node the read-in made produced by the Issue,
      // the Zone among them; the board draws the one line to the Zone.
      ...made.map((node) => produced(ISSUE, node.id)),
      ...members,
      {
        id: "read-in-blocks",
        from: ACCEPTANCE,
        to: noteId("read-in-build", 3),
        kind: "blocks",
        standing: "proposed",
        created_at: AT,
        added_by: "helm",
      },
    ],
  };
}

/** The `studio-zone` scenario: one Studio, the #1657 read-in landed in its Zone. */
export function zoning<S extends StudyingState, A extends StudyingApi>(nothingYet: S): StudioFleet<S, A> {
  const fleet = studying<S, A>(nothingYet, [readInOf1657()]);
  return {
    ...fleet,
    scenario: {
      ...fleet.scenario,
      name: "studio-zone",
      says: "An issue read in, landed in one Zone with each Cluster round its Notes",
    },
  };
}
