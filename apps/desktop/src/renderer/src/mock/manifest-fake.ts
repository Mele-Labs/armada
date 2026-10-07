// Manifest's mock Fleet lives in `@armada/manifest/fake`, generic over the app's whole state and API.
// `manifesting` fixes it to desktop's `BridgeState` and `BridgeApi`, under the name a scenario list
// and a whole-app test use.

import { manifesting as manifestingOver } from "@armada/manifest/fake";
import type { Manifesting } from "@armada/manifest/fake";

import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";
import type { Scenario } from "./moment";

export * from "@armada/manifest/fake";

export const manifesting = (options: Manifesting = {}): Scenario => manifestingOver<BridgeState, BridgeApi>(NOTHING_YET, options);
