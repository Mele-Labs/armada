// The lifecycle reads, answered empty: no scenario holds a Workspace, a lifecycle, a rule or a Job's instance yet, and nothing
// draws them. A Job that holds nothing is what Fleet answers too. The event never fires.

import type { LifecyclesApi } from "../../../../shared/api/lifecycles";
import type { Slice } from "../fake-context";

export const lifecycles: Slice<LifecyclesApi, Record<string, never>> = {
  name: "lifecycles",
  state: {},
  api: () => ({
    listRepositoryWorkspaces: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, workspaces: [], left_out: [] } }),
    listLifecycles: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, lifecycles: [], left_out: [] } }),
    listLifecycleRules: async (manifestId) => ({ ok: true, value: { manifest_id: manifestId, rules: [], left_out: [] } }),
    getJobLifecycles: async (sessionId) => ({ ok: true, value: { session_id: sessionId, lifecycles: [], instances: [], moves: [] } }),
    onJobLifecyclesChanged: () => () => {},
  }),
};
