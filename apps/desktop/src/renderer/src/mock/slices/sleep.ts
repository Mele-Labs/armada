// Sleep's members: the wire is answered by Fleet alone, so the mock has none for it. The walk's own
// night is `mock/sleep.ts`, provided beside the sources rather than through the bridge.

import type { SleepApi } from "../../../../shared/api/sleep";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const sleep: Slice<SleepApi, Record<string, never>> = {
  name: "sleep",
  state: {},
  api: () => ({
    getSleep: async () => ({ ok: false, outcome: unanswered("/sleep") }),
    setSleep: async () => ({ ok: false, outcome: unanswered("/sleep") }),
    overrideSleep: async () => ({ ok: false, outcome: unanswered("/sleep/override") }),
    onSleepChanged: () => () => {},
  }),
};
