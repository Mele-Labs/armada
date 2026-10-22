// The `sessions` scenario with its Jobs on a workflow, the two already open and each one Dispatch starts, as every Job Fleet hosts
// is, so each opens as a Job does on the Cockpit: what it made or used in the Artifacts layer, its slot in the Bays layer. The walk
// `sessions` plays it.

import { IMPLEMENTING, onAWorkflow } from "../sessions/on-a-workflow";
import { s200Sessions } from "./sessions";

const opened = onAWorkflow(s200Sessions, "sessions-on-a-workflow", "Sessions beside Helm, each a Job on a workflow: a blank one, its first write, and what it accumulates", ["s2", "s3"]);

export const s319SessionsOnAWorkflow = {
  ...opened,
  draft: {
    ...opened.draft,
    sessions: (board, more) => {
      const store = opened.draft!.sessions!(board, more);
      const start = store.start;
      store.start = (...args) => {
        const id = start(...args);
        // The mock's start answers at once; a Promise is Fleet's.
        if (typeof id === "string") store.edit(id, (one) => ({ ...one, work: IMPLEMENTING }));
        return id;
      };
      return store;
    },
  },
} satisfies typeof opened;
