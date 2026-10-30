// The lifecycle reads. No scenario holds a Workspace, a lifecycle or a rule yet, so those answer empty. **A Job answers what the scenario serves of it**
// (`workspaces-lanes-wire.ts`), the lifecycles, instances and moves a real Job's read carries, and a Job that holds nothing answers three empty lists, as Fleet does.
// The event fires when a served Job moves.

import type { LifecyclesApi } from "../../../../shared/api/lifecycles";
import type { Slice } from "../fake-context";
import { servedLifecycles } from "../workspaces-lanes-wire";

export const lifecycles: Slice<LifecyclesApi, Record<string, never>> = {
  name: "lifecycles",
  state: {},
  api: () => ({
    listRepositoryWorkspaces: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, workspaces: [], left_out: [] } }),
    listLifecycles: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, lifecycles: [], left_out: [] } }),
    listLifecycleRules: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, rules: [], left_out: [] } }),
    getJobLifecycles: async (sessionId) => ({ ok: true, value: servedLifecycles.get(sessionId) ?? { session_id: sessionId, lifecycles: [], instances: [], moves: [] } }),
    onJobLifecyclesChanged: (onChanged) => servedLifecycles.listen(onChanged),
  }),
};
