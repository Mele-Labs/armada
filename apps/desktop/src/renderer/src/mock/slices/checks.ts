// The Checks page's reads: a Session's check log, the merge line's, and a stop.

import type { ChecksApi, ChecksState } from "../../../../shared/api/checks";
import { CHECKS_NOTHING_YET } from "../../../../shared/api/checks";
import type { Slice } from "../fake-context";
import { nothing } from "../fake-context";

export const checks: Slice<ChecksApi, ChecksState> = {
  name: "checks",
  state: CHECKS_NOTHING_YET,
  api: (_scenario, { publish, refused }) => ({
    followCheckOutput: async () => publish({ followed: nothing }),
    followLandCheck: async () => publish({ landFollowed: nothing }),
    readSessionCheckOutput: async (run) => refused(`/sessions/checks/${run}/output`),
    stopSessionCheck: async () => ({ ok: true }),
  }),
};
