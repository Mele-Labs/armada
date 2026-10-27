// A workflow definition's text, read for the Workflows page and changed in the one place it edits: a step's
// `instructions`. `workflow-definition/json.ts` reads the text as JSON or YAML; nothing else in the file is touched
// here, so what goes back to `save_workflow` is the file as it was but for those words.

import { parse } from "../../workflow-definition/json";

type Table = Record<string, unknown>;

const table = (value: unknown): Table =>
  typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Table) : {};
const word = (value: unknown): string | undefined => (typeof value === "string" ? value : undefined);

/** One step as its file wrote it, as far as the page draws it. */
export type DefinitionStep = { id: string; label: string; instructions?: string };

function treeOf(text: string): Table | undefined {
  try {
    const top = table(parse(text));
    return Array.isArray(top.steps) ? top : undefined;
  } catch {
    return undefined;
  }
}

/** The steps of a definition in order, or none where the text is not a workflow. A step with no label is named by its id. */
export function stepsOfDefinition(text: string): DefinitionStep[] | undefined {
  const tree = treeOf(text);
  if (tree === undefined) return undefined;
  return (tree.steps as unknown[]).map((raw) => {
    const step = table(raw);
    const id = word(step.id) ?? "";
    const instructions = word(step.instructions);
    return { id, label: word(step.label) ?? id, ...(instructions === undefined ? {} : { instructions }) };
  });
}

/** The line a definition says it is for, where it says one. */
export function forRequestsOf(text: string): string | undefined {
  return word(treeOf(text)?.for_requests);
}

/** The definition with one step's words replaced and every other key as it was, or none where the step is not there. */
export function withWords(text: string, stepId: string, words: string): string | undefined {
  const tree = treeOf(text);
  if (tree === undefined) return undefined;
  const steps = (tree.steps as unknown[]).map(table);
  const at = steps.findIndex((one) => one.id === stepId);
  if (at === -1) return undefined;
  steps[at] = { ...steps[at], instructions: words };
  return JSON.stringify({ ...tree, steps }, null, 2);
}
