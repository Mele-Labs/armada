// What the Workflow creator hands the app beyond the component itself. Named here
// so `index.ts` stays a list of `export *` lines.
export {
  leftOutRowsOf,
  MOCK_FILES,
  workflowRowsOf,
} from "./mock";
export type { MockFile } from "./mock";
export { entriesOf, readDefinition, writeDefinition } from "./json";
export type {
  Definition as WorkflowDefinitionDraft,
  Entry as WorkflowEntry,
  Read as WorkflowRead,
  Saved as WorkflowSavedAnswer,
} from "./def";
export { KIT as WORKFLOW_KIT } from "./def";
