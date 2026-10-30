// Teaching: Fleet reads and keeps it, and lists the words it opens. A scenario that teaches (`teach-anything`, `teach-mods`, `teach-rule`) binds its
// Fleet to `../teach`, which answers from a few canned inputs; any other has none, and each call is a route the mock has none for.

import type { TeachingApi } from "../../../../shared/api/teaching";
import type { Slice } from "../fake-context";
import { listWordRules, readTeaching, saveTeaching } from "../teach";

export const teaching: Slice<TeachingApi, Record<string, never>> = {
  name: "teaching",
  state: {},
  api: () => ({
    readTeaching: async (read) => readTeaching(read),
    saveTeaching: async (save) => saveTeaching(save),
    listWordRules: async (repository) => listWordRules(repository),
  }),
};
