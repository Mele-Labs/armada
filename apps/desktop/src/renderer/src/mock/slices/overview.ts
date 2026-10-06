// Overview's member: the watch held for the life of the window.

import type { OverviewApi, OverviewState } from "../../../../shared/api/overview";
import { OVERVIEW_NOTHING_YET } from "../../../../shared/api/overview";
import type { Slice } from "../fake-context";

export const overview: Slice<OverviewApi, OverviewState> = {
  name: "overview",
  state: OVERVIEW_NOTHING_YET,
  // Held for the life of the window: a failure here would draw every surface's Fleet panel in trouble.
  api: () => ({ watchOverview: async () => undefined }),
};
