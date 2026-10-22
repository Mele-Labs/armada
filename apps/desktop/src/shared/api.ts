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
import type { DispatchApi } from "./api/dispatch";
import type { ChecksApi } from "./api/checks";
import type { LessonsApi } from "./api/lessons";
import type { SessionsApi } from "./api/sessions";
import type { SleepApi } from "./api/sleep";
import type { AccountsApi } from "./api/accounts";
import type { PlanUsageApi } from "./api/plan-usage";
import type { ModelDownshiftApi } from "./api/model-downshift";
import type { ClashesApi } from "./api/clashes";
import type { ReviewsApi } from "./api/reviews";
import type { TerritoryApi } from "./api/territory";
import type { TimelineApi } from "./api/timeline";
import type { BayHistoryApi } from "./api/bay-history";
import type { UsageSummaryApi } from "./api/usage-summary";
import type { IssuesApi } from "./api/issues";
import type { AutopilotApi } from "./api/autopilot";
import type { SketchShareApi } from "./api/sketch-share";
import type { FilesApi } from "./api/files";
import type { FlightControlsApi } from "./api/flight-controls";
import type { OnboardingApi } from "./api/onboarding";
import type { RequestsApi } from "./api/requests";
import type { SharingApi } from "./api/sharing";
import type { BridgeState } from "./bridge";

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
  DispatchApi &
  ChecksApi &
  LessonsApi &
  SessionsApi &
  SleepApi &
  AccountsApi &
  PlanUsageApi &
  ModelDownshiftApi &
  ClashesApi &
  ReviewsApi &
  TerritoryApi &
  TimelineApi &
  BayHistoryApi &
  UsageSummaryApi &
  IssuesApi &
  AutopilotApi &
  SketchShareApi &
  FilesApi &
  FlightControlsApi &
  OnboardingApi &
  RequestsApi &
  SharingApi;
