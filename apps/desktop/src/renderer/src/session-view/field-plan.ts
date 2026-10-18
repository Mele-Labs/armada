// The Session's plan as the Field reads it: one piece for each task, in the language the map card's plan bar already has (solid is done
// with edits seen, outlined is done with none seen, amber is working, faint is open), and one piece for each plot a group names.
// Pure. Nothing here draws.

import type { WorkGroup, WorkPlan, WorkTask } from "@armada/screens/src/draft/sessions";

import type { PlotPiece } from "../cockpit/deck/pixi/scene";

export type PlanPiece = PlotPiece;

/** What a piece says of its task, as the plan bar's tooltip words it. */
export const PIECE_SAID: Record<PlanPiece, string> = { solid: "Done, edits seen", hollow: "Done, no edits seen", scaffold: "Working", ghost: "Open", failed: "Failed" };

export type FieldTask = { id: string; text: string; piece: PlanPiece };
export type FieldGroup = { id: string; title: string; paths: readonly string[]; tasks: readonly FieldTask[]; piece: PlanPiece };

/** A task the agent's list marks completed is done, whether or not Fleet saw an edit for it. */
const done = (task: WorkTask) => task.state === "done" || task.state === "handed_in";

/** One task's piece. A task handed in is done with no edit seen, so it stands hollow. */
export function pieceOfTask(task: WorkTask): PlanPiece {
  if (task.state === "failed") return "failed";
  if (done(task)) return task.seen === true ? "solid" : "hollow";
  return task.state === "open" ? "ghost" : "scaffold";
}

/** The piece of a set of tasks: whole when all are done, amber once any has started, faint while none has. */
export function pieceOfTasks(tasks: readonly WorkTask[]): PlanPiece {
  if (tasks.some((task) => task.state === "failed")) return "failed";
  if (tasks.every(done)) return tasks.some((task) => task.seen === true) ? "solid" : "hollow";
  return tasks.some((task) => task.state !== "open") ? "scaffold" : "ghost";
}

const groupOf = (group: WorkGroup): FieldGroup => ({
  id: group.id,
  title: group.title,
  paths: group.paths,
  tasks: group.tasks.map((task) => ({ id: task.id, text: task.text, piece: pieceOfTask(task) })),
  piece: pieceOfTasks(group.tasks),
});

/** The plan's groups in order, each with a piece for every task. A group with no tasks is not drawn. */
export const groupsOf = (plan: WorkPlan): FieldGroup[] => plan.groups.filter((group) => group.tasks.length > 0).map(groupOf);

/** What a plot says of the plan: the groups that name it, and the piece their tasks make together. */
export type PlotPlan = { piece: PlanPiece; said: string };

/**
 * The plots the plan names, by path. A group names a plot when one of its paths is the plot or lies inside it. Where two groups
 * name one plot, the plot stands as their tasks together do.
 */
export function plotsOf(plan: WorkPlan, plots: readonly string[]): Map<string, PlotPlan> {
  const out = new Map<string, PlotPlan>();
  for (const plot of plots) {
    const named = plan.groups.filter((group) => group.tasks.length > 0 && group.paths.some((path) => path === plot || path.startsWith(`${plot}/`)));
    if (named.length === 0) continue;
    const piece = pieceOfTasks(named.flatMap((group) => group.tasks));
    out.set(plot, { piece, said: `${named.map((group) => group.title).join(", ")} · ${PIECE_SAID[piece]}` });
  }
  return out;
}
