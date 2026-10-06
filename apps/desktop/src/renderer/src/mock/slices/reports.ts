// Reports' members: filing one and reading the list.

import type { Outcome } from "@armada/protocol";

import type { ReportsApi, ReportsState } from "../../../../shared/api/reports";
import { REPORTS_NOTHING_YET } from "../../../../shared/api/reports";
import type { Slice } from "../fake-context";
import { nothing } from "../fake-context";

const OK: Outcome = { ok: true };

export const reports: Slice<ReportsApi, ReportsState> = {
  name: "reports",
  state: REPORTS_NOTHING_YET,
  api: (_scenario, { publish, unread }) => ({
    fileReport: async () => OK,
    readReports: async (want) => publish({ reports: want ? unread("/reports") : nothing }),
  }),
};
