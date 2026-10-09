// What the state main publishes is, and the channel names it and every
// capability travel over.
//
// **The capabilities are `api.ts`, split out at 897 lines.** This file kept
// growing because a state field and a capability answer different questions
// and one file was answering both; `BridgeApi` is what the renderer may ask
// for, and what is left here is what it reads without asking.
//
// Nothing here runs in more than one process — the preload is a wire and not an
// import path, and this file is the shape of what crosses it.

import type { CheckoutRunFollowed, CheckoutRunSheetRead, ManifestDriftRead } from "@armada/protocol";
import type { LeftOutWorkflow, ManifestReading } from "@armada/protocol";
import type { HealthRead } from "@armada/screens/src/overview-reads";
import { spoken } from "@armada/protocol";
import { CORE_CHANNELS, CORE_NOTHING_YET } from "./api/core";
import type { CoreState } from "./api/core";
import { STUDIOS_CHANNELS, STUDIOS_NOTHING_YET } from "./api/studios";
import type { StudiosState } from "./api/studios";
import { MANIFEST_CHANNELS, MANIFEST_NOTHING_YET } from "./api/manifest";
import type { ManifestState } from "./api/manifest";
import { SETUP_CHANNELS, SETUP_NOTHING_YET } from "./api/setup";
import type { SetupState } from "./api/setup";
import { ADDED_STEPS_CHANNELS, ADDED_STEPS_NOTHING_YET } from "./api/added-steps";
import type { AddedStepsState } from "./api/added-steps";
import { TRIGGERS_CHANNELS, TRIGGERS_NOTHING_YET } from "./api/triggers";
import type { TriggersState } from "./api/triggers";
import { WORKFLOWS_CHANNELS, WORKFLOWS_NOTHING_YET } from "./api/workflows";
import type { WorkflowsState } from "./api/workflows";
import { HELM_CHANNELS, HELM_NOTHING_YET } from "./api/helm";
import type { HelmState } from "./api/helm";
import { SETTINGS_CHANNELS, SETTINGS_NOTHING_YET } from "./api/settings";
import type { SettingsState } from "./api/settings";
import { OVERVIEW_CHANNELS, OVERVIEW_NOTHING_YET } from "./api/overview";
import type { OverviewState } from "./api/overview";
import { CLEANUP_CHANNELS, CLEANUP_NOTHING_YET } from "./api/cleanup";
import type { CleanupState } from "./api/cleanup";
import { REPORTS_CHANNELS, REPORTS_NOTHING_YET } from "./api/reports";
import type { ReportsState } from "./api/reports";
import { JOBS_CHANNELS, JOBS_NOTHING_YET } from "./api/jobs";
import type { JobsState } from "./api/jobs";
import { SESSIONS_CHANNELS, SESSIONS_NOTHING_YET } from "./api/sessions";
import type { SessionsState } from "./api/sessions";

export type { HistoryStep, Summons } from "./api/core";

/**
 * The state with its identity current, which today means Fleet's version.
 *
 * **Here rather than at the five places a failure is built**, for the reason
 * `connectedTo` is here: a fact derived from the connection is derived once, by
 * the thing that owns the connection, so no surface can publish a state whose
 * identity disagrees with it. Four of the five failure builders are handed no
 * connection at all, and a refusal — the one Fleet itself answered — was the
 * payload most obviously wrong to omit Fleet's version from.
 *
 * The identity is rewritten only when the version moves, so a state whose
 * connection did not change keeps the same object and nothing redraws for it.
 *
 * `null` for the three connection states that never read a runtime file. Absent
 * rather than guessed: a version written in for a Fleet Bridge never identified
 * would be the one row of that payload nobody could check.
 */
export function identifying(state: BridgeState): BridgeState {
  const fleetProtocol =
    "fleet" in state.connection ? spoken(state.connection.fleet.protocolId) : null;
  if (fleetProtocol === state.bridge.fleetProtocol) return state;
  return { ...state, bridge: { ...state.bridge, fleetProtocol } };
}

/**
 * Everything one window's own pick decides, or reads against its own scope: the root, the
 * Manifest reading, Overview's health and drift, the workflows left out on this scope, and the
 * Manifest surface's own run sheet, its followed run and its drift.
 *
 * **Every window keeps its own** — `main/picked.ts`'s `PickedByWindow` holds one `Picked`
 * per window, and `main/index.ts` overlays this shape onto the shared state each window
 * receives, so a pick made in one window never moves what another window reads back.
 */
export type PickedView = {
  repository: string | null;
  manifestReading: ManifestReading | null;
  health: HealthRead;
  leftOut?: LeftOutWorkflow[];
  checkoutRunSheet: CheckoutRunSheetRead;
  checkoutRunFollowed: CheckoutRunFollowed;
  manifestDrift: ManifestDriftRead;
};

/**
 * Everything the renderer draws, published by main and never assembled twice.
 * **Composed from the slices in `api/`**; each owns the fields its surface reads.
 */
export type BridgeState = CoreState &
  StudiosState &
  ManifestState &
  SetupState &
  WorkflowsState &
  TriggersState &
  AddedStepsState &
  HelmState &
  SettingsState &
  OverviewState &
  CleanupState &
  ReportsState &
  JobsState &
  SessionsState;

/**
 * What Bridge holds before anything has answered.
 *
 * **One statement, not two.** Main and the renderer each used to declare their
 * own, and the two drifted the first time a field was added — a renderer
 * missing a key main publishes reads as a field that is always absent.
 * Spread from the slices' own, which a type check holds to `BridgeState`.
 */
export const NOTHING_YET: BridgeState = {
  ...CORE_NOTHING_YET,
  ...STUDIOS_NOTHING_YET,
  ...MANIFEST_NOTHING_YET,
  ...SETUP_NOTHING_YET,
  ...WORKFLOWS_NOTHING_YET,
  ...TRIGGERS_NOTHING_YET,
  ...ADDED_STEPS_NOTHING_YET,
  ...HELM_NOTHING_YET,
  ...SETTINGS_NOTHING_YET,
  ...OVERVIEW_NOTHING_YET,
  ...CLEANUP_NOTHING_YET,
  ...REPORTS_NOTHING_YET,
  ...JOBS_NOTHING_YET,
  ...SESSIONS_NOTHING_YET,
};

/** The channels the preload is allowed to name. There is no general `invoke`. */
export const CHANNELS = {
  ...CORE_CHANNELS,
  ...STUDIOS_CHANNELS,
  ...MANIFEST_CHANNELS,
  ...SETUP_CHANNELS,
  ...WORKFLOWS_CHANNELS,
  ...TRIGGERS_CHANNELS,
  ...ADDED_STEPS_CHANNELS,
  ...HELM_CHANNELS,
  ...SETTINGS_CHANNELS,
  ...OVERVIEW_CHANNELS,
  ...CLEANUP_CHANNELS,
  ...REPORTS_CHANNELS,
  ...JOBS_CHANNELS,
  ...SESSIONS_CHANNELS,
} as const;
