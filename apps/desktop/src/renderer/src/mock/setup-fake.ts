// Setup's mock Fleet lives in `@armada/setup/fake`, generic over the app's whole state and API.
// `settingUp` fixes it to desktop's `BridgeState` and `BridgeApi`, under the name a scenario list
// and a whole-app test use.

import { settingUp as settingUpOver } from "@armada/setup/fake";
import type { SettingUp } from "@armada/setup/fake";

import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";
import type { Scenario } from "./moment";

export * from "@armada/setup/fake";

export const settingUp = (options: SettingUp = {}): Scenario => settingUpOver<BridgeState, BridgeApi>(NOTHING_YET, options);
