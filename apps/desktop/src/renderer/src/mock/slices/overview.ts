// Overview's registration: its member comes from `@armada/overview/fake`.

import { overviewApi } from "@armada/overview/fake";

import type { OverviewApi, OverviewState } from "../../../../shared/api/overview";
import { OVERVIEW_NOTHING_YET } from "../../../../shared/api/overview";
import type { Slice } from "../fake-context";

export const overview: Slice<OverviewApi, OverviewState> = {
  name: "overview",
  state: OVERVIEW_NOTHING_YET,
  api: () => overviewApi(),
};
