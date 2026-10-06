// Studios' mock fleet lives in `@armada/studios/fake`, generic over the app's whole state and API.
// These four fix it to desktop's `BridgeState` and `BridgeApi`, under the names a scenario list and
// a whole-app test use.

import {
  readingNothing as readingNothingOver,
  studying as studyingOver,
  zoneProposing as zoneProposingOver,
  zoning as zoningOver,
} from "@armada/studios/fake";
import type { StudioFleet } from "@armada/studios/fake";
import type { Studio } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";

export * from "@armada/studios/fake";

type DesktopStudioFleet = StudioFleet<BridgeState, BridgeApi>;

export const studying = (seeded?: readonly Studio[]): DesktopStudioFleet =>
  studyingOver<BridgeState, BridgeApi>(NOTHING_YET, seeded);
export const zoning = (): DesktopStudioFleet => zoningOver<BridgeState, BridgeApi>(NOTHING_YET);
export const zoneProposing = (): DesktopStudioFleet => zoneProposingOver<BridgeState, BridgeApi>(NOTHING_YET);
export const readingNothing = (): DesktopStudioFleet => readingNothingOver<BridgeState, BridgeApi>(NOTHING_YET);
