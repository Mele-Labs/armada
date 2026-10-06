// Studios' registration: its members and scenarios come from `@armada/studios/fake`.

import { studiosApi } from "@armada/studios/fake";

import type { StudiosApi, StudiosState } from "../../../../shared/api/studios";
import { STUDIOS_NOTHING_YET } from "../../../../shared/api/studios";
import type { Slice } from "../fake-context";
import { readingNothing, studying, zoneProposing, zoning } from "../studios-fake";

export const studios: Slice<StudiosApi, StudiosState> = {
  name: "studios",
  state: STUDIOS_NOTHING_YET,
  scenarios: () => [studying().scenario, zoning().scenario, zoneProposing().scenario, readingNothing().scenario],
  api: (scenario, fleet) => studiosApi(scenario.studios, fleet),
};
