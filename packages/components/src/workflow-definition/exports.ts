// What reads a workflow definition's text, handed to the app. Named here
// so `index.ts` stays a list of `export *` lines.
export { entriesOf, readDefinition, writeDefinition } from "./json";
export type {
  Definition as WorkflowDefinitionDraft,
  Entry as WorkflowEntry,
  Read as WorkflowRead,
  Saved as WorkflowSavedAnswer,
} from "./def";
export { KIT as WORKFLOW_KIT } from "./def";
