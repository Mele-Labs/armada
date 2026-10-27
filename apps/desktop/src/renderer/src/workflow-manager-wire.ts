// The Workflows page's reading of what Fleet answers: steps as the page draws them, the nudges as drifted Jobs, and the
// changes an agent drafted as the box under the steps and the cards on a step. Nothing here asks Fleet.

import { ageOf, glyphOf, stepsOfDefinition } from "@armada/components";
import type { AgentState, DriftedJob, ManagedStep, Suggestion, WorkflowSource } from "@armada/components";
import type { LeftOutWorkflow, StepNudge, StepReading, WorkflowChange, WorkflowSummary } from "@armada/protocol";

/** Fleet's name for each place the page groups by. */
export const WIRE_OF: Readonly<Record<WorkflowSource, string>> = { armada: "armada", repository: "repository", yours: "kit" };

/** The page's place for Fleet's `armada`, `repository` or `kit`. */
export function placeOf(wire: string): WorkflowSource | undefined {
  return wire === "armada" ? "armada" : wire === "repository" ? "repository" : wire === "kit" ? "yours" : undefined;
}

/** A step Fleet read, as the page draws it. */
export function stepOf(reading: StepReading): ManagedStep {
  const glyph = glyphOf(reading.id);
  return {
    id: reading.id,
    label: reading.label,
    ...(glyph === undefined ? {} : { glyph }),
    ends: reading.ends_when ?? "",
    ...(reading.passed_over_when === undefined ? {} : { passedOver: reading.passed_over_when }),
    words: reading.instructions ?? "",
  };
}

/** The Jobs Fleet nudged at each step, by step id. */
export function driftOf(steps: readonly StepReading[], now: number): Readonly<Record<string, readonly DriftedJob[]>> {
  const jobOf = (one: StepNudge): DriftedJob => ({
    id: String(one.id),
    jobId: one.job_id,
    job: one.job_title ?? one.job_id,
    what: one.fact,
    when: ageOf(Date.parse(one.at), now),
  });
  return Object.fromEntries(steps.filter((one) => (one.nudges ?? []).length > 0).map((one) => [one.id, (one.nudges ?? []).map(jobOf)]));
}

/** A definition's text as steps. A step the workflow already has keeps what Fleet read of it; one it lacks has no end to say. */
export function stepsOfText(text: string, known: readonly ManagedStep[]): ManagedStep[] {
  return (stepsOfDefinition(text) ?? []).map((one) => {
    const had = known.find((each) => each.id === one.id);
    const glyph = glyphOf(one.id);
    return {
      ...(had ?? { id: one.id, ...(glyph === undefined ? {} : { glyph }), ends: "" }),
      label: one.label,
      words: one.instructions ?? "",
    };
  });
}

/** The change he asked for, if one is open: the oldest, since nothing asks again while one stands. */
export const askedChange = (changes: readonly WorkflowChange[]): WorkflowChange | undefined =>
  changes.find((one) => one.step_id === undefined);

/** The box under the steps, from the open changes. */
export function agentOf(changes: readonly WorkflowChange[], known: readonly ManagedStep[]): AgentState {
  const one = askedChange(changes);
  if (one === undefined) return { phase: "idle" };
  const asked = one.origin.kind === "asked" ? { asked: one.origin.text } : {};
  if (one.state === "drafting") return { phase: "working", ...asked };
  if (one.state === "failed") return { phase: "failed", reason: one.reason ?? "", ...asked };
  if (one.state === "ready" && one.after !== undefined) {
    return { phase: "proposed", before: stepsOfText(one.before, known), after: stepsOfText(one.after, known), ...asked };
  }
  return { phase: "idle" };
}

/** The change waiting on a step, if one is open. */
export const changeAt = (changes: readonly WorkflowChange[], stepId: string): WorkflowChange | undefined =>
  changes.find((one) => one.step_id === stepId);

/** The cards on the steps, and the step an agent is drawing one up for (`workflow/step`). */
export function suggestionsOf(
  changes: readonly WorkflowChange[],
  workflow: string,
): { suggestions: Readonly<Record<string, Suggestion>>; suggesting: string | null } {
  const suggestions: Record<string, Suggestion> = {};
  let suggesting: string | null = null;
  for (const one of changes) {
    const stepId = one.step_id;
    if (stepId === undefined || stepId in suggestions) continue;
    const from = one.origin.kind === "drift" ? one.origin.nudges.map(String) : [];
    if (one.state === "drafting") suggesting ??= `${workflow}/${stepId}`;
    else if (one.state === "failed") suggestions[stepId] = { words: "", from, failed: one.reason ?? "" };
    else if (one.state === "ready" && one.after !== undefined) {
      const words = stepsOfDefinition(one.after)?.find((each) => each.id === stepId)?.instructions ?? "";
      suggestions[stepId] = { words, from };
    }
  }
  return { suggestions, suggesting };
}

/** One row of the list: a definition Fleet holds in a place, or a file it left out and why. */
export type Listed = { id: string; source: WorkflowSource; leftOut?: { said: string; file: string }; forRequests?: string };

/**
 * The rows of the list from what Fleet answered: each workflow it runs, a row for each definition of the same id a
 * more specific place replaced, and each file it left out with its sentence. **Kit's wins over the repository's, and
 * that over Armada's**, so the replaced ones are what `overrides` names. A workflow Fleet gives no place for is a
 * Fleet older than the field, and draws nothing.
 */
export function listedOf(workflows: readonly WorkflowSummary[], leftOut: readonly LeftOutWorkflow[]): Listed[] {
  const out: Listed[] = [];
  const seen = new Set<string>();
  const add = (one: Listed) => {
    const key = `${one.source}:${one.id}:${one.leftOut?.file ?? ""}`;
    if (!seen.has(key)) out.push(one);
    seen.add(key);
  };
  for (const one of workflows) {
    const source = placeOf(one.source ?? "");
    if (source === undefined) continue;
    add({ id: one.id, source, ...(one.for_requests === undefined ? {} : { forRequests: one.for_requests }) });
    for (const under of one.overrides ?? []) {
      const lower = placeOf(under.source);
      if (lower !== undefined) add({ id: one.id, source: lower });
    }
  }
  for (const one of leftOut) {
    const source = placeOf(one.source);
    if (source !== undefined) add({ id: one.id ?? one.file.split("/").pop() ?? one.file, source, leftOut: { said: one.said, file: one.file } });
  }
  return out;
}

/** Whether Armada carries a workflow of this id among the rows. */
export const armadaCarries = (rows: readonly Listed[], id: string): boolean =>
  rows.some((one) => one.source === "armada" && one.id === id && one.leftOut === undefined);
