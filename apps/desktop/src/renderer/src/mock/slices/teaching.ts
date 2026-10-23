// Teaching: Fleet reads and keeps it, so the mock has nothing for it. A scenario that wants a proposal replaces these.

import type { TeachingApi } from "../../../../shared/api/teaching";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const teaching: Slice<TeachingApi, Record<string, never>> = {
  name: "teaching",
  state: {},
  api: () => ({
    readTeaching: async () => ({ ok: false, outcome: unanswered("/teachings/read") }),
    saveTeaching: async () => ({ ok: false, outcome: unanswered("/teachings") }),
    listWordRules: async () => ({ ok: false, outcome: unanswered("/teachings/words") }),
  }),
};
