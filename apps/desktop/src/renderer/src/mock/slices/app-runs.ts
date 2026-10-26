// App runs' members: the wire is answered by Fleet alone, so the mock has none for it. The walks' runs are `mock/runs.ts`, handed to the
// window as its source of runs rather than through the bridge.

import type { AppRunsApi } from "../../../../shared/api/app-runs";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const appRuns: Slice<AppRunsApi, Record<string, never>> = {
  name: "appRuns",
  state: {},
  api: () => ({
    listAppRuns: async () => ({ ok: false, outcome: unanswered("/app_runs") }),
    getAppRunDraft: async () => ({ ok: false, outcome: unanswered("/app_runs/draft") }),
    actOnAppRun: async (act) => ({ ok: false, outcome: unanswered(`/app_runs/${act}`) }),
    saveAppRun: async () => ({ ok: false, outcome: unanswered("/app_runs/save") }),
    pinAppRunPort: async () => ({ ok: false, outcome: unanswered("/app_runs/pin") }),
    unpinAppRunPort: async () => ({ ok: false, outcome: unanswered("/app_runs/unpin") }),
    onAppRunChanged: () => () => {},
    followServer: async (serverId) => unanswered(`/servers/${serverId}/observe`),
    unfollowServer: () => {},
    onServerOutput: () => () => {},
  }),
};
