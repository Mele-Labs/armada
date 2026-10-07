// The whole preload surface: every capability the renderer may reach.
//
// **Split out of `bridge.ts`**, which held the state, the capabilities and the
// channel names in one file at 897 lines. `BridgeState` and `NOTHING_YET`
// answer *what the renderer reads*; this answers *what the renderer may ask
// for* — a different question, and the file's own size was the two questions
// sharing one answer.
//
// **Composed from the slices in `api/`**, one file per surface. Each slice is
// `<X>Api`, `<X>State`, its empty state and its channel names, and imports no
// other slice. This file and `bridge.ts` keep the names every importer reads.

import type { CoreApi } from "./api/core";
import type { StudiosApi } from "./api/studios";
import type { ManifestApi } from "./api/manifest";
import type { SetupApi } from "./api/setup";
import type { TriggersApi } from "./api/triggers";
import type { WorkflowsApi } from "./api/workflows";
import type { HelmApi } from "./api/helm";
import type { SettingsApi } from "./api/settings";
import type { OverviewApi } from "./api/overview";
import type { CleanupApi } from "./api/cleanup";
import type { ReportsApi } from "./api/reports";
import type { JobsApi } from "./api/jobs";
import type { SessionsApi } from "./api/sessions";
import type { BridgeState } from "./bridge";

/**
 * What `explain_command` came back as. **Protocol's, beside `CheckOutputRead`**, and
 * re-exported here because this is the surface the renderer reads.
 */
export type { CommandExplainedRead } from "@armada/protocol";

/** The whole preload surface, and therefore everything the renderer can reach. */
export type BridgeApi = CoreApi<BridgeState> &
  StudiosApi &
  ManifestApi &
  SetupApi &
  WorkflowsApi &
  TriggersApi &
  HelmApi &
  SettingsApi &
  OverviewApi &
  CleanupApi &
  ReportsApi &
  JobsApi &
  SessionsApi;
