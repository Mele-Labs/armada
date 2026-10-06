// Setup's registration: its members and its moment come from `@armada/setup/fake`.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { setupApi } from "@armada/setup/fake";

import type { SetupApi, SetupState } from "../../../../shared/api/setup";
import { SETUP_NOTHING_YET } from "../../../../shared/api/setup";
import type { Slice } from "../fake-context";
import { SCRATCH, SHEET_READ, settingUp } from "../setup-fake";

export const setup: Slice<SetupApi, SetupState> = {
  name: "setup",
  state: SETUP_NOTHING_YET,
  scenarios: () => [settingUp({ repositories: [repository(), SCRATCH], sheet: SHEET_READ })],
  api: (_scenario, fleet) => setupApi(fleet),
};
