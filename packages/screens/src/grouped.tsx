// The activity log, with runs of one tool folded to a line.
//
// **The grouping is derived from the turns and drawn through the log.** What a
// row is, how it opens and which keyboard reaches it stay `Log.tsx`'s — this
// only decides which rows belong together and what the folded line says. So a
// group's body is the same component the log draws unfolded, and the region and
// payload names `detail-keys.ts` reads are untouched.
//
// **The ids line up because both sides use the socket's sequence.** A
// `WorkingAct` is named `String(turn.seq)` and so is a `LogRow`, which is what
// lets a run select its own rows without a second pass over the wire.
import { ToolName, WorkGroups, type WorkGroup } from "@armada/components";
import type { Turn } from "@armada/protocol";

import type { Calls } from "./calls";
import type { DetailKeys } from "./detail-keys";
import { briefly } from "./duration";
import { Log } from "./Log";
import type { LogRow } from "./story";
import { runsOf, workingOf, type WorkingRun } from "./working";

/**
 * The whole activity log, runs of one tool folded to a line — the sheet behind
 * *Open the log*.
 *
 * **It draws every row it is handed, and it is the one surface that does.** It
 * once carried a bound in groups for the chapter's preview. The preview draws
 * no rows now, so a bound here would answer a question this surface is not
 * asked: it is where everything is read.
 */
export function WorkGrouped({
  rows,
  turns,
  stepId,
  unread,
  emptyNote,
  calls: fetched,
  log,
}: {
  /** The step's readable rows, in order — what the log would draw flat. */
  rows: LogRow[];
  /** The same turns those rows came from, which carry the tool and the timing. */
  turns: readonly Turn[];
  stepId: string;
  /** How many rows this Bridge has no drawing for, by the wire's own kind. */
  unread?: { kind: string; count: number }[];
  emptyNote: string;
  calls: Calls;
  /** The log's own keyboard binding, by region. Threaded to every group. */
  log: ReturnType<DetailKeys["inLog"]>;
}) {
  // **Which run each row belongs to**, the call's row and its answer's alike.
  // The derivation folds an answer into its call to count one thing that
  // happened; a surface drawing the transcript still has to draw both.
  const runOf = new Map<string, WorkingRun>();
  const calls = new Set<string>();
  for (const run of runsOf(workingOf(turns, stepId).acts)) {
    for (const act of run.acts) {
      runOf.set(act.id, run);
      if (act.kind === "call") calls.add(act.id);
      if (act.answeredId !== undefined) runOf.set(act.answeredId, run);
    }
  }

  // **Walked in row order, and the run is what breaks it.** Selecting rows by
  // the groups would silently drop any row no group claimed — an `unreadable`
  // line today, and whatever the wire adds next. Walking the rows cannot: a
  // row belonging to no run becomes a chunk that draws bare, and every row is
  // drawn exactly once, in the order it arrived.
  const chunks: { run?: WorkingRun; rows: LogRow[] }[] = [];
  for (const row of rows) {
    const run = runOf.get(row.id);
    const last = chunks[chunks.length - 1];
    if (last !== undefined && last.run === run) last.rows.push(row);
    else chunks.push({ ...(run === undefined ? {} : { run }), rows: [row] });
  }

  const groups: WorkGroup[] = chunks.map((chunk) => {
    // **Counted in calls, and only the ones in hand.** The sheet filters its
    // rows by actor, so a run of nine can arrive with two of its calls drawn —
    // and a heading counting the run rather than what is under it would say
    // nine above two rows.
    const held = chunk.rows.filter((row) => calls.has(row.id)).length;
    const first = chunk.rows[0];
    return {
      id: first === undefined ? "" : first.id,
      // The tool's own colour, the same one its rows carry underneath: a
      // folded run is the rows it hides, and the two reading differently is
      // the fold changing what a person sees. #1196.
      name: chunk.run?.tool === undefined ? "" : <ToolName tool={chunk.run.tool} />,
      mono: true,
      meta: `${held} ${held === 1 ? "call" : "calls"} · ${briefly(chunk.run?.ms ?? 0)}`,
      // Folded where the derivation says so, and never over a single call: a
      // heading over one row hides nothing and costs a press.
      folded: chunk.run?.folded === true && held > 1,
      body: <Log rows={chunk.rows} emptyNote={emptyNote} calls={fetched} {...log} />,
    };
  });

  return (
    <WorkGroups groups={groups} {...(unread === undefined ? {} : { unread })} emptyNote={emptyNote} />
  );
}
